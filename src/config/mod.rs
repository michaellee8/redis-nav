pub mod cli;
pub mod file;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub connection: ConnectionConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub url: String,
    pub db: u8,
    pub readonly: bool,
}

#[derive(Debug, Clone)]
pub struct UiConfig {
    pub delimiters: Vec<char>,
    pub protected_namespaces: Vec<ProtectedNamespace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedNamespace {
    pub prefix: String,
    pub level: ProtectionLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProtectionLevel {
    Warn,
    Confirm,
    Block,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            connection: ConnectionConfig {
                url: "redis://127.0.0.1:6379".to_string(),
                db: 0,
                readonly: false,
            },
            ui: UiConfig {
                delimiters: vec![':', '/'],
                protected_namespaces: vec![],
            },
        }
    }
}
