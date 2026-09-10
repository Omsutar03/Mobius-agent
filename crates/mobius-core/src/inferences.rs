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
    pub fn as_provider_str(&self) -> &'static str {
        match self {
            InferenceEngine::LlamaCpp => "llama",
            InferenceEngine::Ollama => "ollama",
            InferenceEngine::GenericOpenAI => "lmstudio",
        }
    }

    pub fn from_provider(provider: &str) -> Self {
        match provider {
            "ollama" => InferenceEngine::Ollama,
            "lmstudio" => InferenceEngine::GenericOpenAI,
            _ => InferenceEngine::LlamaCpp,
        }
    }

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
        config: &crate::config::MobiusConfig,
    ) -> serde_json::Value {
        let (enable_thinking, effort_str) = thinking_level.to_params();

        // 1. Build the mandatory base payload
        let mut payload = match self {
            InferenceEngine::LlamaCpp => {
                json!({
                    "messages": messages,
                    "stream": true,
                    "stream_options": { "include_usage": true },
                    "chat_template_kwargs": {
                        "enable_thinking": enable_thinking,
                        "reasoning_effort": effort_str
                    }
                })
            }
            InferenceEngine::Ollama | InferenceEngine::GenericOpenAI => {
                json!({
                    "model": model_name,
                    "messages": messages,
                    "stream": true,
                    "stream_options": { "include_usage": true },
                })
            }
        };

        // // 2. Fetch the target engine's parameters from the config file
        // let engine_params = match self {
        //     InferenceEngine::LlamaCpp => &config.llama_cpp,
        //     InferenceEngine::Ollama => &config.ollama,
        //     InferenceEngine::GenericOpenAI => &config.generic_openai,
        // };

        // 3. Dynamically inject all keys from config.json into the payload
        // if let Some(payload_obj) = payload.as_object_mut() {
        //     for (key, value) in engine_params {
        //         payload_obj.entry(key.clone()).or_insert(value.clone());
        //     }
        // }

        payload
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
                for model in json.data {
                    active_models.push(ModelInfo {
                        engine: engine.clone(),
                        model_name: model.id.clone(),
                        display_name: format!("{}: {}", label, model.id),
                    });
                }
            }
        }
    }

    active_models
}

// Drop this function at the bottom of inferences.rs
pub async fn get_context_window(
    client: &Client,
    engine: &InferenceEngine,
    model_name: &str,
) -> usize {
    match engine {
        InferenceEngine::LlamaCpp => {
            if let Ok(res) = client.get("http://localhost:8080/v1/models").send().await {
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        for model in data {
                            if model.get("id").and_then(|i| i.as_str()) == Some(model_name) {
                                if let Some(n_ctx) = model
                                    .get("meta")
                                    .and_then(|m| m.get("n_ctx"))
                                    .and_then(|n| n.as_u64())
                                {
                                    return n_ctx as usize;
                                }
                            }
                        }
                    }
                }
            }
            8192 // Fallback
        }
        InferenceEngine::Ollama => {
            if let Ok(res) = client
                .post("http://localhost:11434/api/show")
                .json(&serde_json::json!({"model": model_name}))
                .send()
                .await
            {
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    if let Some(info) = json.get("model_info").and_then(|i| i.as_object()) {
                        for (k, v) in info {
                            if k.ends_with(".context_length") {
                                if let Some(n_ctx) = v.as_u64() {
                                    return n_ctx as usize;
                                }
                            }
                        }
                    }
                }
            }
            4096 // Ollama default fallback
        }
        InferenceEngine::GenericOpenAI => {
            if let Ok(res) = client.get("http://localhost:1234/v1/models").send().await {
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                        for model in data {
                            if model.get("id").and_then(|i| i.as_str()) == Some(model_name) {
                                if let Some(n_ctx) =
                                    model.get("context_length").and_then(|n| n.as_u64())
                                {
                                    return n_ctx as usize;
                                }
                            }
                        }
                    }
                }
            }
            8192 // Fallback
        }
    }
}
