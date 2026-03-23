use anyhow::{Context, Result};
use reqwest::blocking::Client;
use scraper::{Html, Selector};

pub const DEFAULT_DXP_BASE_URL: &str = "https://releases-cdn.liferay.com/dxp/";
pub const DEFAULT_PORTAL_BASE_URL: &str = "https://releases-cdn.liferay.com/portal/";

pub struct BundleResolver;

impl BundleResolver {
    pub fn resolve(product: &str, base_url_override: Option<String>) -> Result<String> {
        let (prefix, default_base) = if product.starts_with("portal-") {
            (
                product.strip_prefix("portal-").unwrap(),
                DEFAULT_PORTAL_BASE_URL,
            )
        } else if product.starts_with("dxp-") {
            (product.strip_prefix("dxp-").unwrap(), DEFAULT_DXP_BASE_URL)
        } else {
            anyhow::bail!(
                "Unknown product: {}. Try 'portal-7.4.3' or 'dxp-2024.q1'.",
                product
            )
        };

        let base_url = base_url_override.unwrap_or_else(|| default_base.to_string());
        let mut resolved_version = Self::find_latest_in_cdn(&base_url, prefix)?;

        // Handle LTS suffix for Q1 releases
        if product.starts_with("dxp-")
            && resolved_version.contains(".q1.")
            && !resolved_version.ends_with("-lts")
        {
            resolved_version = format!("{}-lts", resolved_version);
        }

        let version_url = format!("{}/{}", base_url.trim_end_matches('/'), resolved_version);
        Self::find_bundle_in_version_dir(&version_url, product.starts_with("dxp-"))
    }

    /// Finds the actual ZIP file link inside a version directory (e.g. /dxp/2025.q4.12/)
    fn find_bundle_in_version_dir(version_url: &str, is_dxp: bool) -> Result<String> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let response = client
            .get(version_url)
            .send()
            .context("Failed to reach version directory")?;
        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to list version directory (HTTP {}): {}",
                response.status(),
                version_url
            );
        }

        let body = response.text()?;
        let document = Html::parse_document(&body);
        let selector = Selector::parse("a").unwrap();

        let target_prefix = if is_dxp {
            "liferay-dxp-tomcat-"
        } else {
            "liferay-portal-tomcat-"
        };

        let matches: Vec<String> = document
            .select(&selector)
            .filter_map(|element| {
                let href = element.value().attr("href")?;
                let text = href.trim_matches('/');

                // Get just the filename if it's a path
                let filename = text.split('/').next_back()?;

                if filename.starts_with(target_prefix) && filename.ends_with(".zip") {
                    Some(filename.to_string())
                } else {
                    None
                }
            })
            .collect();

        if matches.is_empty() {
            anyhow::bail!("No tomcat zip found in {}", version_url);
        }

        // Return the full URL to the zip
        Ok(format!(
            "{}/{}",
            version_url.trim_end_matches('/'),
            matches[0]
        ))
    }

    /// Attempts to find the latest version matching a prefix by scraping the CDN index
    fn find_latest_in_cdn(base_url: &str, prefix: &str) -> Result<String> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let response = client
            .get(base_url)
            .send()
            .context("Failed to reach CDN index")?;
        if !response.status().is_success() {
            if prefix.split('.').count() >= 3 {
                return Ok(prefix.to_string());
            }
            anyhow::bail!(
                "Failed to list CDN versions (HTTP {}) and prefix is incomplete: {}",
                response.status(),
                prefix
            );
        }

        let body = response.text()?;
        let document = Html::parse_document(&body);
        let selector = Selector::parse("a").unwrap();

        let mut matches: Vec<String> = document
            .select(&selector)
            .filter_map(|element| {
                let href = element.value().attr("href")?;
                let mut text = href.trim_matches('/');

                if text.starts_with("dxp") {
                    text = text.strip_prefix("dxp")?.trim_matches('/');
                } else if text.starts_with("portal") {
                    text = text.strip_prefix("portal")?.trim_matches('/');
                }

                if text.starts_with(prefix) && !text.is_empty() {
                    Some(text.to_string())
                } else {
                    None
                }
            })
            .collect();

        if matches.is_empty() {
            if prefix.split('.').count() >= 3 {
                return Ok(prefix.to_string());
            }
            anyhow::bail!(
                "No versions found matching prefix '{}' at {}",
                prefix,
                base_url
            );
        }

        matches.sort_by(|a, b| Self::compare_versions(a, b));

        Ok(matches.last().unwrap().to_string())
    }

    fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
        let a_parts: Vec<&str> = a.split(['.', '-']).collect();
        let b_parts: Vec<&str> = b.split(['.', '-']).collect();

        for (p1, p2) in a_parts.iter().zip(b_parts.iter()) {
            if p1 != p2 {
                let n1 = p1.parse::<u32>();
                let n2 = p2.parse::<u32>();
                match (n1, n2) {
                    (Ok(v1), Ok(v2)) => return v1.cmp(&v2),
                    _ => return p1.cmp(p2),
                }
            }
        }
        a_parts.len().cmp(&b_parts.len())
    }
}
