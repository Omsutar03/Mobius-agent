use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MobiusConfig {
    pub llama_cpp: serde_json::Map<String, serde_json::Value>,
    pub ollama: serde_json::Map<String, serde_json::Value>,
    pub generic_openai: serde_json::Map<String, serde_json::Value>,
}

impl Default for MobiusConfig {
    fn default() -> Self {
        // Standard defaults for local LLMs
        let default_params = json!({
            "temperature": 0.1,
            "min_p": 0.05,
            "top_p": 0.9
        })
        .as_object()
        .unwrap()
        .clone();

        MobiusConfig {
            llama_cpp: default_params.clone(),
            ollama: default_params.clone(),
            generic_openai: default_params,
        }
    }
}

impl MobiusConfig {
    pub fn get_config_path() -> PathBuf {
        let home = env::var("HOME").expect("Could not find HOME directory");
        let dir = PathBuf::from(home).join(".config").join("mobius");

        if !dir.exists() {
            fs::create_dir_all(&dir).expect("Failed to create config directory");
        }
        dir.join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::get_config_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&data) {
                    return config;
                }
            }
        }

        // If file doesn't exist or JSON is invalid, create defaults and save
        let default_cfg = Self::default();
        default_cfg.save();
        default_cfg
    }

    pub fn save(&self) {
        let path = Self::get_config_path();
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, data);
        }
    }
}
