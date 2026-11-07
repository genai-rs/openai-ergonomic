//! Middleware that adds the `OpenAI-Organization` and `OpenAI-Project` headers to every request.

use crate::config::Config;
use crate::{Error, Result};
use http::header::{HeaderName, HeaderValue};
use http::Extensions;
use reqwest::{Request, Response};
use reqwest_middleware::{ClientWithMiddleware, Middleware, Next};

const ORGANIZATION: HeaderName = HeaderName::from_static("openai-organization");
const PROJECT: HeaderName = HeaderName::from_static("openai-project");

/// Sets the organization and project headers on each request that goes through the client, so
/// that the generated `openai-client-base` operations send them too.
///
/// A header already on the request is replaced rather than repeated, and a default header of a
/// user-supplied `reqwest::Client` is not added on top, since reqwest only fills in defaults that
/// the request does not have.
#[derive(Clone, Debug)]
struct OrganizationHeaders {
    organization: Option<HeaderValue>,
    project: Option<HeaderValue>,
}

impl OrganizationHeaders {
    fn from_config(config: &Config) -> Result<Option<Self>> {
        let organization = config
            .organization()
            .map(|value| header_value("organization", value))
            .transpose()?;
        let project = config
            .project()
            .map(|value| header_value("project", value))
            .transpose()?;
        if organization.is_none() && project.is_none() {
            return Ok(None);
        }
        Ok(Some(Self {
            organization,
            project,
        }))
    }
}

fn header_value(name: &str, value: &str) -> Result<HeaderValue> {
    let mut value = HeaderValue::from_str(value)
        .map_err(|_| Error::Config(format!("the {name} ID is not a valid header value")))?;
    value.set_sensitive(true);
    Ok(value)
}

#[async_trait::async_trait]
impl Middleware for OrganizationHeaders {
    async fn handle(
        &self,
        mut req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        let headers = req.headers_mut();
        if let Some(organization) = &self.organization {
            headers.insert(ORGANIZATION, organization.clone());
        }
        if let Some(project) = &self.project {
            headers.insert(PROJECT, project.clone());
        }
        next.run(req, extensions).await
    }
}

/// The client with the organization and project headers of `config` added to its requests, or
/// the client unchanged when neither is configured.
pub(super) fn with_organization_headers(
    client: ClientWithMiddleware,
    config: &Config,
) -> Result<ClientWithMiddleware> {
    Ok(match OrganizationHeaders::from_config(config)? {
        Some(headers) => reqwest_middleware::ClientBuilder::from_client(client)
            .with(headers)
            .build(),
        None => client,
    })
}
