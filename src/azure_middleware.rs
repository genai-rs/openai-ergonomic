//! Middleware for Azure `OpenAI` authentication.
//!
//! This module provides middleware that adds the appropriate authentication
//! headers for Azure `OpenAI` API requests and transforms paths to Azure format.

use http::Extensions;
use reqwest::{Request, Response};
use reqwest_middleware::{Middleware, Next};
use std::sync::Arc;

/// Middleware that adds Azure `OpenAI` authentication headers and transforms paths.
#[derive(Clone)]
pub struct AzureAuthMiddleware {
    api_key: Arc<String>,
    api_version: Arc<String>,
    deployment: Arc<Option<String>>,
}

impl AzureAuthMiddleware {
    /// Create a new Azure authentication middleware.
    pub fn new(api_key: String, api_version: Option<String>, deployment: Option<String>) -> Self {
        Self {
            api_key: Arc::new(api_key),
            api_version: Arc::new(api_version.unwrap_or_else(|| "2024-02-01".to_string())),
            deployment: Arc::new(deployment),
        }
    }
}

#[async_trait::async_trait]
impl Middleware for AzureAuthMiddleware {
    async fn handle(
        &self,
        mut req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        // Add api-key header for Azure OpenAI
        req.headers_mut()
            .insert("api-key", self.api_key.parse().unwrap());

        // Transform the URL path for Azure `OpenAI`
        let url = req.url().clone();
        let path = url.path();

        // Azure `OpenAI` uses paths like: /openai/deployments/{deployment-id}/chat/completions
        // Standard `OpenAI` uses: /v1/chat/completions
        // We need to transform both /v1/* and /* to /openai/deployments/{deployment}/*
        if let Some(deployment) = self.deployment.as_ref() {
            let new_path = if path.starts_with("/v1/") {
                // Handle /v1/chat/completions -> /openai/deployments/{deployment}/chat/completions
                path.replacen("/v1/", &format!("/openai/deployments/{deployment}/"), 1)
            } else if !path.starts_with("/openai/") {
                // Handle /chat/completions -> /openai/deployments/{deployment}/chat/completions
                format!("/openai/deployments/{deployment}{path}")
            } else {
                // Path already in correct format
                path.to_string()
            };

            if new_path != path {
                let mut new_url = url.clone();
                new_url.set_path(&new_path);

                // Add api-version as query parameter if not already present
                if new_url.query_pairs().all(|(key, _)| key != "api-version") {
                    new_url
                        .query_pairs_mut()
                        .append_pair("api-version", &self.api_version);
                }

                *req.url_mut() = new_url;
            }
        }

        next.run(req, extensions).await
    }
}
