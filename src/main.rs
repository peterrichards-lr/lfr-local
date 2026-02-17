mod cli;
mod core;
mod utils;

use crate::cli::{App, AppCommands};
use crate::core::{LiferayWorkspace, Workspace};
use clap::Parser;
use dialoguer::Confirm;
use edit_xml::{Document, Element};
use std::fs;
use std::net::{TcpStream, ToSocketAddrs};
use std::process::Command;
use std::time::Duration;
use sysinfo::System;

/// Helper to find all Connector elements at any depth in the XML tree
fn collect_connectors(doc: &Document) -> Vec<Element> {
    let mut connectors = Vec::new();
    let mut stack: Vec<Element> = doc.root_element().into_iter().collect();

    while let Some(el) = stack.pop() {
        if el.name(doc) == "Connector" {
            connectors.push(el);
        }
        for child in el.children(doc) {
            if let Some(child_el) = child.as_element() {
                stack.push(child_el);
            }
        }
    }
    connectors
}

fn main() -> Result<(), String> {
    let args = App::parse();
    let ws = LiferayWorkspace {
        current_dir: std::env::current_dir().unwrap_or_default(),
    };

    match args.command {
        AppCommands::Configure {
            instance_id,
            workspace_path,
            db_name,
            clear_data,
        } => {
            let root_path = workspace_path.unwrap_or(ws.find_root()?);
            let tomcat = ws.find_tomcat(&root_path)?;
            let bundles = root_path.join("bundles");

            let offset = instance_id * 100;
            let p_stop = 8005 + offset;
            let p_http = 8080 + offset;
            let p_ajp = 8009 + offset;
            let p_ssl = 8443 + offset;
            let cookie = format!("LFR_SESSION_{}", instance_id);
            let db = db_name.unwrap_or(format!("lportal_{}", instance_id));

            println!("--- Reconfiguring: Instance {} ---", instance_id);

            let server_xml_path = tomcat.join("conf/server.xml");
            let server_xml_raw = fs::read_to_string(&server_xml_path).map_err(|e| e.to_string())?;
            let mut server_doc =
                Document::parse_str(&server_xml_raw).map_err(|e| format!("XML Error: {}", e))?;

            if let Some(root) = server_doc.root_element() {
                root.set_attribute(&mut server_doc, "port", p_stop.to_string());
            }

            let connectors = collect_connectors(&server_doc);
            for connector in connectors {
                let protocol: String = connector
                    .attribute(&server_doc, "protocol")
                    .unwrap_or("http/1.1")
                    .to_lowercase();
                if protocol.contains("http") || protocol == "http/1.1" {
                    connector.set_attribute(&mut server_doc, "port", p_http.to_string());
                    connector.set_attribute(&mut server_doc, "redirectPort", p_ssl.to_string());
                } else if protocol.contains("ajp") {
                    connector.set_attribute(&mut server_doc, "port", p_ajp.to_string());
                    connector.set_attribute(&mut server_doc, "redirectPort", p_ssl.to_string());
                }
            }

            let mut server_output = Vec::new();
            server_doc
                .write(&mut server_output)
                .map_err(|e| e.to_string())?;
            fs::write(&server_xml_path, server_output).map_err(|e| e.to_string())?;

            let context_xml_path = tomcat.join("conf/context.xml");
            let context_xml_raw =
                fs::read_to_string(&context_xml_path).map_err(|e| e.to_string())?;
            let mut context_doc =
                Document::parse_str(&context_xml_raw).map_err(|e| format!("XML Error: {}", e))?;
            if let Some(root) = context_doc.root_element() {
                root.set_attribute(&mut context_doc, "sessionCookieName", cookie.clone());
            }
            let mut context_output = Vec::new();
            context_doc
                .write(&mut context_output)
                .map_err(|e| e.to_string())?;
            fs::write(context_xml_path, context_output).map_err(|e| e.to_string())?;

            let prop_path = bundles.join("portal-ext.properties");
            let props_content = fs::read_to_string(&prop_path).unwrap_or_default();
            let mut new_props: Vec<String> = props_content
                .lines()
                .filter(|l| {
                    !l.starts_with("session.cookie.name") && !l.starts_with("jdbc.default.url")
                })
                .map(|s| s.to_string())
                .collect();
            new_props.push(format!("session.cookie.name={}", cookie));
            new_props.push(format!("jdbc.default.url=jdbc:hsqldb:${{liferay.home}}/data/hypersonic/{};hsqldb.write_delay=false", db));
            fs::write(prop_path, new_props.join("\n")).map_err(|e| e.to_string())?;

            if clear_data {
                let _ = fs::remove_dir_all(bundles.join("data"));
            }

            println!("Success! Instance {} configured.", instance_id);
            Ok(())
        }

        AppCommands::Summary => {
            let root = ws.find_root()?;
            let tomcat = ws.find_tomcat(&root)?;
            let bundles = root.join("bundles");

            println!("\n{:<25} {:<45}", "PROPERTY", "VALUE");
            println!("{}", "=".repeat(70));

            println!("{:<25} {:<45}", "Liferay Home", bundles.to_string_lossy());
            if let Ok(output) = Command::new("java").arg("-version").output() {
                let v = String::from_utf8_lossy(&output.stderr)
                    .lines()
                    .next()
                    .unwrap_or("-")
                    .to_string();
                println!(
                    "{:<25} {:<45}",
                    "Java Version",
                    v.replace("java version ", "").replace("\"", "")
                );
            }

            if let Ok(content) = fs::read_to_string(root.join("gradle.properties")) {
                for line in content.lines() {
                    if line.starts_with("liferay.workspace.product") {
                        println!(
                            "{:<25} {:<45}",
                            "Liferay Product",
                            line.split('=').nth(1).unwrap_or("N/A").trim()
                        );
                    }
                }
            }

            let mut current_http: u16 = 8080;
            if let Ok(content) = fs::read_to_string(tomcat.join("conf/server.xml")) {
                if let Ok(doc) = Document::parse_str(&content) {
                    if let Some(r) = doc.root_element() {
                        println!(
                            "{:<25} {:<45}",
                            "Shutdown Port",
                            r.attribute(&doc, "port").unwrap_or("8005")
                        );

                        for node in collect_connectors(&doc) {
                            let port_str: &str = node.attribute(&doc, "port").unwrap_or("-");
                            let proto: String = node
                                .attribute(&doc, "protocol")
                                .unwrap_or("http")
                                .to_lowercase();

                            if proto.contains("ajp") {
                                println!("{:<25} {:<45}", "AJP Port", port_str);
                            } else {
                                println!("{:<25} {:<45}", "HTTP Port", port_str);
                                if let Ok(p) = port_str.parse::<u16>() {
                                    current_http = p;
                                }
                                println!(
                                    "{:<25} {:<45}",
                                    "HTTPS Redirect",
                                    node.attribute(&doc, "redirectPort").unwrap_or("8443")
                                );
                            }
                        }
                    }
                }
            }

            let mut es_port = "9200 (Default)".to_string();
            if let Ok(c) =
                fs::read_to_string(bundles.join("elasticsearch/config/elasticsearch.yml"))
            {
                for line in c.lines() {
                    if line.contains("http.port:") {
                        es_port = line.split(':').nth(1).unwrap_or("9200").trim().to_string();
                    }
                }
            }
            println!("{:<25} {:<45}", "Elasticsearch Port", es_port);

            let offset = (current_http / 100).saturating_sub(80);
            println!("{:<25} {:<45}", "Gogo Shell Port", 11311 + (offset * 100));

            let mut session_cookie = "JSESSIONID (Default)".to_string();
            let mut db_url = "Default (HSQL in-memory)".to_string();
            if let Ok(content) = fs::read_to_string(bundles.join("portal-ext.properties")) {
                for line in content.lines() {
                    if line.starts_with("session.cookie.name") {
                        session_cookie = line.split('=').nth(1).unwrap_or("-").trim().to_string();
                    }
                    if line.starts_with("jdbc.default.url") {
                        db_url = line.split('=').nth(1).unwrap_or("-").trim().to_string();
                    }
                }
            }
            println!("{:<25} {:<45}", "Session Cookie", session_cookie);
            println!("{:<25} {:<45}", "Database URL", db_url);

            println!("{}", "=".repeat(70));
            Ok(())
        }

        AppCommands::Status { instance_id } => {
            let mut sys = System::new_all();
            sys.refresh_all();
            println!(
                "{:<12} {:<10} {:<10} {:<10}",
                "INSTANCE ID", "PORT", "STATUS", "PID"
            );
            println!("{}", "-".repeat(45));
            let ids = match instance_id {
                Some(id) => id..=id,
                None => 0..=5,
            };
            for id in ids {
                let port = 8080 + (id * 100);
                let addr = format!("127.0.0.1:{}", port);
                let is_open = if let Ok(mut addrs) = addr.to_socket_addrs() {
                    if let Some(s) = addrs.next() {
                        TcpStream::connect_timeout(&s, Duration::from_millis(50)).is_ok()
                    } else {
                        false
                    }
                } else {
                    false
                };
                if is_open {
                    let pid = sys
                        .processes()
                        .values()
                        .find(|p| p.name().to_lowercase().contains("java"))
                        .map(|p| p.pid().to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    println!("{:<12} {:<10} {:<10} {:<10}", id, port, "RUNNING", pid);
                } else {
                    println!("{:<12} {:<10} {:<10} {:<10}", id, port, "STOPPED", "-");
                }
            }
            Ok(())
        }

        AppCommands::Kill { instance_id } => {
            let mut sys = System::new_all();
            sys.refresh_all();
            let port = 8080 + (instance_id * 100);
            let addr = format!("127.0.0.1:{}", port);
            if let Ok(mut addrs) = addr.to_socket_addrs() {
                if let Some(s) = addrs.next() {
                    if TcpStream::connect_timeout(&s, Duration::from_millis(50)).is_err() {
                        return Err(format!("Instance {} not running.", instance_id));
                    }
                }
            }
            let process = sys
                .processes()
                .values()
                .find(|p| p.name().to_lowercase().contains("java"))
                .ok_or_else(|| "No process found.".to_string())?;
            process.kill();
            println!("Killed PID {}.", process.pid());
            Ok(())
        }

        AppCommands::Reset {
            workspace_path,
            all,
            props,
            ports,
        } => {
            let root = workspace_path.unwrap_or(ws.find_root()?);
            let tomcat = ws.find_tomcat(&root)?;
            let bundles = root.join("bundles");

            if props {
                println!("Resetting session cookies and database URLs in portal-ext.properties...");
                let prop_path = bundles.join("portal-ext.properties");
                if let Ok(content) = fs::read_to_string(&prop_path) {
                    let filtered: Vec<String> = content
                        .lines()
                        .filter(|l| {
                            !l.starts_with("session.cookie.name")
                                && !l.starts_with("jdbc.default.url")
                        })
                        .map(|s| s.to_string())
                        .collect();
                    let _ = fs::write(prop_path, filtered.join("\n"));
                }
            }

            if ports {
                println!("Resetting server.xml ports to 8080/8005 defaults...");
                let server_xml_path = tomcat.join("conf/server.xml");
                if let Ok(raw) = fs::read_to_string(&server_xml_path) {
                    if let Ok(mut doc) = Document::parse_str(&raw) {
                        if let Some(root) = doc.root_element() {
                            root.set_attribute(&mut doc, "port", "8005");
                            for connector in collect_connectors(&doc) {
                                let protocol: String = connector
                                    .attribute(&doc, "protocol")
                                    .unwrap_or("http/1.1")
                                    .to_lowercase();
                                if protocol.contains("http") {
                                    connector.set_attribute(&mut doc, "port", "8080");
                                    connector.set_attribute(&mut doc, "redirectPort", "8443");
                                } else if protocol.contains("ajp") {
                                    connector.set_attribute(&mut doc, "port", "8009");
                                    connector.set_attribute(&mut doc, "redirectPort", "8443");
                                }
                            }
                            let mut out = Vec::new();
                            let _ = doc.write(&mut out);
                            let _ = fs::write(server_xml_path, out);
                        }
                    }
                }
            }

            if all
                && !Confirm::new()
                    .with_prompt("Wipe ALL data?")
                    .interact()
                    .unwrap_or(false)
            {
                return Ok(());
            }
            let _ = fs::remove_dir_all(bundles.join("osgi/state"));
            let t_temp = tomcat.join("temp");
            let t_work = tomcat.join("work");
            let _ = fs::remove_dir_all(&t_temp);
            let _ = fs::remove_dir_all(&t_work);
            let _ = fs::create_dir_all(t_temp);
            let _ = fs::create_dir_all(t_work);
            if all {
                let _ = fs::remove_dir_all(bundles.join("data"));
            }
            println!("Reset complete.");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edit_xml::Document;

    #[test]
    fn test_xml_logic() {
        let xml_data = r#"<?xml version="1.0" encoding="UTF-8"?><Server port="8005"><Service><Connector port="8080" protocol="HTTP/1.1" /></Service></Server>"#;
        let mut doc = Document::parse_str(xml_data).unwrap();
        for c in collect_connectors(&doc) {
            c.set_attribute(&mut doc, "port", "8180");
        }
        let mut output = Vec::new();
        doc.write(&mut output).unwrap();
        assert!(String::from_utf8(output).unwrap().contains("port=\"8180\""));
    }
}
