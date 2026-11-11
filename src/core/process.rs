//! Identifies the Java process that belongs to a given lfr-local instance.
//!
//! Each instance's Tomcat listens on `8080 + id * 100`. A Tomcat JVM is started
//! by `catalina.sh` / `catalina.bat` with `-Dcatalina.base=<tomcat dir>` (and
//! `-Dcatalina.home=<tomcat dir>`) on its command line. We read that directory
//! from each Java process's command line, look up the HTTP port(s) configured in
//! its `conf/server.xml`, and keep the processes whose port matches the
//! requested instance. This never falls back to "any Java process".

use std::path::{Path, PathBuf};

/// A minimal, OS-independent view of a running process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcInfo {
    pub pid: u32,
    pub name: String,
    pub cmd: Vec<String>,
}

/// A process matched to an instance, along with the Tomcat directory that
/// identified it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceProcess {
    pub pid: u32,
    pub catalina_base: PathBuf,
}

/// Returns true if the process name looks like a JVM launcher
/// (`java`, `java.exe`, `javaw.exe`, ...).
fn is_java(name: &str) -> bool {
    name.to_lowercase().contains("java")
}

/// Extracts the Tomcat directory from a JVM command line. Prefers
/// `-Dcatalina.base`, falling back to `-Dcatalina.home`. Surrounding quotes
/// (as can appear on Windows) are stripped.
pub fn catalina_base_from_cmd(cmd: &[String]) -> Option<PathBuf> {
    let find = |key: &str| {
        let prefix = format!("-D{}=", key);
        cmd.iter().find_map(|arg| {
            let arg = arg.trim().trim_matches('"');
            let value = arg.strip_prefix(&prefix)?;
            let value = value.trim().trim_matches('"').trim();
            if value.is_empty() {
                None
            } else {
                Some(PathBuf::from(value))
            }
        })
    };
    find("catalina.base").or_else(|| find("catalina.home"))
}

/// Returns every Java process whose Tomcat directory is configured to serve
/// HTTP on `port`. `ports_for_base` returns the HTTP port(s) configured for a
/// Tomcat directory (normally read from its `conf/server.xml`).
pub fn find_instance_processes<I, F>(procs: I, port: u16, ports_for_base: F) -> Vec<InstanceProcess>
where
    I: IntoIterator<Item = ProcInfo>,
    F: Fn(&Path) -> Vec<u16>,
{
    let mut matches: Vec<InstanceProcess> = procs
        .into_iter()
        .filter(|p| is_java(&p.name))
        .filter_map(|p| {
            let base = catalina_base_from_cmd(&p.cmd)?;
            if ports_for_base(&base).contains(&port) {
                Some(InstanceProcess {
                    pid: p.pid,
                    catalina_base: base,
                })
            } else {
                None
            }
        })
        .collect();
    matches.sort_by_key(|m| m.pid);
    matches.dedup_by_key(|m| m.pid);
    matches
}

/// Snapshot of the running processes, as `ProcInfo`.
pub fn running_processes(sys: &sysinfo::System) -> Vec<ProcInfo> {
    sys.processes()
        .values()
        .map(|p| ProcInfo {
            pid: p.pid().as_u32(),
            name: p.name().to_string(),
            cmd: p.cmd().to_vec(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(pid: u32, name: &str, cmd: &[&str]) -> ProcInfo {
        ProcInfo {
            pid,
            name: name.to_string(),
            cmd: cmd.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn tomcat_cmd(base: &str) -> Vec<String> {
        vec![
            "/usr/bin/java".into(),
            "-Xmx2560m".into(),
            format!("-Dcatalina.base={}", base),
            format!("-Dcatalina.home={}", base),
            "-Djava.io.tmpdir=/x/temp".into(),
            "org.apache.catalina.startup.Bootstrap".into(),
            "start".into(),
        ]
    }

    /// Instance 0 -> 8080, instance 2 -> 8280.
    fn ports(base: &Path) -> Vec<u16> {
        let s = base.to_string_lossy();
        if s.contains("inst0") {
            vec![8080, 8443]
        } else if s.contains("inst2") {
            vec![8280]
        } else {
            vec![]
        }
    }

    fn sample() -> Vec<ProcInfo> {
        let mut v = vec![
            // Unrelated Java process (IDE) listed first, as in the old bug.
            proc(
                100,
                "java",
                &["/opt/idea/jbr/bin/java", "-Xmx4g", "com.intellij.idea.Main"],
            ),
            proc(50, "zsh", &["-zsh", "-Dcatalina.base=/w/inst2/tomcat"]),
        ];
        let mut p0 = proc(200, "java", &[]);
        p0.cmd = tomcat_cmd("/w/inst0/bundles/tomcat-9.0.90");
        let mut p2 = proc(300, "java", &[]);
        p2.cmd = tomcat_cmd("/w/inst2/bundles/tomcat-9.0.90");
        v.push(p0);
        v.push(p2);
        v
    }

    #[test]
    fn matches_the_requested_instance_only() {
        let m = find_instance_processes(sample(), 8280, ports);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].pid, 300);
        assert_eq!(
            m[0].catalina_base,
            PathBuf::from("/w/inst2/bundles/tomcat-9.0.90")
        );

        let m = find_instance_processes(sample(), 8080, ports);
        assert_eq!(m.iter().map(|m| m.pid).collect::<Vec<_>>(), vec![200]);
    }

    #[test]
    fn ignores_unrelated_java_and_non_java_processes() {
        let procs = vec![
            proc(100, "java", &["java", "-jar", "app.jar"]),
            proc(50, "bash", &["bash", "-Dcatalina.base=/w/inst2/tomcat"]),
        ];
        assert!(find_instance_processes(procs, 8280, ports).is_empty());
    }

    #[test]
    fn no_match_when_no_instance_on_port() {
        assert!(find_instance_processes(sample(), 8180, ports).is_empty());
    }

    #[test]
    fn reports_multiple_matches() {
        let mut procs = sample();
        let mut dup = proc(400, "java", &[]);
        dup.cmd = tomcat_cmd("/w/inst2/other-tomcat");
        procs.push(dup);
        let m = find_instance_processes(procs, 8280, ports);
        assert_eq!(m.iter().map(|m| m.pid).collect::<Vec<_>>(), vec![300, 400]);
    }

    #[test]
    fn handles_windows_style_command_lines() {
        let procs = vec![
            proc(
                10,
                "javaw.exe",
                &[r"C:\Program Files\Java\bin\javaw.exe", "-jar", "ide.jar"],
            ),
            proc(
                20,
                "java.exe",
                &[
                    r"C:\Program Files\Java\bin\java.exe",
                    r#"-Dcatalina.base="C:\liferay\inst2\bundles\tomcat-9.0.90""#,
                    "org.apache.catalina.startup.Bootstrap",
                ],
            ),
        ];
        let m = find_instance_processes(procs, 8280, ports);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].pid, 20);
        assert_eq!(
            m[0].catalina_base,
            PathBuf::from(r"C:\liferay\inst2\bundles\tomcat-9.0.90")
        );
    }

    #[test]
    fn catalina_base_preferred_over_home() {
        let cmd: Vec<String> = vec![
            "java".into(),
            "-Dcatalina.home=/shared/tomcat".into(),
            "-Dcatalina.base=/w/inst2/tomcat".into(),
        ];
        assert_eq!(
            catalina_base_from_cmd(&cmd),
            Some(PathBuf::from("/w/inst2/tomcat"))
        );
        let cmd: Vec<String> = vec!["java".into(), "-Dcatalina.home=/w/inst0/tomcat".into()];
        assert_eq!(
            catalina_base_from_cmd(&cmd),
            Some(PathBuf::from("/w/inst0/tomcat"))
        );
        let cmd: Vec<String> = vec!["java".into(), "-Dcatalina.base=".into()];
        assert_eq!(catalina_base_from_cmd(&cmd), None);
    }
}
