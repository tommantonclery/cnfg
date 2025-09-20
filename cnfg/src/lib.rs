#![deny(missing_docs, warnings)]

//! User-facing entry point for the `cnfg` configuration framework.
//!
//! The meta-crate re-exports all commonly used traits, error types, macros,
//! and utilities from the individual `cnfg-*` crates. Downstream users can
//! depend on this crate alone and access the framework with
//! `use cnfg::Config;` without worrying about the internal crate layout.

/// Re-export of the core configuration trait and supporting types.
pub use cnfg_core::{Config, ConfigError, MergeStrategy};
/// Placeholder re-export of the derive macros crate.
pub use cnfg_derive as derive;
/// Placeholder re-export of the configuration sources crate.
pub use cnfg_sources as sources;
/// Placeholder re-export of the validation utilities crate.
pub use cnfg_validate as validate;
