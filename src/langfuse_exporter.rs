//! Configure an OTLP/HTTP exporter for Langfuse with OpenTelemetry 0.32.

use base64::{engine::general_purpose::STANDARD, Engine};
use opentelemetry_otlp::{SpanExporter, WithExportConfig, WithHttpConfig};
use std::collections::HashMap;

/// Error while configuring a Langfuse OTLP exporter.
#[derive(Debug, thiserror::Error)]
pub enum LangfuseExporterError {
    /// One of the required Langfuse credentials is absent.
    #[error("missing environment variable: {0}")]
    MissingEnvironmentVariable(&'static str),
    /// The OTLP exporter could not be built.
    #[error("OTLP exporter error: {0}")]
    ExporterBuild(#[from] opentelemetry_otlp::ExporterBuildError),
}

/// Build an OTLP/HTTP exporter using `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY`,
/// and optional `LANGFUSE_HOST` (default: `https://cloud.langfuse.com`).
///
/// # Errors
///
/// Returns an error if a required credential is missing or the exporter is invalid.
pub fn langfuse_exporter_from_env() -> Result<SpanExporter, LangfuseExporterError> {
    let public_key = std::env::var("LANGFUSE_PUBLIC_KEY")
        .map_err(|_| LangfuseExporterError::MissingEnvironmentVariable("LANGFUSE_PUBLIC_KEY"))?;
    let secret_key = std::env::var("LANGFUSE_SECRET_KEY")
        .map_err(|_| LangfuseExporterError::MissingEnvironmentVariable("LANGFUSE_SECRET_KEY"))?;
    let host =
        std::env::var("LANGFUSE_HOST").unwrap_or_else(|_| "https://cloud.langfuse.com".to_string());
    build_exporter(&host, &public_key, &secret_key)
}

fn build_exporter(
    host: &str,
    public_key: &str,
    secret_key: &str,
) -> Result<SpanExporter, LangfuseExporterError> {
    let endpoint = format!(
        "{}/api/public/otel/v1/traces",
        host.trim().trim_end_matches('/')
    );
    let authorization = format!(
        "Basic {}",
        STANDARD.encode(format!("{public_key}:{secret_key}"))
    );

    Ok(SpanExporter::builder()
        .with_http()
        .with_http_client(reqwest::Client::new())
        .with_endpoint(endpoint)
        .with_headers(HashMap::from([(
            "Authorization".to_string(),
            authorization,
        )]))
        .build()?)
}

#[cfg(test)]
mod tests {
    use super::build_exporter;
    use opentelemetry::trace::{Span, Tracer, TracerProvider};
    use opentelemetry_sdk::{
        runtime::Tokio,
        trace::{span_processor_with_async_runtime::BatchSpanProcessor, SdkTracerProvider},
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn exporter_sends_authenticated_spans_to_langfuse_endpoint() {
        let mut server = mockito::Server::new_async().await;
        let request = server
            .mock("POST", "/api/public/otel/v1/traces")
            .match_header("authorization", "Basic cGstdGVzdDpzay10ZXN0")
            .with_status(200)
            .create_async()
            .await;
        let exporter = build_exporter(&format!("{}/", server.url()), "pk-test", "sk-test")
            .expect("valid Langfuse exporter");
        let provider = SdkTracerProvider::builder()
            .with_span_processor(BatchSpanProcessor::builder(exporter, Tokio).build())
            .build();
        provider.tracer("langfuse-test").start("chat").end();
        provider.force_flush().expect("span export succeeds");
        request.assert_async().await;
        provider.shutdown().expect("shutdown succeeds");
        drop(provider);
        drop(request);
        drop(server);
    }
}
