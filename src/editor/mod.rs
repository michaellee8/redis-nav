use anyhow::{anyhow, Result};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use crate::format::{detect_format, DetectedFormat};

pub struct ExternalEditor {
    temp_dir: PathBuf,
}

impl ExternalEditor {
    pub fn new() -> Result<Self> {
        let temp_dir = std::env::temp_dir().join("redis-nav");
        fs::create_dir_all(&temp_dir)?;
        Ok(Self { temp_dir })
    }

    pub fn edit(&self, key: &str, value: &[u8]) -> Result<Option<Vec<u8>>> {
        let ext = match detect_format(value) {
            DetectedFormat::Json => ".json",
            DetectedFormat::Xml | DetectedFormat::Html => ".xml",
            _ => ".txt",
        };

        let safe_key = sanitize_filename(key);
        let temp_path = self.temp_dir.join(format!("{}{}", safe_key, ext));

        // Write current value
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(value)?;
        file.flush()?;
        drop(file);

        let before_hash = hash_bytes(value);

        // Get editor
        let editor = std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| {
                if cfg!(windows) {
                    "notepad".to_string()
                } else {
                    "vi".to_string()
                }
            });

        // Spawn editor
        let status = Command::new(&editor)
            .arg(&temp_path)
            .status()
            .map_err(|e| anyhow!("Failed to launch editor '{}': {}", editor, e))?;

        if !status.success() {
            fs::remove_file(&temp_path).ok();
            return Err(anyhow!("Editor exited with non-zero status"));
        }

        // Read modified content
        let new_value = fs::read(&temp_path)?;
        fs::remove_file(&temp_path).ok();

        let after_hash = hash_bytes(&new_value);

        if before_hash == after_hash {
            Ok(None) // No changes
        } else {
            Ok(Some(new_value))
        }
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(50)
        .collect()
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}
