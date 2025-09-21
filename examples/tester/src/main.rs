use cnfg::{Cnfg, CnfgError}; // bring in the derive macro and error type
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Cnfg)]
struct Database {
    /// host with a default
    #[cnfg(default = "localhost", env = "DB_HOST")]
    host: String,

    /// port with validation (must be in range)
    #[cnfg(default = 5432, cli, validate(range(min = "1024", max = "65535")))]
    port: u16,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
        }
    }
}

/// Application configuration for the tester example.
#[derive(Debug, Serialize, Deserialize, Cnfg)]
struct AppConfig {
    /// application name with a sensible default
    #[cnfg(default = "demo-app", cli)]
    name: String,

    /// optional debug flag, defaults to false
    #[cnfg(default = false, cli)]
    debug: bool,

    /// nested struct also derives Cnfg
    #[serde(default)]
    #[cnfg(nested)]
    database: Database,
}

fn main() {
    match AppConfig::load() {
        Ok(cfg) => {
            println!("Loaded config: {:#?}", cfg);
        }
        Err(CnfgError::HelpPrinted) => {
            // help text already written to stdout by the loader
            std::process::exit(0);
        }
        Err(err) => {
            eprintln!("Config error: {}", err);
            std::process::exit(1);
        }
    }
}
