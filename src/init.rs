use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct InitProjectResponse {
    pub status: String,
    pub project_root: String,
    pub opencode_config_path: String,
    pub skill_stub_path: String,
}

pub fn init_project(
    project_root: impl AsRef<Path>,
    binary_path: &Path,
) -> Result<InitProjectResponse> {
    let project_root = project_root.as_ref();
    fs::create_dir_all(project_root)
        .with_context(|| format!("failed to create {}", project_root.display()))?;
    let config_path = project_root.join("opencode.json");
    let skill_path = project_root.join(".zeropdf").join("SKILL.md");
    fs::create_dir_all(skill_path.parent().unwrap())?;

    let mut root: Value = if config_path.exists() {
        serde_json::from_str(&fs::read_to_string(&config_path)?)?
    } else {
        json!({
            "$schema": "https://opencode.ai/config.json",
            "mcp": {}
        })
    };

    if root.get("mcp").and_then(Value::as_object).is_none() {
        root["mcp"] = json!({});
    }

    root["mcp"]["zeropdf"] = json!({
        "type": "local",
        "command": [binary_path.display().to_string(), "mcp-stdio"],
        "enabled": true
    });

    fs::write(
        &config_path,
        format!("{}\n", serde_json::to_string_pretty(&root)?),
    )?;
    fs::write(
        &skill_path,
        "# ZeroPDF Local Helper\n\nUse the local zeropdf MCP entry from `opencode.json` for PDF inspection, annotation, and @agent task workflows.\n",
    )?;

    Ok(InitProjectResponse {
        status: "success".to_string(),
        project_root: project_root.display().to_string(),
        opencode_config_path: config_path.display().to_string(),
        skill_stub_path: skill_path.display().to_string(),
    })
}

#[allow(dead_code)]
fn _normalize(path: &Path) -> PathBuf {
    path.to_path_buf()
}
