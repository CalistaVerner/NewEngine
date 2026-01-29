use serde::{Deserialize, Serialize};
use std::fs;

use crate::error::{EngineError, EngineResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    #[serde(default = "default_title")]
    pub title: String,

    #[serde(default = "default_width")]
    pub width: u32,

    #[serde(default = "default_height")]
    pub height: u32,

    #[serde(default = "default_fixed_dt_ms")]
    pub fixed_dt_ms: f32,

    #[serde(default)]
    pub modules: Vec<ModuleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    pub id: String,

    /// Любые дополнительные настройки модуля.
    /// Важно: toml::Value не Default, поэтому даём свой default.
    #[serde(default = "default_module_data")]
    pub data: toml::Value,
}

fn default_title() -> String {
    "NewEngine".to_string()
}
fn default_width() -> u32 {
    1280
}
fn default_height() -> u32 {
    720
}
fn default_fixed_dt_ms() -> f32 {
    16.6667
}

fn default_module_data() -> toml::Value {
    toml::Value::Table(toml::map::Map::new())
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: default_title(),
            width: default_width(),
            height: default_height(),
            fixed_dt_ms: default_fixed_dt_ms(),
            modules: vec![
                ModuleConfig {
                    id: "telemetry".into(),
                    data: default_module_data(),
                },
                ModuleConfig {
                    id: "cef".into(),
                    data: default_module_data(),
                },
            ],
        }
    }
}

impl EngineConfig {
    pub fn load_or_default(path: &str) -> EngineResult<Self> {
        match fs::read_to_string(path) {
            Ok(s) => {
                let cfg: EngineConfig = toml::from_str(&s)
                    .map_err(|e| EngineError::Config(format!("parse {}: {}", path, e)))?;
                Ok(cfg)
            }
            Err(_) => Ok(Self::default()),
        }
    }
}