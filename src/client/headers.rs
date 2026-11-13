//! The `OpenAI-Organization` and `OpenAI-Project` headers, which the provider's layer sets on
//! every request to the provider, so that the generated `openai-client-base` operations send
//! them too.
//!
//! A header already on the request is replaced rather than repeated, and a default header of a
//! user-supplied `reqwest::Client` is not added on top, since reqwest only fills in defaults that
//! the request does not have.

use crate::config::Config;
use crate::{Error, Result};
use http::header::{HeaderName, HeaderValue};

const ORGANIZATION: HeaderName = HeaderName::from_static("openai-organization");
const PROJECT: HeaderName = HeaderName::from_static("openai-project");

/// The organization and project headers of `config`, if set.
pub(super) fn organization_headers(config: &Config) -> Result<Vec<(HeaderName, HeaderValue)>> {
    let mut headers = Vec::new();
    if let Some(organization) = config.organization() {
        headers.push((ORGANIZATION, header_value("organization", organization)?));
    }
    if let Some(project) = config.project() {
        headers.push((PROJECT, header_value("project", project)?));
    }
    Ok(headers)
}

fn header_value(name: &str, value: &str) -> Result<HeaderValue> {
    let mut value = HeaderValue::from_str(value)
        .map_err(|_| Error::Config(format!("the {name} ID is not a valid header value")))?;
    value.set_sensitive(true);
    Ok(value)
}
