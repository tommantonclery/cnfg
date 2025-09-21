use serde_json::{Map, Value};

/// Deep merge `other` into `base`.
///
/// - Objects are merged recursively.
/// - Non-objects overwrite.
/// - `other` always takes precedence over `base`.
pub fn merge(base: &mut Value, override_val: Value) {
    match (base, override_val) {
        (Value::Object(base_map), Value::Object(override_map)) => {
            for (k, v) in override_map {
                merge(base_map.entry(k).or_insert(Value::Null), v);
            }
        }
        (slot, v) => {
            *slot = v;
        }
    }
}

/// Insert a nested value into a JSON object given a dotted path.
///
/// Example:
/// ```rust
/// use serde_json::json;
/// use cnfg::merge::insert_path;
///
/// let mut obj = json!({});
/// insert_path(&mut obj, &["database", "url"], json!("postgres://..."));
/// assert_eq!(obj["database"]["url"], "postgres://...");
/// ```
pub fn insert_path(root: &mut Value, path: &[&str], value: Value) {
    // Split off the last segment — that’s where we’ll insert the actual `value`.
    let (last_key, parents) = path.split_last().expect("path must not be empty");

    // Navigate down to the parent object.
    let mut current = root;
    for part in parents {
        // Ensure `current` is an object
        if !current.get(*part).is_some() {
            if let Value::Object(map) = current {
                map.insert((*part).to_string(), Value::Object(Map::new()));
            } else {
                let mut map = Map::new();
                map.insert((*part).to_string(), Value::Object(Map::new()));
                *current = Value::Object(map);
            }
        }

        // Descend one level
        current = current
            .as_object_mut()
            .and_then(|map| map.get_mut(*part))
            .unwrap();
    }

    // Now insert the `value` at the last key (only moved once here)
    if let Value::Object(map) = current {
        map.insert(last_key.to_string(), value);
    } else {
        let mut map = Map::new();
        map.insert(last_key.to_string(), value);
        *current = Value::Object(map);
    }
}
