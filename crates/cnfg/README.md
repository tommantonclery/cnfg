# cnfg

**cnfg** is a declarative configuration framework for Rust.
Describe your configuration once with normal structs, derive the `Cnfg` macro, and load values from **files**, **environment variables**, and **CLI flags** — with validation, schema metadata, and help text built in.

## ✨ Highlights

* Define your schema with plain Rust structs and `#[derive(Cnfg)]`
* Merge defaults, config files, environment variables, and CLI flags in a predictable order
* Generate `--help` output automatically from doc comments and annotations
* Validate inputs with built-in checks (range, regex, URL) or custom logic
* Compose nested configs without boilerplate and surface rich error messages

## 📦 Installation

Add the crate to your `Cargo.toml` (the derive macro is re-exported — no extra dependency needed):

```toml
[dependencies]
cnfg = { version = "0.1.1", features = ["yaml", "toml"] }
```

Or use `cargo add`:

```bash
cargo add cnfg --features yaml,toml
```

### Feature Flags

| Feature | Default | Purpose                                 |
| ------- | ------- | --------------------------------------- |
| `yaml`  | ✅       | Load `config.yaml` / `config.yml` files |
| `toml`  | ✅       | Load `config.toml` files                |
| `json`  | ✅       | Load `config.json` files                |

To minimize dependencies:

```toml
cnfg = { version = "0.1.1", default-features = false, features = ["toml"] }
```

## 🚀 Define and Load Configuration

Configuration is just a `serde` struct with annotations:

```rust
use cnfg::{Cnfg, CnfgError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Cnfg)]
struct AppConfig {
    /// Name used for logging and help output
    #[cnfg(default = "demo-app", cli)]
    name: String,

    /// Toggle verbose logging (`--debug` or `DEBUG=true`)
    #[cnfg(default = false, cli, env = "APP_DEBUG")]
    debug: bool,

    /// Database connection string (required when not using defaults)
    #[cnfg(env = "DATABASE_URL", required)]
    database_url: String,
}

fn main() -> Result<(), CnfgError> {
    let cfg = AppConfig::load()?;
    println!("Loaded config: {cfg:#?}");
    Ok(())
}
```

### Source Precedence

When loading, cnfg merges sources in this order (later overrides earlier):

1. Struct defaults & `#[cnfg(default = ...)]`
2. Config file (`CONFIG_FILE` override or `config.{toml,yaml,json}`)
3. Environment variables declared with `#[cnfg(env = "NAME")]`
4. Command-line flags declared with `#[cnfg(cli)]`

Missing required values result in `CnfgError::Validation` with field-qualified error messages.

## 🛠 CLI Help for Free

Doc comments flow into the generated help output:

```
Usage:
  <binary> [OPTIONS]

Options:
  --name <value>    Name used for logging and help output [default: demo-app]
  --debug           Toggle verbose logging (--debug or DEBUG=true)
```

Running with `--help` prints usage and returns `CnfgError::HelpPrinted` so your program can exit gracefully.

## 🧩 Nested Configurations

Split large configs into smaller pieces with `#[cnfg(nested)]`:

```rust
#[derive(Debug, Serialize, Deserialize, Cnfg)]
struct Database {
    #[cnfg(default = "localhost", env = "DB_HOST")]
    host: String,

    #[cnfg(default = 5432, cli, validate(range(min = "1024", max = "65535")))]
    port: u16,
}

#[derive(Debug, Serialize, Deserialize, Cnfg)]
struct AppConfig {
    #[serde(default)]
    #[cnfg(nested)]
    database: Database,
}
```

Errors are tracked with fully-qualified paths (e.g. `database.host`).

## ✅ Validation

Built-in validators:

```rust
#[derive(Debug, Serialize, Deserialize, Cnfg)]
struct Limits {
    #[cnfg(required, validate(range(min = "1", max = "99")))]
    workers: usize,

    #[cnfg(validate(regex = "^[a-z0-9_-]+$"))]
    cluster: String,

    #[cnfg(validate(url))]
    callback: String,
}
```

Custom validation is possible via manual `Validate` impls. Attribute-based custom functions (`#[cnfg(validate(custom_fn = "..."))]`) are on the roadmap.

## 🧪 Tips & Testing

* `.env` files are auto-loaded via `dotenvy`.
* In tests, guard environment changes with a mutex to avoid cross-test interference.
* Use `AppConfig::defaults_json()` to inspect defaults without touching real files.

## 📚 Examples

A runnable demo lives in [`examples/tester`](https://github.com/tommantonclery/cnfg/tree/main/examples/tester).
It shows nested configs, CLI help, and validation in action.

## 🔧 MSRV

Requires **Rust 1.85.0+** (for `OnceLock`). Uses the **2024 edition**.

## 📊 Status

Stable for building internal tools and services. Feedback and contributions are welcome — open an issue or discussion on [GitHub](https://github.com/tommantonclery/cnfg).

## 📄 License

Licensed under the [Apache License, Version 2.0](https://github.com/tommantonclery/cnfg/blob/main/LICENSE).
