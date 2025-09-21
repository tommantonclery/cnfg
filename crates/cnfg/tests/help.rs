use cnfg::{Cnfg, LoaderExt};
use serde::{Deserialize, Serialize};

/// Demonstrates CLI help output extraction.
#[derive(Debug, Serialize, Deserialize, Cnfg)]
struct HelpConfig {
    /// Listening port for the HTTP API.
    #[cnfg(default = 8080, cli)]
    port: u16,

    /// Enable verbose logging output.
    #[cnfg(default = false, cli)]
    verbose: bool,
}

#[test]
fn renders_cli_help() {
    let help = HelpConfig::help();
    assert!(help.contains("Usage:"));
    assert!(help.contains("--port <value>"));
    assert!(help.contains("Listening port"));
    assert!(help.contains("--verbose"));
}
