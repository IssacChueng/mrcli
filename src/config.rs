use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};

use crate::model::AppConfig;

pub fn load_config() -> Result<AppConfig> {
    let path = config_path()?;
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    let config: AppConfig = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse config file: {}", path.display()))?;
    validate_config(&config)?;
    Ok(config)
}

pub fn config_path() -> Result<PathBuf> {
    let mut path = dirs::config_dir().ok_or_else(|| anyhow!("config directory not found"))?;
    path.push("mrcli");
    path.push("config.json");
    Ok(path)
}

pub fn default_connection_index(config: &AppConfig) -> usize {
    config
        .default_connection
        .as_ref()
        .and_then(|name| {
            config
                .connections
                .iter()
                .position(|conn| &conn.name == name)
        })
        .unwrap_or(0)
}

fn validate_config(config: &AppConfig) -> Result<()> {
    if config.connections.is_empty() {
        bail!("config must contain at least one connection")
    }

    let mut names = HashSet::new();
    for connection in &config.connections {
        if connection.name.trim().is_empty() {
            bail!("connection name cannot be empty")
        }
        if !names.insert(connection.name.clone()) {
            bail!("duplicate connection name: {}", connection.name)
        }
        if connection.kind != "mysql" {
            bail!("unsupported connection kind: {}", connection.kind)
        }
        if connection.host.trim().is_empty() {
            bail!("connection host cannot be empty")
        }
        if connection.username.trim().is_empty() {
            bail!("connection username cannot be empty")
        }
        if connection.database.trim().is_empty() {
            bail!("connection database cannot be empty")
        }
        if connection.port == 0 {
            bail!("connection port must be greater than zero")
        }
    }

    Ok(())
}
