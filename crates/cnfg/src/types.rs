use crate::error::ValidationErrors;
use serde::Deserialize;

/// Kind of configuration value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Bool,
    Int,
    Float,
    String,
    Object,
}

/// Specification of a config field (for env + defaults).
#[derive(Debug, Clone)]
pub struct FieldSpec {
    /// Field name in the struct
    pub name: &'static str,
    /// Optional env var to read from
    pub env: Option<&'static str>,
    /// Fully-qualified dotted path (e.g. `database.url`).
    pub path: &'static str,
    /// Combined doc comments extracted from the field.
    pub doc: Option<&'static str>,
    /// Kind of value exposed by this field.
    pub kind: Kind,
    /// Default literal (for help output), if any.
    pub default: Option<&'static str>,
    /// Whether this field was declared as required.
    pub required: bool,
}

/// Specification of a CLI argument.
#[derive(Debug, Clone)]
pub struct CliSpec {
    /// Flag name (e.g. `--port` or `--debug`)
    pub flag: &'static str,
    /// Field name this flag maps to
    pub field: &'static str,
    /// The expected kind of value
    pub kind: Kind,
    /// Fully-qualified dotted path for insertion.
    pub path: &'static str,
    /// Documentation extracted from the field.
    pub doc: Option<&'static str>,
    /// Whether the flag expects a following value.
    pub takes_value: bool,
    /// Default literal displayed in help, if any.
    pub default: Option<&'static str>,
    /// Whether this flag is required (mirrors field requirement).
    pub required: bool,
}

/// Trait that all derived config structs will implement
/// via the `#[derive(Cnfg)]` macro.
///
/// This provides compile-time metadata about the config schema.
pub trait ConfigMeta: Sized + for<'de> Deserialize<'de> {
    /// JSON object containing defaults for each field.
    fn defaults_json() -> serde_json::Value;

    /// Metadata about all fields in the struct.
    fn field_specs() -> &'static [FieldSpec];

    /// CLI argument specifications.
    fn cli_specs() -> &'static [CliSpec];

    /// Which fields are required (no default, no option).
    fn required_fields() -> &'static [&'static str];

    /// Aggregated documentation for the struct (from `///` comments).
    fn doc() -> Option<&'static str> {
        None
    }
}

/// Trait implemented by config structs that support runtime validation.
///
/// The derive macro `#[derive(Cnfg)]` will auto-generate
/// an implementation based on attributes like `#[cnfg(validate(...))]`.
pub trait Validate {
    fn validate(&self) -> Result<(), ValidationErrors>;
}

impl FieldSpec {
    /// Produce a copy of this spec with `prefix.` applied to the path.
    pub fn with_prefix(&self, prefix: &'static str) -> Self {
        let combined_path = crate::util::leak_string(format!("{prefix}.{}", self.path));
        Self {
            name: self.name,
            env: self.env,
            path: combined_path,
            doc: self.doc,
            kind: self.kind,
            default: self.default,
            required: self.required,
        }
    }

    /// Return dotted path segments for this field.
    pub fn segments(&self) -> Vec<&'static str> {
        self.path.split('.').collect()
    }
}

impl CliSpec {
    /// Produce a copy of this spec with the provided prefix applied.
    pub fn with_prefix(&self, prefix: &'static str) -> Self {
        let combined_path = crate::util::leak_string(format!("{prefix}.{}", self.path));
        let combined_flag = if self.flag.is_empty() {
            crate::util::leak_string(prefix.replace('_', "-"))
        } else {
            crate::util::leak_string(format!("{}-{}", prefix.replace('_', "-"), self.flag))
        };
        Self {
            flag: combined_flag,
            field: self.field,
            kind: self.kind,
            path: combined_path,
            doc: self.doc,
            takes_value: self.takes_value,
            default: self.default,
            required: self.required,
        }
    }

    /// Return dotted path segments for this CLI flag.
    pub fn segments(&self) -> Vec<&'static str> {
        self.path.split('.').collect()
    }
}
