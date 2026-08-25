use crate::engine::ThinkingLevel;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum InferenceEngine {
    LlamaCpp,
    Ollama,
    GenericOpenAI,
}

impl InferenceEngine {
    pub fn get_url(&self) -> &'static str {
        match self {
            InferenceEngine::LlamaCpp => "http://localhost:8080/v1/chat/completions",
            InferenceEngine::Ollama => "http://localhost:11434/v1/chat/completions",
            InferenceEngine::GenericOpenAI => "http://localhost:1234/v1/chat/completions",
        }
    }

    pub fn build_payload(
        &self,
        messages: serde_json::Value,
        thinking_level: ThinkingLevel,
        model_name: &str,
    ) -> serde_json::Value {
        let (enable_thinking, effort_str) = thinking_level.to_params();

        match self {
            InferenceEngine::LlamaCpp => {
                // llama.cpp expects chat_template_kwargs
                json!({
                    "messages": messages,
                    "stream": true,
                    "chat_template_kwargs": {
                        "enable_thinking": enable_thinking,
                        "reasoning_effort": effort_str
                    },
                    "temperature": 0.1,
                    "min_p": 0.05,
                    "top_p": 0.9
                })
            }
            InferenceEngine::Ollama | InferenceEngine::GenericOpenAI => {
                // Standard OpenAI spec
                json!({
                    "model": model_name,
                    "messages": messages,
                    "stream": true,
                    "temperature": 0.1,
                    "top_p": 0.9
                })
            }
        }
    }
}

#[derive(Debug)]
pub struct ModelInfo {
    pub engine: InferenceEngine,
    pub model_name: String,
    pub display_name: String,
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelData>,
}

#[derive(Deserialize)]
struct ModelData {
    id: String,
}

/// Scans standard ports to see which local AI runners are currently active.
pub async fn discover_local_models() -> Vec<ModelInfo> {
    let client = Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();

    let mut active_models = Vec::new();

    let endpoints = vec![
        (
            InferenceEngine::LlamaCpp,
            "http://localhost:8080/v1/models",
            "llama.cpp",
        ),
        (
            InferenceEngine::Ollama,
            "http://localhost:11434/v1/models",
            "Ollama",
        ),
        (
            InferenceEngine::GenericOpenAI,
            "http://localhost:1234/v1/models",
            "LM Studio/Generic",
        ),
    ];

    for (engine, url, label) in endpoints {
        if let Ok(res) = client.get(url).send().await {
            if let Ok(json) = res.json::<ModelsResponse>().await {
                if let Some(first_model) = json.data.first() {
                    active_models.push(ModelInfo {
                        engine,
                        model_name: first_model.id.clone(),
                        display_name: format!("{}: {}", label, first_model.id),
                    });
                }
            }
        }
    }

    active_models
}
