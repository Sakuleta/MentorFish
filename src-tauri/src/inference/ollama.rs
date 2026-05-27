// ─── OllamaClient ───
//
// Implements InferenceClient using Ollama's HTTP API.
// Parses streaming NDJSON responses in real time.

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use std::pin::Pin;
use std::time::Duration;

use crate::inference::{InferenceClient, InferenceOptions, Message, MessageRole, ModelId, Token};
use tokio_stream::wrappers::ReceiverStream;

pub struct OllamaClient {
    pub base_url: String,
    pub primary_model: String,
    pub fast_model: String,
    pub embedding_model: String,
    client: Client,
}

impl OllamaClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            primary_model: "qwen3:14b-q8_0".into(),
            fast_model: "qwen3:8b-q4_K_M".into(),
            embedding_model: "nomic-embed-text".into(),
            client: Self::build_client(),
        }
    }

    pub fn with_models(
        base_url: String,
        primary: String,
        fast: String,
        embedding_model: String,
    ) -> Self {
        Self {
            base_url,
            primary_model: primary,
            fast_model: fast,
            embedding_model,
            client: Self::build_client(),
        }
    }

    fn model_tag(&self, model: ModelId) -> &str {
        match model {
            ModelId::Primary => &self.primary_model,
            ModelId::Fast => &self.fast_model,
            ModelId::Embedding => &self.embedding_model,
        }
    }

    /// Build a reqwest client with sensible timeouts to prevent indefinite hangs
    /// when Ollama is unresponsive or a model is still loading.
    fn build_client() -> Client {
        Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build reqwest client")
    }

    /// Check if a model is currently loaded in Ollama.
    pub async fn is_model_loaded(&self, model: ModelId) -> anyhow::Result<bool> {
        let url = format!("{}/api/ps", self.base_url);
        let response = self.client.get(&url).send().await?;
        let json: serde_json::Value = response.json().await?;

        let tag = self.model_tag(model);
        if let Some(models) = json["models"].as_array() {
            for m in models {
                if m["name"].as_str() == Some(tag) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

#[async_trait]
impl InferenceClient for OllamaClient {
    async fn complete(
        &self,
        model: ModelId,
        messages: Vec<Message>,
        options: InferenceOptions,
    ) -> anyhow::Result<Pin<Box<dyn tokio_stream::Stream<Item = Token> + Send>>> {
        let tag = self.model_tag(model).to_string();
        let url = format!("{}/api/chat", self.base_url);

        let ollama_messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": tag,
            "messages": ollama_messages,
            "stream": true,
            "options": {
                "temperature": options.temperature,
                "num_predict": options.max_tokens,
            },
        });

        // Ollama uses a separate field for thinking mode
        if options.enable_thinking {
            body["enable_thinking"] = serde_json::Value::Bool(true);
        }

        let response = self.client.post(&url).json(&body).send().await?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama API error ({}): {}", status, text));
        }

        // Parse streaming NDJSON response
        let (tx, rx) = tokio::sync::mpsc::channel::<Token>(64);
        let mut byte_stream = response.bytes_stream();

        tokio::spawn(async move {
            let mut buffer = String::new();

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // Process complete NDJSON lines
                        while let Some(nl) = buffer.find('\n') {
                            let line = buffer[..nl].trim().to_string();
                            buffer = buffer[nl + 1..].to_string();

                            if line.is_empty() {
                                continue;
                            }

                            match serde_json::from_str::<serde_json::Value>(&line) {
                                Ok(json) => {
                                    let done = json["done"].as_bool().unwrap_or(false);
                                    let content = json["message"]["content"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string();

                                    if tx
                                        .send(Token {
                                            content,
                                            is_final: done,
                                        })
                                        .await
                                        .is_err()
                                    {
                                        return; // receiver dropped
                                    }

                                    if done {
                                        return;
                                    }
                                }
                                Err(e) => {
                                    log::warn!(
                                        "Failed to parse Ollama JSON line: {} — line: {}",
                                        e,
                                        line
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Ollama stream error: {}", e);
                        let _ = tx
                            .send(Token {
                                content: format!("[Stream error: {}]", e),
                                is_final: true,
                            })
                            .await;
                        return;
                    }
                }
            }

            // Stream ended without done=true — send final token
            let _ = tx
                .send(Token {
                    content: String::new(),
                    is_final: true,
                })
                .await;
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let url = format!("{}/api/embeddings", self.base_url);

        let body = serde_json::json!({
            "model": self.embedding_model,
            "prompt": text,
        });

        let response = self.client.post(&url).json(&body).send().await?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Ollama embedding error ({}): {}",
                status,
                text
            ));
        }

        let json: serde_json::Value = response.json().await?;
        let embedding: Vec<f32> = json["embedding"]
            .as_array()
            .ok_or_else(|| {
                anyhow::anyhow!("Invalid embedding response: missing 'embedding' field")
            })?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_tags() {
        let client = OllamaClient::new("http://localhost:11434".into());
        assert_eq!(client.model_tag(ModelId::Primary), "qwen3:14b-q8_0");
        assert_eq!(client.model_tag(ModelId::Fast), "qwen3:8b-q4_K_M");
        assert_eq!(client.model_tag(ModelId::Embedding), "nomic-embed-text");
    }
}
