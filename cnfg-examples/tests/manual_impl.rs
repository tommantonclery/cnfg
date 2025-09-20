use cnfg_core::{Config, ConfigError, MergeStrategy};

#[derive(Debug)]
struct DummyConfig {
    debug: bool,
    strategy: MergeStrategy,
}

impl Config for DummyConfig {
    fn load() -> Result<Self, ConfigError> {
        Ok(Self {
            debug: true,
            strategy: MergeStrategy::Override,
        })
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.debug {
            Ok(())
        } else {
            Err(ConfigError::Validation {
                message: "debug must be enabled".into(),
            })
        }
    }
}

#[test]
fn manual_config_impl_succeeds() -> Result<(), ConfigError> {
    let config = DummyConfig::load()?;
    config.validate()?;
    assert!(matches!(config.strategy, MergeStrategy::Override));
    Ok(())
}
