use crate::error::{CnfgError, Issue, IssueKind, ValidationErrors};
use crate::merge::{insert_path, merge};
use crate::types::{ConfigMeta, Kind};
use crate::util::{format_doc, format_flag};
use serde::Serialize;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::Path;

/// Trait implemented for every `#[derive(Cnfg)]` struct.
///
/// Provides the `load()` method to build the config and helpers for CLI output.
pub trait LoaderExt: ConfigMeta + Serialize + Sized {
    fn load() -> Result<Self, CnfgError>
    where
        for<'de> Self: serde::Deserialize<'de>,
    {
        // Load a .env file if present (ignore missing files).
        let _ = dotenvy::dotenv();

        // 1. Start with defaults.
        let mut acc = Self::defaults_json();

        // 2. Load config file (CONFIG_FILE env or default names).
        if let Some(file) = load_config_file()? {
            merge(&mut acc, file);
        }

        // 3. Overlay environment variables.
        apply_environment::<Self>(&mut acc)?;

        // 4. Overlay CLI flags.
        let cli_values = parse_cli::<Self>()?;
        merge(&mut acc, cli_values);

        // 5. Check required fields on the assembled value before deserializing.
        let mut errs = ValidationErrors::new();
        check_required::<Self>(&acc, &mut errs);
        if !errs.is_empty() {
            return Err(CnfgError::Validation(errs));
        }

        // 6. Deserialize into the target struct.
        let cfg: Self = serde_json::from_value(acc)?;

        // 7. Run user-defined validations (from derive macro).
        cfg.validate()?;

        Ok(cfg)
    }

    /// Render CLI help text.
    fn help() -> String {
        render_help::<Self>()
    }

    /// Print CLI help text to stdout.
    fn print_help() {
        println!("{}", Self::help());
    }

    /// Run validations for this config (injected by derive macro).
    fn validate(&self) -> Result<(), ValidationErrors>;
}

fn load_config_file() -> Result<Option<Value>, CnfgError> {
    if let Ok(path) = env::var("CONFIG_FILE") {
        return load_file_value(&path).map(Some);
    }

    for candidate in &["config.toml", "config.yaml", "config.yml", "config.json"] {
        if Path::new(candidate).exists() {
            return load_file_value(candidate).map(Some);
        }
    }

    Ok(None)
}

fn load_file_value(path: &str) -> Result<Value, CnfgError> {
    let data = fs::read_to_string(path)?;
    if path.ends_with(".toml") {
        #[cfg(feature = "toml")]
        {
            let t: toml::Value = toml::from_str(&data)?;
            Ok(serde_json::to_value(t)?)
        }
        #[cfg(not(feature = "toml"))]
        {
            Err(CnfgError::Cli(format!(
                "toml support disabled but attempted to load {path}"
            )))
        }
    } else if path.ends_with(".yaml") || path.ends_with(".yml") {
        #[cfg(feature = "yaml")]
        {
            let y: serde_json::Value = serde_yaml::from_str(&data)?;
            Ok(y)
        }
        #[cfg(not(feature = "yaml"))]
        {
            Err(CnfgError::Cli(format!(
                "yaml support disabled but attempted to load {path}"
            )))
        }
    } else if path.ends_with(".json") {
        Ok(serde_json::from_str(&data)?)
    } else {
        Err(CnfgError::Cli(format!(
            "unknown config extension for {path}; use .toml, .yaml, .yml, or .json"
        )))
    }
}

fn apply_environment<T: ConfigMeta>(root: &mut Value) -> Result<(), CnfgError> {
    for spec in T::field_specs() {
        if let Some(env_name) = spec.env {
            if let Ok(val) = env::var(env_name) {
                let parsed = parse_literal(&val, spec.kind)
                    .map_err(|msg| CnfgError::Env(format!("{env_name}: {msg}")))?;
                insert_path(root, &spec.segments(), parsed);
            }
        }
    }
    Ok(())
}

fn parse_cli<T: LoaderExt>() -> Result<Value, CnfgError> {
    let mut args = env::args().skip(1);
    let mut cli_val = Value::Object(Default::default());

    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            <T as LoaderExt>::print_help();
            return Err(CnfgError::HelpPrinted);
        }

        if !arg.starts_with("--") {
            return Err(CnfgError::Cli(format!(
                "unexpected positional argument `{arg}`"
            )));
        }

        let flag = arg.trim_start_matches("--");
        let spec = T::cli_specs()
            .iter()
            .find(|s| s.flag == flag)
            .ok_or_else(|| CnfgError::Cli(format!("unknown flag --{flag}")))?;

        if spec.takes_value {
            let value = args
                .next()
                .ok_or_else(|| CnfgError::Cli(format!("missing value for --{flag}")))?;
            let parsed = parse_literal(&value, spec.kind)
                .map_err(|msg| CnfgError::Cli(format!("--{flag}: {msg}")))?;
            insert_path(&mut cli_val, &spec.segments(), parsed);
        } else {
            insert_path(&mut cli_val, &spec.segments(), Value::Bool(true));
        }
    }

    Ok(cli_val)
}

fn parse_literal(raw: &str, kind: Kind) -> Result<Value, String> {
    match kind {
        Kind::Bool => match raw {
            "1" | "true" | "TRUE" | "True" => Ok(Value::Bool(true)),
            "0" | "false" | "FALSE" | "False" => Ok(Value::Bool(false)),
            _ => Err("expected a boolean".into()),
        },
        Kind::Int => raw
            .parse::<i64>()
            .map(|v| Value::Number(v.into()))
            .map_err(|_| "expected an integer".into()),
        Kind::Float => raw
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .ok_or_else(|| "expected a float".into()),
        Kind::String => Ok(Value::String(raw.to_string())),
        Kind::Object => Err("cannot assign composite value from string".into()),
    }
}

fn check_required<T: ConfigMeta>(value: &Value, errs: &mut ValidationErrors) {
    if T::required_fields().is_empty() {
        return;
    }
    for path in T::required_fields() {
        if !value_has_path(value, path) {
            errs.push(Issue {
                field: (*path).to_string(),
                kind: IssueKind::Missing,
                message: "required field missing".into(),
            });
        }
    }
}

fn value_has_path(value: &Value, path: &str) -> bool {
    let mut current = value;
    for segment in path.split('.') {
        match current {
            Value::Object(map) => match map.get(segment) {
                Some(next) => current = next,
                None => return false,
            },
            _ => return false,
        }
    }
    !matches!(current, Value::Null)
}

fn render_help<T: ConfigMeta>() -> String {
    let mut lines = Vec::new();

    if let Some(doc) = format_doc(T::doc()) {
        lines.push(doc);
        lines.push(String::new());
    }

    lines.push("Usage:".to_string());
    lines.push("  <binary> [OPTIONS]".to_string());

    if !T::cli_specs().is_empty() {
        lines.push(String::new());
        lines.push("Options:".to_string());
        for spec in T::cli_specs() {
            let flag = format_flag(spec.flag, spec.takes_value);
            let mut detail = format_doc(spec.doc).unwrap_or_default();
            if let Some(def) = spec.default {
                if !detail.is_empty() {
                    detail.push(' ');
                }
                detail.push_str(&format!("[default: {def}]"));
            }
            if spec.required {
                if !detail.is_empty() {
                    detail.push(' ');
                }
                detail.push_str("(required)");
            }
            let detail_trimmed = detail.trim().to_string();
            if detail_trimmed.is_empty() {
                lines.push(format!("  {}", flag));
            } else {
                lines.push(format!("  {:<24} {}", flag, detail_trimmed));
            }
        }
    }

    lines.join("\n").trim_end().to_string()
}
