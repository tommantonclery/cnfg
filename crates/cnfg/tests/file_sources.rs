use cnfg::Cnfg;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Cnfg)]
struct FileConfig {
    #[cnfg(default = "from-default")]
    name: String,

    #[cnfg(default = 3000)]
    port: u16,
}

#[test]
fn loads_from_yaml_and_json() {
    let dir = tempfile::tempdir().expect("tempdir");

    let yaml_path = dir.path().join("config.yaml");
    std::fs::write(&yaml_path, "name: yaml-source\nport: 7777\n").expect("write yaml");

    unsafe { std::env::set_var("CONFIG_FILE", &yaml_path) };
    let yaml_cfg = FileConfig::load().expect("yaml config");
    assert_eq!(yaml_cfg.name, "yaml-source");
    assert_eq!(yaml_cfg.port, 7777);

    unsafe { std::env::remove_var("CONFIG_FILE") };

    let json_path = dir.path().join("config.json");
    std::fs::write(&json_path, r#"{ "name": "json-source", "port": 4242 }"#).expect("write json");

    unsafe { std::env::set_var("CONFIG_FILE", &json_path) };
    let json_cfg = FileConfig::load().expect("json config");
    assert_eq!(json_cfg.name, "json-source");
    assert_eq!(json_cfg.port, 4242);

    unsafe { std::env::remove_var("CONFIG_FILE") };
}
