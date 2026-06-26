//! AI provider trait — abstraction for LLM backends.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Configuration for an AI provider.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiProviderConfig {
    /// Provider type (openai, anthropic, ollama, etc.).
    pub provider: String,
    /// API endpoint URL.
    pub endpoint: String,
    /// API key (or reference to keychain).
    pub api_key: Option<String>,
    /// Model name to use.
    pub model: String,
    /// Maximum tokens in the response.
    pub max_tokens: u32,
    /// Temperature (0.0 - 2.0).
    pub temperature: f32,
    /// Extra provider-specific options.
    pub extra: serde_json::Value,
}

impl Default for AiProviderConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".into(),
            endpoint: "http://localhost:11434/v1".into(),
            api_key: None,
            model: "codellama".into(),
            max_tokens: 4096,
            temperature: 0.1,
            extra: serde_json::Value::Null,
        }
    }
}

/// A message in a conversation with the AI.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiMessage {
    /// Role (system, user, assistant).
    pub role: String,
    /// Message content.
    pub content: String,
}

/// The request sent to an AI provider.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiRequest {
    /// System prompt.
    pub system_prompt: Option<String>,
    /// Conversation messages.
    pub messages: Vec<AiMessage>,
    /// Maximum tokens in response.
    pub max_tokens: Option<u32>,
    /// Temperature.
    pub temperature: Option<f32>,
    /// Whether to stream the response.
    pub stream: bool,
}

/// The response from an AI provider.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiResponse {
    /// The generated text.
    pub content: String,
    /// Token usage statistics.
    pub usage: Option<TokenUsage>,
    /// Model used.
    pub model: Option<String>,
    /// Whether content was filtered/refused.
    pub finish_reason: Option<String>,
}

/// Token usage statistics.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Prompt tokens.
    pub prompt_tokens: u64,
    /// Completion tokens.
    pub completion_tokens: u64,
    /// Total tokens.
    pub total_tokens: u64,
}

/// The AI provider trait.
///
/// Implementations exist for OpenAI, Anthropic, Ollama, OpenRouter,
/// LM Studio, vLLM, and local GGUF models.
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Send a chat completion request.
    async fn chat(&self, request: AiRequest) -> Result<AiResponse, AiError>;

    /// Stream a chat completion response.
    async fn chat_stream(
        &self,
        request: AiRequest,
    ) -> Result<Box<dyn AiStream>, AiError>;

    /// List available models.
    async fn list_models(&self) -> Result<Vec<String>, AiError>;

    /// Get the provider name.
    fn name(&self) -> &str;

    /// Get provider configuration.
    fn config(&self) -> &AiProviderConfig;
}

/// A streaming AI response.
#[async_trait]
pub trait AiStream: Send + Sync {
    /// Get the next chunk of the response. Returns `None` when done.
    async fn next_chunk(&mut self) -> Result<Option<String>, AiError>;

    /// Cancel the stream.
    async fn cancel(&self) -> Result<(), AiError>;
}

/// AI-specific error type.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    /// Network or HTTP error.
    #[error("network error: {0}")]
    Network(String),
    /// Authentication error.
    #[error("authentication error: {0}")]
    Auth(String),
    /// Rate limit exceeded.
    #[error("rate limited: {0}")]
    RateLimited(String),
    /// Invalid request.
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// Provider returned an error.
    #[error("provider error: {0}")]
    Provider(String),
    /// Token limit exceeded.
    #[error("token limit exceeded")]
    TokenLimit,
    /// Content filtered.
    #[error("content filtered: {0}")]
    ContentFiltered(String),
    /// Unexpected response format.
    #[error("unexpected response: {0}")]
    UnexpectedResponse(String),
    /// Operation cancelled.
    #[error("cancelled")]
    Cancelled,
    /// Unknown error.
    #[error("unknown: {0}")]
    Unknown(String),
}
