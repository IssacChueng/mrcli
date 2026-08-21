use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use chrono::Local;

use crate::model::DraftEntry;

pub fn list_drafts() -> Result<Vec<DraftEntry>> {
    let drafts_dir = ensure_drafts_dir()?;
    let mut drafts = Vec::new();

    for entry in fs::read_dir(&drafts_dir)
        .with_context(|| format!("failed to read drafts directory: {}", drafts_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("sql") {
            continue;
        }

        let metadata = entry.metadata()?;
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("invalid draft file name"))?
            .to_string();
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read draft file: {}", path.display()))?;

        drafts.push(DraftEntry {
            file_name,
            path,
            content,
            modified_at: metadata.modified().ok(),
        });
    }

    drafts.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    Ok(drafts)
}

pub fn create_new_draft() -> Result<DraftEntry> {
    let drafts_dir = ensure_drafts_dir()?;
    let file_name = next_draft_file_name(&drafts_dir)?;
    let path = drafts_dir.join(&file_name);
    fs::write(&path, "")
        .with_context(|| format!("failed to create draft file: {}", path.display()))?;

    Ok(DraftEntry {
        file_name,
        path,
        content: String::new(),
        modified_at: None,
    })
}

pub fn save_draft(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content)
        .with_context(|| format!("failed to save draft file: {}", path.display()))
}

pub fn delete_draft(path: &Path) -> Result<()> {
    fs::remove_file(path)
        .with_context(|| format!("failed to delete draft file: {}", path.display()))
}

pub fn drafts_dir() -> Result<PathBuf> {
    let mut path = dirs::config_dir().ok_or_else(|| anyhow!("config directory not found"))?;
    path.push("mrcli");
    path.push("drafts");
    Ok(path)
}

fn ensure_drafts_dir() -> Result<PathBuf> {
    let drafts_dir = drafts_dir()?;
    fs::create_dir_all(&drafts_dir).with_context(|| {
        format!(
            "failed to create drafts directory: {}",
            drafts_dir.display()
        )
    })?;
    Ok(drafts_dir)
}

fn next_draft_file_name(drafts_dir: &Path) -> Result<String> {
    let prefix = Local::now().format("%Y-%m-%d").to_string();
    let mut max_index = 0usize;

    for entry in fs::read_dir(drafts_dir)? {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };

        if !name.starts_with(&prefix) || !name.ends_with(".sql") {
            continue;
        }

        let Some(index_part) = name
            .strip_prefix(&(prefix.clone() + "-"))
            .and_then(|value| value.strip_suffix(".sql"))
        else {
            continue;
        };

        if let Ok(index) = index_part.parse::<usize>() {
            max_index = max_index.max(index);
        }
    }

    Ok(format!("{}-{}.sql", prefix, max_index + 1))
}
