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
    pub rename: HashMap<String, String>,
    pub exclude: Vec<String>,
    pub external_packages: HashMap<String, String>,
    pub custom_types: HashMap<String, CustomTypeConfig>,
}

/// Per-custom-type configuration for lift/lower transforms.
///
/// Configured in `uniffi.toml` as:
/// ```toml
/// [bindings.js.custom_types.Url]
/// type_name = "URL"
/// imports = ["{ URL } from 'url'"]
/// lift = "new URL({})"
/// lower = "{}.toString()"
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CustomTypeConfig {
    /// TypeScript type to use in signatures (default: builtin type).
    pub type_name: Option<String>,
    /// Extra imports needed for the custom type.
    pub imports: Option<Vec<String>>,
    /// Template converting builtin → custom: `"new URL({})"`.
    pub lift: Option<String>,
    /// Template converting custom → builtin: `"{}.toString()"`.
    pub lower: Option<String>,
}

impl CustomTypeConfig {
    /// Apply the lift template (builtin → custom). Identity when unset.
    pub fn lift_expr(&self, builtin_expr: &str) -> String {
        match &self.lift {
            Some(template) => template.replace("{}", builtin_expr),
            None => builtin_expr.to_string(),
        }
    }

    /// Apply the lower template (custom → builtin). Identity when unset.
    pub fn lower_expr(&self, custom_expr: &str) -> String {
        match &self.lower {
            Some(template) => template.replace("{}", custom_expr),
            None => custom_expr.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RootConfig {
    #[serde(default)]
    bindings: BindingsConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
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
            crate_name: None,
        };
        let cfg = load(&args).unwrap();
        assert!(cfg.module_name.is_none());
        assert!(cfg.rename.is_empty());
        assert!(cfg.exclude.is_empty());
        assert!(cfg.custom_types.is_empty());
    }

    #[test]
    fn custom_type_lift_expr_with_template() {
        let ct = CustomTypeConfig {
            type_name: Some("URL".to_string()),
            imports: None,
            lift: Some("new URL({})".to_string()),
            lower: None,
        };
        assert_eq!(ct.lift_expr("rawValue"), "new URL(rawValue)");
    }

    #[test]
    fn custom_type_lift_expr_identity_when_unset() {
        let ct = CustomTypeConfig::default();
        assert_eq!(ct.lift_expr("rawValue"), "rawValue");
    }

    #[test]
    fn custom_type_lower_expr_with_template() {
        let ct = CustomTypeConfig {
            type_name: None,
            imports: None,
            lift: None,
            lower: Some("{}.toString()".to_string()),
        };
        assert_eq!(ct.lower_expr("myUrl"), "myUrl.toString()");
    }

    #[test]
    fn custom_type_lower_expr_identity_when_unset() {
        let ct = CustomTypeConfig::default();
        assert_eq!(ct.lower_expr("myUrl"), "myUrl");
    }

    #[test]
    fn parse_shared_multi_language_toml() {
        let toml_str = r#"
[bindings.js]
module_name = "MyModule"

[bindings.python]
package_name = "my_package"

[bindings.swift]
module_name = "MySwiftModule"

[bindings.kotlin]
package_name = "com.example"
"#;
        let parsed: RootConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(parsed.bindings.js.module_name.as_deref(), Some("MyModule"));
    }

    #[test]
    fn parse_custom_types_config() {
        let toml_str = r#"
[bindings.js.custom_types.Url]
type_name = "URL"
imports = ["{ URL } from 'url'"]
lift = "new URL({})"
lower = "{}.toString()"
"#;
        let parsed: RootConfig = toml::from_str(toml_str).unwrap();
        let cfg = parsed.bindings.js;
        let url_cfg = cfg.custom_types.get("Url").unwrap();
        assert_eq!(url_cfg.type_name.as_deref(), Some("URL"));
        assert_eq!(url_cfg.lift.as_deref(), Some("new URL({})"));
        assert_eq!(url_cfg.lower.as_deref(), Some("{}.toString()"));
        assert_eq!(
            url_cfg.imports.as_ref().unwrap(),
            &vec!["{ URL } from 'url'".to_string()]
        );
    }
}
