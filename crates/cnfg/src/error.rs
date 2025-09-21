use std::fmt;

/// A structured validation error for a config field.
#[derive(Debug, Clone)]
pub struct Issue {
    pub field: String,
    pub kind: IssueKind,
    pub message: String,
}

/// The type of validation error.
#[derive(Debug, Clone)]
pub enum IssueKind {
    Missing,
    Range,
    Regex,
    Url,
    Custom,
}

/// Aggregated validation errors across multiple fields.
#[derive(Debug, Default)]
pub struct ValidationErrors {
    issues: Vec<Issue>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        Self { issues: Vec::new() }
    }

    pub fn push(&mut self, issue: Issue) {
        self.issues.push(issue);
    }

    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn into_vec(self) -> Vec<Issue> {
        self.issues
    }

    pub fn len(&self) -> usize {
        self.issues.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Issue> {
        self.issues.iter()
    }

    pub fn extend(&mut self, other: ValidationErrors) {
        self.issues.extend(other.issues);
    }

    pub fn with_prefix(mut self, prefix: &str) -> Self {
        for issue in &mut self.issues {
            issue.field = format!("{prefix}.{}", issue.field);
        }
        self
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.issues.is_empty() {
            return write!(f, "no validation errors");
        }
        writeln!(f, "validation failed:")?;
        for issue in &self.issues {
            writeln!(f, "  - {} — {}", issue.field, issue.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

/// The top-level error type for config loading.
#[derive(Debug)]
pub enum CnfgError {
    Io(std::io::Error),
    ParseToml(toml::de::Error),
    ParseJson(serde_json::Error),
    ParseYaml(serde_yaml::Error),
    Validation(ValidationErrors),
    Cli(String),
    Env(String),
    HelpPrinted,
}

impl fmt::Display for CnfgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CnfgError::Io(e) => write!(f, "I/O error: {e}"),
            CnfgError::ParseToml(e) => write!(f, "TOML parse error: {e}"),
            CnfgError::ParseJson(e) => write!(f, "JSON parse error: {e}"),
            CnfgError::ParseYaml(e) => write!(f, "YAML parse error: {e}"),
            CnfgError::Validation(e) => write!(f, "{e}"),
            CnfgError::Cli(msg) => write!(f, "CLI error: {msg}"),
            CnfgError::Env(msg) => write!(f, "Env error: {msg}"),
            CnfgError::HelpPrinted => write!(f, "help requested"),
        }
    }
}

impl std::error::Error for CnfgError {}

impl From<std::io::Error> for CnfgError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<toml::de::Error> for CnfgError {
    fn from(e: toml::de::Error) -> Self {
        Self::ParseToml(e)
    }
}

impl From<serde_json::Error> for CnfgError {
    fn from(e: serde_json::Error) -> Self {
        Self::ParseJson(e)
    }
}

impl From<ValidationErrors> for CnfgError {
    fn from(e: ValidationErrors) -> Self {
        Self::Validation(e)
    }
}

impl From<serde_yaml::Error> for CnfgError {
    fn from(e: serde_yaml::Error) -> Self {
        Self::ParseYaml(e)
    }
}
