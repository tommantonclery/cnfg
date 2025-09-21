//! cnfg – declarative configuration loading and validation.

pub mod error;
pub mod loader;
pub mod merge;
pub mod types;
pub mod util;

pub use cnfg_derive::Cnfg;
pub use error::{CnfgError, ValidationErrors};
pub use loader::LoaderExt;
pub use types::{CliSpec, ConfigMeta, FieldSpec, Kind, Validate};
