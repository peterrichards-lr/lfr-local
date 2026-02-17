use std::fs;
use std::path::Path;
use regex::Regex;
use serde::de::DeserializeOwned;

/// Reads a specific key from a Liferay .properties file
pub fn get_property(path: &Path, key: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let re = Regex::new(&format!(r"^{}\s*=\s*(.*)", regex::escape(key))).ok()?;
    
    content.lines()
        .find_map(|line| re.captures(line))
        .map(|cap| cap[1].trim().to_string())
}

/// Generic JSON file reader
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Read error: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Parse error: {}", e))
}