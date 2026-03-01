use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::cli::GenerateArgs;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct JsBindingsConfig {
    pub module_name: Option<String>,
    pub library_name: Option<String>,
    pub rename: HashMap<String, String>,
    pub exclude: Vec<String>,
    pub external_packages: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootConfig {
    #[serde(default)]
    bindings: BindingsConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct BindingsConfig {
    js: JsBindingsConfig,
}

pub fn load(args: &GenerateArgs) -> Result<JsBindingsConfig> {
    let Some(config_path) = resolve_config_path(args) else {
        return Ok(JsBindingsConfig::default());
    };

    let src = fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read config file: {}", config_path.display()))?;
    let parsed: RootConfig = toml::from_str(&src)
        .with_context(|| format!("failed to parse config file: {}", config_path.display()))?;

    Ok(parsed.bindings.js)
}

fn resolve_config_path(args: &GenerateArgs) -> Option<PathBuf> {
    if let Some(path) = &args.config {
        return Some(path.clone());
    }
    find_uniffi_toml(&args.source)
}

fn find_uniffi_toml(source: &Path) -> Option<PathBuf> {
    source.canonicalize().ok().and_then(|path| {
        let mut cursor = if path.is_dir() {
            path
        } else {
            path.parent()?.to_path_buf()
        };

        loop {
            let candidate = cursor.join("uniffi.toml");
            if candidate.exists() {
                return Some(candidate);
            }
            if !cursor.pop() {
                return None;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_when_no_file() {
        let args = GenerateArgs {
            source: std::path::PathBuf::from("/nonexistent/path.udl"),
            out_dir: std::path::PathBuf::from("/tmp"),
            config: None,
        };
        let cfg = load(&args).unwrap();
        assert!(cfg.module_name.is_none());
        assert!(cfg.library_name.is_none());
        assert!(cfg.rename.is_empty());
        assert!(cfg.exclude.is_empty());
    }
}
