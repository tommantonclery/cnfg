#![deny(missing_docs, warnings)]

//! Core traits and error types for the `cnfg` configuration framework.
//!
//! This crate defines the `Config` trait alongside supporting enums and
//! errors used throughout the configuration ecosystem. Library authors can
//! depend on `cnfg-core` without pulling in any optional configuration
//! sources or validation helpers.
//!
//! # Example
//!
//! ```
//! use cnfg_core::{Config, ConfigError};
//!
//! #[derive(Debug)]
//! struct SampleConfig {
//!     debug: bool,
//! }
//!
//! impl Config for SampleConfig {
//!     fn load() -> Result<Self, ConfigError> {
//!         Ok(Self { debug: true })
//!     }
//!
//!     fn validate(&self) -> Result<(), ConfigError> {
//!         if self.debug {
//!             Ok(())
//!         } else {
//!             Err(ConfigError::Validation {
//!                 message: "debug must be enabled".into(),
//!             })
//!         }
//!     }
//! }
//!
//! # fn main() -> Result<(), ConfigError> {
//! let cfg = SampleConfig::load()?;
//! cfg.validate()?;
//! # Ok(())
//! # }
//! ```

use thiserror::Error;

/// A trait implemented by all configuration types.
///
/// Types implementing `Config` are expected to provide a static `load`
/// function that aggregates configuration from known sources as well as a
/// `validate` method that ensures the resulting configuration is usable.
/// Implementations should strive to keep both operations side-effect free
/// and deterministic.
pub trait Config: Sized {
    /// Load the configuration from the configured sources.
    ///
    /// Implementations typically gather configuration from multiple
    /// providers before merging the results using a [`MergeStrategy`].
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when no valid configuration can be
    /// constructed.
    fn load() -> Result<Self, ConfigError>;

    /// Validate the configuration according to custom invariants.
    ///
    /// Implementations should ensure that all required invariants are
    /// checked. The method is expected to be idempotent and not mutate the
    /// configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Validation`] when validation fails.
    fn validate(&self) -> Result<(), ConfigError>;
}

/// Strategies available when merging multiple configuration sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Later sources override earlier values during merging.
    Override,
    /// Earlier values are preserved unless later sources produce new keys.
    Preserve,
}

/// Error variants produced when loading or validating configuration data.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Errors that occur during the loading phase.
    #[error("failed to load configuration: {message}")]
    Load {
        /// Description of the loading failure.
        message: String,
    },
    /// Errors produced by merging multiple configuration sources.
    #[error("failed to merge configuration: {message}")]
    Merge {
        /// Description of the merging failure.
        message: String,
    },
    /// Errors emitted by validation logic.
    #[error("configuration validation failed: {message}")]
    Validation {
        /// Description of the validation failure.
        message: String,
    },
}
