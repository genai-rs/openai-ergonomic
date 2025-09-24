//! Image-generation API builders.
//!
//! This module offers ergonomic wrappers around `openai-client-base`'s image
//! endpoints. Builders provide fluent setters, lightweight validation, and
//! produce request types that can be supplied directly to the generated client
//! functions.

use std::path::{Path, PathBuf};

pub use openai_client_base::models::create_image_request::{
    Background, Moderation, OutputFormat, Quality, ResponseFormat, Size, Style,
};
pub use openai_client_base::models::{
    image_input_fidelity_text_variant_enum::ImageInputFidelityTextVariantEnum, CreateImageRequest,
    ImageInputFidelity,
};

use crate::{Builder, Error, Result};

/// Builder for image generation (`/images/generations`).
#[derive(Debug, Clone)]
pub struct ImageGenerationBuilder {
    prompt: String,
    model: Option<String>,
    n: Option<i32>,
    quality: Option<Quality>,
    response_format: Option<ResponseFormat>,
    output_format: Option<OutputFormat>,
    output_compression: Option<i32>,
    stream: Option<bool>,
    partial_images: Option<Option<i32>>,
    size: Option<Size>,
    moderation: Option<Moderation>,
    background: Option<Background>,
    style: Option<Style>,
    user: Option<String>,
}

impl ImageGenerationBuilder {
    /// Create a new generation builder with the required prompt text.
    #[must_use]
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            model: None,
            n: None,
            quality: None,
            response_format: None,
            output_format: None,
            output_compression: None,
            stream: None,
            partial_images: None,
            size: None,
            moderation: None,
            background: None,
            style: None,
            user: None,
        }
    }

    /// Override the model (`gpt-image-1`, `dall-e-3`, etc.).
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set how many images to generate (1-10).
    #[must_use]
    pub fn n(mut self, n: i32) -> Self {
        self.n = Some(n);
        self
    }

    /// Select image quality.
    #[must_use]
    pub fn quality(mut self, quality: Quality) -> Self {
        self.quality = Some(quality);
        self
    }

    /// Select the response format for `dall-e-*` models.
    #[must_use]
    pub fn response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Choose the binary output format (only supported for `gpt-image-1`).
    #[must_use]
    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = Some(format);
        self
    }

    /// Tune image compression (0-100). Only applies to JPEG/WEBP outputs.
    #[must_use]
    pub fn output_compression(mut self, compression: i32) -> Self {
        self.output_compression = Some(compression);
        self
    }

    /// Enable streaming responses.
    #[must_use]
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Configure the number of partial images to emit when streaming (0-3).
    #[must_use]
    pub fn partial_images(mut self, partial_images: Option<i32>) -> Self {
        self.partial_images = Some(partial_images);
        self
    }

    /// Select the output size preset.
    #[must_use]
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Control content moderation (`auto`/`low`).
    #[must_use]
    pub fn moderation(mut self, moderation: Moderation) -> Self {
        self.moderation = Some(moderation);
        self
    }

    /// Configure transparent/opaque backgrounds.
    #[must_use]
    pub fn background(mut self, background: Background) -> Self {
        self.background = Some(background);
        self
    }

    /// Select stylistic hints supported by `dall-e-3`.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Attach an end-user identifier for abuse monitoring.
    #[must_use]
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Borrow the configured prompt.
    #[must_use]
    pub fn prompt(&self) -> &str {
        &self.prompt
    }
}

impl Builder<CreateImageRequest> for ImageGenerationBuilder {
    fn build(self) -> Result<CreateImageRequest> {
        if let Some(n) = self.n {
            if !(1..=10).contains(&n) {
                return Err(Error::InvalidRequest(format!(
                    "Image generation `n` must be between 1 and 10 (got {n})"
                )));
            }
        }

        if let Some(Some(partial)) = self.partial_images {
            if !(0..=3).contains(&partial) {
                return Err(Error::InvalidRequest(format!(
                    "Partial image count must be between 0 and 3 (got {partial})"
                )));
            }
        }

        if let Some(compression) = self.output_compression {
            if !(0..=100).contains(&compression) {
                return Err(Error::InvalidRequest(format!(
                    "Output compression must be between 0 and 100 (got {compression})"
                )));
            }
        }

        Ok(CreateImageRequest {
            prompt: self.prompt,
            model: self.model,
            n: self.n,
            quality: self.quality,
            response_format: self.response_format,
            output_format: self.output_format,
            output_compression: self.output_compression,
            stream: self.stream,
            partial_images: self.partial_images,
            size: self.size,
            moderation: self.moderation,
            background: self.background,
            style: self.style,
            user: self.user,
        })
    }
}

/// Builder describing an image edit request (`/images/edits`).
#[derive(Debug, Clone)]
pub struct ImageEditBuilder {
    image: PathBuf,
    prompt: String,
    mask: Option<PathBuf>,
    background: Option<String>,
    model: Option<String>,
    n: Option<i32>,
    size: Option<String>,
    response_format: Option<String>,
    output_format: Option<String>,
    output_compression: Option<i32>,
    user: Option<String>,
    input_fidelity: Option<ImageInputFidelity>,
    stream: Option<bool>,
    partial_images: Option<i32>,
    quality: Option<String>,
}

impl ImageEditBuilder {
    /// Create a new edit request using a base image and prompt.
    #[must_use]
    pub fn new(image: impl AsRef<Path>, prompt: impl Into<String>) -> Self {
        Self {
            image: image.as_ref().to_path_buf(),
            prompt: prompt.into(),
            mask: None,
            background: None,
            model: None,
            n: None,
            size: None,
            response_format: None,
            output_format: None,
            output_compression: None,
            user: None,
            input_fidelity: None,
            stream: None,
            partial_images: None,
            quality: None,
        }
    }

    /// Supply a mask file that indicates editable regions.
    #[must_use]
    pub fn mask(mut self, mask: impl AsRef<Path>) -> Self {
        self.mask = Some(mask.as_ref().to_path_buf());
        self
    }

    /// Control the generated background (`transparent`, `opaque`, ... as string).
    #[must_use]
    pub fn background(mut self, background: impl Into<String>) -> Self {
        self.background = Some(background.into());
        self
    }

    /// Override the model (defaults to `gpt-image-1`).
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Configure the number of images to generate (1-10).
    #[must_use]
    pub fn n(mut self, n: i32) -> Self {
        self.n = Some(n);
        self
    }

    /// Set the output size (e.g. `1024x1024`).
    #[must_use]
    pub fn size(mut self, size: impl Into<String>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Choose the response format (`url`, `b64_json`).
    #[must_use]
    pub fn response_format(mut self, format: impl Into<String>) -> Self {
        self.response_format = Some(format.into());
        self
    }

    /// Choose the binary output format (`png`, `jpeg`, `webp`).
    #[must_use]
    pub fn output_format(mut self, format: impl Into<String>) -> Self {
        self.output_format = Some(format.into());
        self
    }

    /// Configure output compression (0-100).
    #[must_use]
    pub fn output_compression(mut self, compression: i32) -> Self {
        self.output_compression = Some(compression);
        self
    }

    /// Attach an end-user identifier.
    #[must_use]
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Control fidelity for the input image (`low`/`high`).
    #[must_use]
    pub fn input_fidelity(mut self, fidelity: ImageInputFidelity) -> Self {
        self.input_fidelity = Some(fidelity);
        self
    }

    /// Enable streaming responses for edits.
    #[must_use]
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Configure partial image count when streaming (0-3).
    #[must_use]
    pub fn partial_images(mut self, value: i32) -> Self {
        self.partial_images = Some(value);
        self
    }

    /// Provide quality hints (`low`, `medium`, `high`, ...).
    #[must_use]
    pub fn quality(mut self, quality: impl Into<String>) -> Self {
        self.quality = Some(quality.into());
        self
    }

    /// Borrow the underlying image path.
    #[must_use]
    pub fn image(&self) -> &Path {
        &self.image
    }

    /// Borrow the edit prompt.
    #[must_use]
    pub fn prompt(&self) -> &str {
        &self.prompt
    }
}

/// Fully prepared payload for the edit endpoint.
#[derive(Debug, Clone)]
pub struct ImageEditRequest {
    /// Path to the original image that will be edited.
    pub image: PathBuf,
    /// Natural-language instructions describing the edit.
    pub prompt: String,
    /// Optional mask describing editable regions.
    pub mask: Option<PathBuf>,
    /// Optional background mode (`transparent`, `opaque`, ...).
    pub background: Option<String>,
    /// Model identifier to use for the edit operation.
    pub model: Option<String>,
    /// Number of images to generate (1-10).
    pub n: Option<i32>,
    /// Requested output size (e.g. `1024x1024`).
    pub size: Option<String>,
    /// Response format for non-streaming outputs (`url`, `b64_json`).
    pub response_format: Option<String>,
    /// Binary output format (`png`, `jpeg`, `webp`).
    pub output_format: Option<String>,
    /// Compression level for JPEG/WEBP outputs (0-100).
    pub output_compression: Option<i32>,
    /// End-user identifier for abuse monitoring.
    pub user: Option<String>,
    /// Fidelity configuration for how closely to follow the input image.
    pub input_fidelity: Option<ImageInputFidelity>,
    /// Whether to stream incremental results.
    pub stream: Option<bool>,
    /// Number of partial images to emit while streaming.
    pub partial_images: Option<i32>,
    /// Additional quality hints accepted by the service.
    pub quality: Option<String>,
}

impl Builder<ImageEditRequest> for ImageEditBuilder {
    fn build(self) -> Result<ImageEditRequest> {
        if let Some(n) = self.n {
            if !(1..=10).contains(&n) {
                return Err(Error::InvalidRequest(format!(
                    "Image edit `n` must be between 1 and 10 (got {n})"
                )));
            }
        }

        if let Some(compression) = self.output_compression {
            if !(0..=100).contains(&compression) {
                return Err(Error::InvalidRequest(format!(
                    "Output compression must be between 0 and 100 (got {compression})"
                )));
            }
        }

        if let Some(partial) = self.partial_images {
            if !(0..=3).contains(&partial) {
                return Err(Error::InvalidRequest(format!(
                    "Partial image count must be between 0 and 3 (got {partial})"
                )));
            }
        }

        Ok(ImageEditRequest {
            image: self.image,
            prompt: self.prompt,
            mask: self.mask,
            background: self.background,
            model: self.model,
            n: self.n,
            size: self.size,
            response_format: self.response_format,
            output_format: self.output_format,
            output_compression: self.output_compression,
            user: self.user,
            input_fidelity: self.input_fidelity,
            stream: self.stream,
            partial_images: self.partial_images,
            quality: self.quality,
        })
    }
}

/// Builder describing an image variation request (`/images/variations`).
#[derive(Debug, Clone)]
pub struct ImageVariationBuilder {
    image: PathBuf,
    model: Option<String>,
    n: Option<i32>,
    response_format: Option<String>,
    size: Option<String>,
    user: Option<String>,
}

impl ImageVariationBuilder {
    /// Create a variation builder for the provided image file.
    #[must_use]
    pub fn new(image: impl AsRef<Path>) -> Self {
        Self {
            image: image.as_ref().to_path_buf(),
            model: None,
            n: None,
            response_format: None,
            size: None,
            user: None,
        }
    }

    /// Override the variation model.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Number of variations to generate (1-10).
    #[must_use]
    pub fn n(mut self, n: i32) -> Self {
        self.n = Some(n);
        self
    }

    /// Choose the response format (`url`, `b64_json`).
    #[must_use]
    pub fn response_format(mut self, format: impl Into<String>) -> Self {
        self.response_format = Some(format.into());
        self
    }

    /// Select the image size preset.
    #[must_use]
    pub fn size(mut self, size: impl Into<String>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Attach an end-user identifier.
    #[must_use]
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Borrow the base image path.
    #[must_use]
    pub fn image(&self) -> &Path {
        &self.image
    }
}

/// Fully prepared payload for the variation endpoint.
#[derive(Debug, Clone)]
pub struct ImageVariationRequest {
    /// Path to the source image to transform.
    pub image: PathBuf,
    /// Optional model override.
    pub model: Option<String>,
    /// Number of variations to create (1-10).
    pub n: Option<i32>,
    /// Response format (`url`, `b64_json`).
    pub response_format: Option<String>,
    /// Output size (e.g. `512x512`).
    pub size: Option<String>,
    /// End-user identifier for abuse monitoring.
    pub user: Option<String>,
}

impl Builder<ImageVariationRequest> for ImageVariationBuilder {
    fn build(self) -> Result<ImageVariationRequest> {
        if let Some(n) = self.n {
            if !(1..=10).contains(&n) {
                return Err(Error::InvalidRequest(format!(
                    "Image variation `n` must be between 1 and 10 (got {n})"
                )));
            }
        }

        Ok(ImageVariationRequest {
            image: self.image,
            model: self.model,
            n: self.n,
            response_format: self.response_format,
            size: self.size,
            user: self.user,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_image_generation_request() {
        let request = ImageGenerationBuilder::new("A scenic valley at sunrise")
            .model("gpt-image-1")
            .n(2)
            .quality(Quality::High)
            .response_format(ResponseFormat::B64Json)
            .output_format(OutputFormat::Webp)
            .output_compression(80)
            .stream(true)
            .partial_images(Some(2))
            .size(Size::Variant1536x1024)
            .moderation(Moderation::Auto)
            .background(Background::Transparent)
            .style(Style::Vivid)
            .user("example-user")
            .build()
            .expect("valid generation builder");

        assert_eq!(request.prompt, "A scenic valley at sunrise");
        assert_eq!(request.model.as_deref(), Some("gpt-image-1"));
        assert_eq!(request.n, Some(2));
        assert_eq!(request.quality, Some(Quality::High));
        assert_eq!(request.response_format, Some(ResponseFormat::B64Json));
        assert_eq!(request.output_format, Some(OutputFormat::Webp));
        assert_eq!(request.output_compression, Some(80));
        assert_eq!(request.stream, Some(true));
        assert_eq!(request.partial_images, Some(Some(2)));
        assert_eq!(request.size, Some(Size::Variant1536x1024));
        assert_eq!(request.moderation, Some(Moderation::Auto));
        assert_eq!(request.background, Some(Background::Transparent));
        assert_eq!(request.style, Some(Style::Vivid));
        assert_eq!(request.user.as_deref(), Some("example-user"));
    }

    #[test]
    fn generation_validates_ranges() {
        let err = ImageGenerationBuilder::new("Prompt")
            .n(0)
            .build()
            .unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));

        let err = ImageGenerationBuilder::new("Prompt")
            .output_compression(150)
            .build()
            .unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));

        let err = ImageGenerationBuilder::new("Prompt")
            .partial_images(Some(5))
            .build()
            .unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
    }

    #[test]
    fn builds_image_edit_request() {
        let request = ImageEditBuilder::new("image.png", "Remove the background")
            .mask("mask.png")
            .background("transparent")
            .model("gpt-image-1")
            .n(1)
            .size("1024x1024")
            .response_format("b64_json")
            .output_format("png")
            .output_compression(90)
            .user("user-1")
            .input_fidelity(ImageInputFidelity::TextVariant(
                ImageInputFidelityTextVariantEnum::High,
            ))
            .stream(true)
            .partial_images(1)
            .quality("standard")
            .build()
            .expect("valid edit builder");

        assert_eq!(request.image, PathBuf::from("image.png"));
        assert_eq!(request.prompt, "Remove the background");
        assert_eq!(request.mask, Some(PathBuf::from("mask.png")));
        assert_eq!(request.background.as_deref(), Some("transparent"));
        assert_eq!(request.model.as_deref(), Some("gpt-image-1"));
        assert_eq!(request.size.as_deref(), Some("1024x1024"));
        assert_eq!(request.response_format.as_deref(), Some("b64_json"));
        assert_eq!(request.output_format.as_deref(), Some("png"));
        assert_eq!(request.output_compression, Some(90));
        assert_eq!(request.stream, Some(true));
        assert_eq!(request.partial_images, Some(1));
    }

    #[test]
    fn edit_validates_ranges() {
        let err = ImageEditBuilder::new("image.png", "Prompt")
            .n(20)
            .build()
            .unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));

        let err = ImageEditBuilder::new("image.png", "Prompt")
            .output_compression(150)
            .build()
            .unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));

        let err = ImageEditBuilder::new("image.png", "Prompt")
            .partial_images(5)
            .build()
            .unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
    }

    #[test]
    fn builds_image_variation_request() {
        let request = ImageVariationBuilder::new("image.png")
            .model("dall-e-2")
            .n(3)
            .response_format("url")
            .size("512x512")
            .user("user-123")
            .build()
            .expect("valid variation builder");

        assert_eq!(request.image, PathBuf::from("image.png"));
        assert_eq!(request.model.as_deref(), Some("dall-e-2"));
        assert_eq!(request.n, Some(3));
        assert_eq!(request.response_format.as_deref(), Some("url"));
        assert_eq!(request.size.as_deref(), Some("512x512"));
    }

    #[test]
    fn variation_validates_n() {
        let err = ImageVariationBuilder::new("image.png")
            .n(0)
            .build()
            .unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
    }
}
