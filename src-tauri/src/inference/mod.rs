// ─── Inference Abstraction Layer ───
//
// All LLM calls go through this backend-agnostic interface.
// Implements Section 6.6 of the PRD.

pub mod ollama;
pub mod router;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_stream::Stream;

// Re-export commonly used types
pub use router::{route_model, ModelRoute};

/// Identifies which model to route a request to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelId {
    /// Qwen3-14B Q8_0 — primary reasoning model
    Primary,
    /// Qwen3-8B Q4_K_M — fast interaction model
    Fast,
    /// nomic-embed-text-v1.5 — embedding model
    Embedding,
}

/// Controls inference behavior for a single request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceOptions {
    pub temperature: f64,
    pub max_tokens: u32,
    pub enable_thinking: bool,
    /// Optional system prompt override
    pub system_prompt: Option<String>,
}

impl InferenceOptions {
    /// Build options from a ModelRoute, applying sensible defaults.
    pub fn from_route(route: &router::ModelRoute) -> Self {
        Self {
            temperature: route.default_temperature(),
            max_tokens: route.default_max_tokens(),
            enable_thinking: route.enable_thinking(),
            system_prompt: None,
        }
    }

    /// Set the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set temperature.
    pub fn with_temperature(mut self, t: f64) -> Self {
        self.temperature = t;
        self
    }
}

impl Default for InferenceOptions {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 2048,
            enable_thinking: false,
            system_prompt: None,
        }
    }
}

/// A single message in a chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Callback invoked for each streaming token from an LLM response.
/// Parameters: (token_content, is_final)
pub type StreamingTokenCallback = std::sync::Arc<dyn Fn(&str, bool) + Send + Sync>;

/// Callback invoked during Stockfish engine analysis progress.
/// Parameters: (depth, eval_cp, nodes)
pub type EngineProgressCallback = std::sync::Arc<dyn Fn(u32, i32, Option<u64>) + Send + Sync>;

/// A streaming token from the LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub content: String,
    pub is_final: bool,
}

/// Backend-agnostic inference client.
///
/// Implementations:
/// - `OllamaClient` — current runtime (llama.cpp Vulkan backend)
/// - `VllmClient` — future, when RDNA4 kernel support ships
/// - `LlamaCppClient` — fallback / direct access
#[async_trait]
pub trait InferenceClient: Send + Sync {
    /// Send a chat completion request and receive a streaming response.
    async fn complete(
        &self,
        model: ModelId,
        messages: Vec<Message>,
        options: InferenceOptions,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = Token> + Send>>>;

    /// Generate an embedding vector for the given text.
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>>;

    /// Check if the inference backend is healthy and reachable.
    async fn health_check(&self) -> anyhow::Result<bool> {
        Ok(true)
    }
}
