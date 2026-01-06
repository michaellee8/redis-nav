use super::ProtectedNamespace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default)]
    pub delimiters: Vec<String>,
    #[serde(default)]
    pub theme: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Profile {
    pub url: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub password: Option<String>,
    pub password_env: Option<String>,
    pub db: Option<u8>,
    #[serde(default)]
    pub delimiters: Vec<String>,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default)]
    pub protected_namespaces: Vec<ProtectedNamespace>,
}

impl ConfigFile {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: ConfigFile = toml::from_str(&content)?;
        Ok(config)
    }
}
