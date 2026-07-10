use async_trait::async_trait;
use operit_providers::{AIService, AiServiceError};

/// Represents a provider implemented by a third-party crate.
struct ExampleProvider {
    endpoint: String,
    model: String,
}

impl ExampleProvider {
    /// Creates a provider from third-party configuration.
    fn new(endpoint: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            model: model.into(),
        }
    }
}

#[async_trait]
impl AIService for ExampleProvider {
    /// Returns the provider and model identifier used by Operit diagnostics.
    fn provider_model(&self) -> String {
        format!("third-party:{}", self.model)
    }

    /// Performs the provider-specific connection check.
    async fn test_connection(&self) -> Result<String, AiServiceError> {
        if !self.endpoint.starts_with("https://") {
            return Err(AiServiceError::ConnectionFailed(
                "the example endpoint must use HTTPS".to_string(),
            ));
        }
        Ok(format!(
            "{} is ready at {}",
            self.provider_model(),
            self.endpoint
        ))
    }
}

/// Creates and invokes a third-party provider through the public provider API.
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let provider = ExampleProvider::new("https://provider.example.com/v1", "example-chat-model");
    let result = provider
        .test_connection()
        .await
        .expect("the example provider configuration must be valid");
    println!("{result}");
}
