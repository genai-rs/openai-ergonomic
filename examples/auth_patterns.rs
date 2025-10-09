#![allow(clippy::uninlined_format_args)]
//! Authentication patterns and configuration examples.
//!
//! This example demonstrates:
//! - Environment variable configuration
//! - API key authentication
//! - Organization ID configuration
//! - Project ID configuration
//! - Custom headers
//! - Proxy configuration
//! - Multiple client configurations
//!
//! Run with: `cargo run --example auth_patterns`

use openai_ergonomic::{Client, Config, Result};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Authentication Patterns ===\n");

    // Example 1: Environment variable authentication
    println!("1. Environment Variable Authentication:");
    env_var_auth().await?;

    // Example 2: Direct API key
    println!("\n2. Direct API Key:");
    direct_api_key().await?;

    // Example 3: Organization configuration
    println!("\n3. Organization Configuration:");
    organization_config()?;

    // Example 4: Project configuration
    println!("\n4. Project Configuration:");
    project_config()?;

    // Example 5: Custom headers
    println!("\n5. Custom Headers:");
    custom_headers()?;

    // Example 6: Proxy configuration
    println!("\n6. Proxy Configuration:");
    proxy_config()?;

    // Example 7: Multiple client configurations
    println!("\n7. Multiple Client Configurations:");
    multiple_clients()?;

    // Example 8: Configuration validation
    println!("\n8. Configuration Validation:");
    config_validation()?;

    // Example 9: Secure key management
    println!("\n9. Secure Key Management:");
    secure_key_management();

    Ok(())
}

async fn env_var_auth() -> Result<()> {
    // Standard environment variables:
    // - OPENAI_API_KEY: Your API key
    // - OPENAI_ORG_ID: Optional organization ID
    // - OPENAI_PROJECT_ID: Optional project ID
    // - OPENAI_BASE_URL: Optional custom base URL

    // Check if environment variables are set
    if env::var("OPENAI_API_KEY").is_err() {
        println!("Warning: OPENAI_API_KEY not set");
        println!("Set it with: export OPENAI_API_KEY=your-key-here");
        return Ok(());
    }

    // Create client from environment
    let client = Client::from_env()?.build();
    println!("Client created from environment variables");

    // Test the client
    match client.send_chat(client.chat_simple("Hello")).await {
        Ok(response) => {
            if let Some(content) = response.content() {
                println!("Response: {}", content);
            } else {
                println!("Response: (no content)");
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    Ok(())
}

async fn direct_api_key() -> Result<()> {
    // Create client with direct API key
    let api_key = "sk-your-api-key-here"; // Replace with actual key
    let config = Config::builder().api_key(api_key).build();
    let client = Client::builder(config)?.build();

    println!("Client created with direct API key");

    // Note: This will fail with invalid key
    match client.send_chat(client.chat_simple("Hello")).await {
        Ok(response) => {
            if let Some(content) = response.content() {
                println!("Response: {}", content);
            } else {
                println!("Response: (no content)");
            }
        }
        Err(e) => println!("Expected error with demo key: {}", e),
    }

    Ok(())
}

fn organization_config() -> Result<()> {
    // Configure client with organization ID
    let config = Config::builder()
        .api_key("your-api-key")
        .organization("org-123456789")
        .build();

    let _client = Client::builder(config)?.build();
    println!("Client configured with organization ID");

    // Organization ID is sent in headers with all requests
    // Useful for:
    // - Usage tracking per organization
    // - Access control
    // - Billing segregation

    Ok(())
}

fn project_config() -> Result<()> {
    // Configure client with project ID
    let config = Config::builder()
        .api_key("your-api-key")
        .project("proj-abc123")
        .build();

    let _client = Client::builder(config)?.build();
    println!("Client configured with project ID");

    // Project ID helps with:
    // - Fine-grained usage tracking
    // - Project-specific rate limits
    // - Cost allocation

    Ok(())
}

fn custom_headers() -> Result<()> {
    // Note: Custom headers are not yet supported in the current API
    // This would typically be used for:
    // - Request tracing
    // - A/B testing
    // - Custom routing

    let config = Config::builder().api_key("your-api-key").build();

    let _client = Client::builder(config)?.build();
    println!("Client configured (custom headers not yet supported)");

    // TODO: Add support for custom headers in the future
    println!("Custom headers feature planned for future implementation");

    Ok(())
}

fn proxy_config() -> Result<()> {
    // Note: Proxy configuration is not yet supported in the current API
    // This would typically be used for:
    // - Enterprise security policies
    // - Request monitoring
    // - Network isolation

    let config = Config::builder().api_key("your-api-key").build();

    let _client = Client::builder(config)?.build();
    println!("Client configured (proxy support not yet available)");

    // TODO: Add proxy support in the future
    println!("Proxy configuration feature planned for future implementation");

    Ok(())
}

fn multiple_clients() -> Result<()> {
    // Create multiple clients for different use cases

    // Production client with retries and longer timeout
    let prod_config = Config::builder()
        .api_key("prod-api-key")
        .organization("org-prod")
        .timeout_seconds(60)
        .max_retries(5)
        .build();
    let prod_client = Client::builder(prod_config)?.build();

    // Development client with debug logging
    let dev_config = Config::builder()
        .api_key("dev-api-key")
        .organization("org-dev")
        .api_base("https://api.openai-dev.com") // Custom endpoint
        .timeout_seconds(10)
        .build();
    let dev_client = Client::builder(dev_config)?.build();

    // Test client with mocked responses
    let test_config = Config::builder()
        .api_key("test-api-key")
        .api_base("http://localhost:8080") // Local mock server
        .build();
    let _test_client = Client::builder(test_config)?.build();

    println!("Created multiple clients:");
    println!("- Production client with retries");
    println!("- Development client with custom endpoint");
    println!("- Test client with mock server");

    // Use appropriate client based on context
    let _client = if cfg!(debug_assertions) {
        &dev_client
    } else {
        &prod_client
    };

    println!(
        "Using {} client",
        if cfg!(debug_assertions) {
            "dev"
        } else {
            "prod"
        }
    );

    Ok(())
}

fn config_validation() -> Result<()> {
    // Validate configuration before use

    fn validate_api_key(key: &str) -> bool {
        // OpenAI API keys typically start with "sk-"
        key.starts_with("sk-") && key.len() > 20
    }

    fn validate_org_id(org: &str) -> bool {
        // Organization IDs typically start with "org-"
        org.starts_with("org-") && org.len() > 4
    }

    let api_key = "sk-test-key-123456789";
    let org_id = "org-12345";

    if !validate_api_key(api_key) {
        println!("Warning: API key format appears invalid");
    }

    if !validate_org_id(org_id) {
        println!("Warning: Organization ID format appears invalid");
    }

    // Build config only if validation passes
    if validate_api_key(api_key) {
        let config = Config::builder()
            .api_key(api_key)
            .organization(org_id)
            .build();

        let _client = Client::builder(config)?.build();
        println!("Configuration validated and client created");
    }

    Ok(())
}

fn secure_key_management() {
    println!("Secure Key Management Best Practices:");
    println!();

    // 1. Never hardcode keys
    println!("1. Never hardcode API keys in source code");
    // BAD: let api_key = "sk-abc123...";
    // GOOD: let api_key = env::var("OPENAI_API_KEY")?;

    // 2. Use environment variables
    println!("2. Use environment variables for local development");
    if let Ok(key) = env::var("OPENAI_API_KEY") {
        println!("   API key loaded from environment (length: {})", key.len());
    }

    // 3. Use secrets management in production
    println!("3. Use proper secrets management in production:");
    println!("   - AWS Secrets Manager");
    println!("   - Azure Key Vault");
    println!("   - HashiCorp Vault");
    println!("   - Kubernetes Secrets");

    // 4. Rotate keys regularly
    println!("4. Rotate API keys regularly");

    // 5. Use different keys per environment
    println!("5. Use different API keys for each environment:");
    let keys_by_env = vec![
        ("development", "OPENAI_API_KEY_DEV"),
        ("staging", "OPENAI_API_KEY_STAGING"),
        ("production", "OPENAI_API_KEY_PROD"),
    ];

    for (env_name, env_var) in keys_by_env {
        if env::var(env_var).is_ok() {
            println!("   ✓ {} key configured", env_name);
        } else {
            println!("   ✗ {} key not found", env_name);
        }
    }

    // 6. Monitor key usage
    println!("6. Monitor API key usage:");
    println!("   - Set up usage alerts");
    println!("   - Track per-project costs");
    println!("   - Audit access logs");

    // Example: Loading from a secrets file (for demonstration)
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let secret_file = "/tmp/openai_secret";

        // Check file permissions (should be 600)
        if let Ok(metadata) = fs::metadata(secret_file) {
            let permissions = metadata.permissions();
            if permissions.mode() & 0o777 == 0o600 {
                println!("7. Secret file has correct permissions (600)");
            } else {
                println!("7. Warning: Secret file permissions too open!");
            }
        }
    }
}
