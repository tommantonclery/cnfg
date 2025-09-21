use cnfg::{Cnfg, CnfgError, ConfigMeta};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[derive(Debug, Default, Serialize, Deserialize, Cnfg)]
struct NestedChild {
    #[cnfg(env = "NESTED_URL", required)]
    url: String,
}

#[derive(Debug, Serialize, Deserialize, Cnfg)]
struct NestedParent {
    #[serde(default)]
    #[cnfg(nested)]
    child: NestedChild,
}

#[test]
fn propagates_nested_environment_values() {
    let _guard = ENV_MUTEX.lock().expect("env mutex poisoned");
    assert!(
        NestedParent::field_specs()
            .iter()
            .any(|spec| spec.path == "child.url")
    );
    unsafe { std::env::set_var("NESTED_URL", "postgres://localhost/one") };
    let cfg = NestedParent::load().expect("nested load succeeds");
    assert_eq!(cfg.child.url, "postgres://localhost/one");
    unsafe { std::env::remove_var("NESTED_URL") };
}

#[test]
fn surfaces_nested_required_errors() {
    let _guard = ENV_MUTEX.lock().expect("env mutex poisoned");
    unsafe { std::env::remove_var("NESTED_URL") };
    assert!(NestedParent::required_fields()
        .iter()
        .any(|path| *path == "child.url"));
    match NestedParent::load() {
        Err(CnfgError::Validation(errors)) => {
            assert!(errors.iter().any(|issue| issue.field == "child.url"));
        }
        Ok(_) => panic!("expected validation failure"),
        Err(other) => panic!("unexpected error: {other:?}"),
    }
}
