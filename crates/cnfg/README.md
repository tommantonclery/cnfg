# cnfg

cnfg lets you describe your application's configuration once and load it from files, environment variables, and CLI flags with compile-time guarantees. Derive the `Cnfg` macro on a normal `serde` struct and the crate handles source precedence, deserialization, validation, and help text for you.

## Highlights
- Define your schema with plain Rust structs and `#[derive(Cnfg)]`
- Merge defaults, config files, environment variables, and CLI flags in a predictable order
- Generate `--help` output automatically from doc comments and annotations
- Validate inputs with built-in range/regex/url checks or custom logic
- Compose nested configs without boilerplate and surface rich error messages

## Install
Add the library crate to your `Cargo.toml` (the derive macro is re-exported, no extra dependency required):

```toml
[dependencies]
cnfg = { version = "0.1.1", features = ["yaml", "toml"] }
```

You can also use `cargo add`:

```bash
cargo add cnfg --features yaml,toml
```

Feature flags are optional:

| Feature | Default? | Purpose |
|---------|----------|---------|
| `yaml`  | ✅       | Enable loading `config.yaml` / `config.yml` files |
| `toml`  | ✅       | Enable loading `config.toml` files |
| `json`  | ✅       | Enable loading `config.json` files |

Disable features if you want a smaller dependency tree:

```toml
cnfg = { version = "0.1.1", default-features = false, features = ["toml"] }
```

## Define and Load Configuration
Create a normal `serde` struct, derive `Cnfg`, and annotate each field with the sources you care about.

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

The generated loader combines all sources in the following order (later sources win):

1. Struct defaults & `#[cnfg(default = ...)]`
2. Parsed config file (`CONFIG_FILE` env override or `config.{toml,yaml,json}`)
3. Environment variables declared with `#[cnfg(env = "NAME")]`
4. Command line flags declared with `#[cnfg(cli)]`

Missing required values surface as `CnfgError::Validation` with field-qualified error messages.

## CLI Help for Free
Doc comments flow into the generated CLI help. Call `AppConfig::help()` or run your binary with `--help` to see output like:

```
Usage:
  <binary> [OPTIONS]

Options:
  --name <value>            Name used for logging and help output [default: demo-app]
  --debug                   Toggle verbose logging (--debug or DEBUG=true)
```

When users pass `--help`, cnfg prints the help text and returns `CnfgError::HelpPrinted` so you can exit gracefully.

## Nested Configurations
Break large configs into focused structs. Mark nested fields with `#[cnfg(nested)]` and use `#[serde(default)]` when the child implements `Default`.

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

Nested required fields are tracked automatically (e.g. `database.host`) and validation errors are surfaced with fully-qualified paths.

## Validation Helpers
The derive macro ships with common validators:

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

For custom checks, implement `Validate` manually or extend with `#[cnfg(validate(custom_fn = "path::to::fn"))]` (coming soon).

## Environment and Testing Tips
- Use `dotenvy` support by dropping a `.env` file next to your binary—cnfg will load it automatically.
- In tests, guard environment mutations with a mutex to avoid cross-test interference (see `crates/cnfg/tests/nested.rs`).
- Call `AppConfig::defaults_json()` in unit tests to assert default shapes without touching real files.

## Examples
A runnable example lives in [`examples/tester`](https://github.com/tommantonclery/cnfg/tree/main/examples/tester). It showcases nested configs, CLI help, and validation.

## Minimum Supported Rust Version (MSRV)
The crate targets Rust 1.70.0 or newer (for `OnceLock`). CI and docs assume the 2021 edition.

## Status
The library is production-ready for building internal tools and services. Feedback is welcome—open an issue or discussion on [GitHub](https://github.com/tommantonclery/cnfg).

## License
Licensed under the [Apache License, Version 2.0](LICENSE).
