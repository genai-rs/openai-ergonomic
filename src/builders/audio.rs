//! Audio API builders.
//!
//! This module provides ergonomic builders for working with the `OpenAI` audio
//! endpoints, covering text-to-speech, transcription, and translation
//! workflows. Builders perform lightweight validation and produce values that
//! can be passed directly to `openai-client-base` request functions.

use std::path::{Path, PathBuf};

use openai_client_base::models::transcription_chunking_strategy::TranscriptionChunkingStrategyTextVariantEnum;
use openai_client_base::models::{
    create_speech_request::{
        ResponseFormat as SpeechResponseFormat, StreamFormat as SpeechStreamFormat,
    },
    AudioResponseFormat, CreateSpeechRequest, TranscriptionChunkingStrategy, TranscriptionInclude,
    VadConfig,
};

use crate::{Builder, Error, Result};

/// Builder for text-to-speech requests.
#[derive(Debug, Clone)]
pub struct SpeechBuilder {
    model: String,
    input: String,
    voice: String,
    instructions: Option<String>,
    response_format: Option<SpeechResponseFormat>,
    speed: Option<f64>,
    stream_format: Option<SpeechStreamFormat>,
}

impl SpeechBuilder {
    /// Create a new speech builder with the required model, input text, and voice.
    #[must_use]
    pub fn new(
        model: impl Into<String>,
        input: impl Into<String>,
        voice: impl Into<String>,
    ) -> Self {
        Self {
            model: model.into(),
            input: input.into(),
            voice: voice.into(),
            instructions: None,
            response_format: None,
            speed: None,
            stream_format: None,
        }
    }

    /// Add additional voice instructions (ignored for legacy TTS models).
    #[must_use]
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Choose the audio response format (default is `mp3`).
    #[must_use]
    pub fn response_format(mut self, format: SpeechResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Set the playback speed. Must be between `0.25` and `4.0` inclusive.
    #[must_use]
    pub fn speed(mut self, speed: f64) -> Self {
        self.speed = Some(speed);
        self
    }

    /// Configure streaming output. Unsupported for some legacy models.
    #[must_use]
    pub fn stream_format(mut self, format: SpeechStreamFormat) -> Self {
        self.stream_format = Some(format);
        self
    }

    /// Access the configured model.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Access the configured input text.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Access the configured voice.
    #[must_use]
    pub fn voice(&self) -> &str {
        &self.voice
    }

    /// Access the configured response format, if any.
    #[must_use]
    pub fn response_format_ref(&self) -> Option<SpeechResponseFormat> {
        self.response_format
    }

    /// Access the configured stream format, if any.
    #[must_use]
    pub fn stream_format_ref(&self) -> Option<SpeechStreamFormat> {
        self.stream_format
    }
}

impl Builder<CreateSpeechRequest> for SpeechBuilder {
    fn build(self) -> Result<CreateSpeechRequest> {
        if let Some(speed) = self.speed {
            if !(0.25..=4.0).contains(&speed) {
                return Err(Error::InvalidRequest(format!(
                    "Speech speed {speed} is outside the supported range 0.25–4.0"
                )));
            }
        }

        Ok(CreateSpeechRequest {
            model: self.model,
            input: self.input,
            instructions: self.instructions,
            voice: self.voice,
            response_format: self.response_format,
            speed: self.speed,
            stream_format: self.stream_format,
        })
    }
}

/// Granularity options for transcription timestamps.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TimestampGranularity {
    /// Include timestamps at the segment level.
    Segment,
    /// Include timestamps at the word level (where supported).
    Word,
}

impl TimestampGranularity {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Segment => "segment",
            Self::Word => "word",
        }
    }
}

/// Builder for audio transcription requests.
#[derive(Debug, Clone)]
pub struct TranscriptionBuilder {
    file: PathBuf,
    model: String,
    language: Option<String>,
    prompt: Option<String>,
    response_format: Option<AudioResponseFormat>,
    temperature: Option<f64>,
    stream: Option<bool>,
    chunking_strategy: Option<TranscriptionChunkingStrategy>,
    timestamp_granularities: Vec<TimestampGranularity>,
    include: Vec<TranscriptionInclude>,
}

impl TranscriptionBuilder {
    /// Create a new transcription builder for the given audio file and model.
    #[must_use]
    pub fn new(file: impl AsRef<Path>, model: impl Into<String>) -> Self {
        Self {
            file: file.as_ref().to_path_buf(),
            model: model.into(),
            language: None,
            prompt: None,
            response_format: None,
            temperature: None,
            stream: None,
            chunking_strategy: None,
            timestamp_granularities: Vec::new(),
            include: Vec::new(),
        }
    }

    /// Provide the input language to improve accuracy.
    #[must_use]
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Supply a prompt to guide the transcription style.
    #[must_use]
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set the desired response format (`json`, `text`, `srt`, `verbose_json`, `vtt`).
    #[must_use]
    pub fn response_format(mut self, format: AudioResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Control randomness (0.0–1.0). `0.0` yields deterministic output.
    #[must_use]
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Enable or disable server-side streaming for partial results.
    #[must_use]
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Use the default automatic chunking strategy.
    #[must_use]
    pub fn chunking_strategy_auto(mut self) -> Self {
        self.chunking_strategy = Some(TranscriptionChunkingStrategy::TextVariant(
            TranscriptionChunkingStrategyTextVariantEnum::Auto,
        ));
        self
    }

    /// Provide a custom VAD configuration for chunking.
    #[must_use]
    pub fn chunking_strategy_vad(mut self, config: VadConfig) -> Self {
        self.chunking_strategy = Some(TranscriptionChunkingStrategy::Vadconfig(config));
        self
    }

    /// Disable chunking hints and fall back to API defaults.
    #[must_use]
    pub fn clear_chunking_strategy(mut self) -> Self {
        self.chunking_strategy = None;
        self
    }

    /// Request specific timestamp granularities.
    #[must_use]
    pub fn timestamp_granularities(
        mut self,
        granularities: impl IntoIterator<Item = TimestampGranularity>,
    ) -> Self {
        self.timestamp_granularities = granularities.into_iter().collect();
        self
    }

    /// Append a timestamp granularity option.
    #[must_use]
    pub fn add_timestamp_granularity(mut self, granularity: TimestampGranularity) -> Self {
        if !self.timestamp_granularities.contains(&granularity) {
            self.timestamp_granularities.push(granularity);
        }
        self
    }

    /// Include additional metadata (e.g., log probabilities).
    #[must_use]
    pub fn include(mut self, include: TranscriptionInclude) -> Self {
        if !self.include.contains(&include) {
            self.include.push(include);
        }
        self
    }

    /// Access the source file path.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Access the target model.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Access the selected language.
    #[must_use]
    pub fn language_ref(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// Access the selected response format.
    #[must_use]
    pub fn response_format_ref(&self) -> Option<AudioResponseFormat> {
        self.response_format
    }
}

/// Fully prepared transcription request data.
#[derive(Debug, Clone)]
pub struct TranscriptionRequest {
    /// Audio file to upload for transcription.
    pub file: PathBuf,
    /// Model identifier to use (e.g., `gpt-4o-mini-transcribe`).
    pub model: String,
    /// Optional language hint.
    pub language: Option<String>,
    /// Optional style/context prompt.
    pub prompt: Option<String>,
    /// Desired response format.
    pub response_format: Option<AudioResponseFormat>,
    /// Randomness control (0.0–1.0).
    pub temperature: Option<f64>,
    /// Enable partial streaming responses.
    pub stream: Option<bool>,
    /// Chunking strategy configuration.
    pub chunking_strategy: Option<TranscriptionChunkingStrategy>,
    /// Requested timestamp granularities.
    pub timestamp_granularities: Option<Vec<TimestampGranularity>>,
    /// Additional metadata to include in the response.
    pub include: Option<Vec<TranscriptionInclude>>,
}

impl Builder<TranscriptionRequest> for TranscriptionBuilder {
    fn build(self) -> Result<TranscriptionRequest> {
        if let Some(temperature) = self.temperature {
            if !(0.0..=1.0).contains(&temperature) {
                return Err(Error::InvalidRequest(format!(
                    "Transcription temperature {temperature} is outside the supported range 0.0–1.0"
                )));
            }
        }

        Ok(TranscriptionRequest {
            file: self.file,
            model: self.model,
            language: self.language,
            prompt: self.prompt,
            response_format: self.response_format,
            temperature: self.temperature,
            stream: self.stream,
            chunking_strategy: self.chunking_strategy,
            timestamp_granularities: if self.timestamp_granularities.is_empty() {
                None
            } else {
                Some(self.timestamp_granularities)
            },
            include: if self.include.is_empty() {
                None
            } else {
                Some(self.include)
            },
        })
    }
}

/// Builder for audio translation (audio → English text).
#[derive(Debug, Clone)]
pub struct TranslationBuilder {
    file: PathBuf,
    model: String,
    prompt: Option<String>,
    response_format: Option<AudioResponseFormat>,
    temperature: Option<f64>,
}

impl TranslationBuilder {
    /// Create a new translation builder for the given audio file and model.
    #[must_use]
    pub fn new(file: impl AsRef<Path>, model: impl Into<String>) -> Self {
        Self {
            file: file.as_ref().to_path_buf(),
            model: model.into(),
            prompt: None,
            response_format: None,
            temperature: None,
        }
    }

    /// Provide an optional prompt to guide translation tone.
    #[must_use]
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Select the output format (defaults to JSON).
    #[must_use]
    pub fn response_format(mut self, format: AudioResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Control randomness (0.0–1.0).
    #[must_use]
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Access the configured model.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Access the configured file path.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }
}

/// Fully prepared translation request data.
#[derive(Debug, Clone)]
pub struct TranslationRequest {
    /// Audio file to translate.
    pub file: PathBuf,
    /// Model to use for translation.
    pub model: String,
    /// Optional prompt for style control.
    pub prompt: Option<String>,
    /// Desired output format.
    pub response_format: Option<AudioResponseFormat>,
    /// Randomness control.
    pub temperature: Option<f64>,
}

impl Builder<TranslationRequest> for TranslationBuilder {
    fn build(self) -> Result<TranslationRequest> {
        if let Some(temperature) = self.temperature {
            if !(0.0..=1.0).contains(&temperature) {
                return Err(Error::InvalidRequest(format!(
                    "Translation temperature {temperature} is outside the supported range 0.0–1.0"
                )));
            }
        }

        Ok(TranslationRequest {
            file: self.file,
            model: self.model,
            prompt: self.prompt,
            response_format: self.response_format,
            temperature: self.temperature,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_speech_request() {
        let request = SpeechBuilder::new("gpt-4o-mini-tts", "Hello world", "alloy")
            .instructions("Speak calmly")
            .response_format(SpeechResponseFormat::Wav)
            .speed(1.25)
            .stream_format(SpeechStreamFormat::Audio)
            .build()
            .expect("valid speech builder");

        assert_eq!(request.model, "gpt-4o-mini-tts");
        assert_eq!(request.input, "Hello world");
        assert_eq!(request.voice, "alloy");
        assert_eq!(request.response_format, Some(SpeechResponseFormat::Wav));
        assert_eq!(request.speed, Some(1.25));
        assert_eq!(request.stream_format, Some(SpeechStreamFormat::Audio));
    }

    #[test]
    fn speech_speed_validation() {
        let err = SpeechBuilder::new("gpt-4o-mini-tts", "Hi", "alloy")
            .speed(5.0)
            .build()
            .expect_err("speed outside supported range");
        assert!(matches!(err, Error::InvalidRequest(_)));
    }

    #[test]
    fn builds_transcription_request() {
        let request = TranscriptionBuilder::new("audio.wav", "gpt-4o-mini-transcribe")
            .language("en")
            .prompt("Friendly tone")
            .response_format(AudioResponseFormat::VerboseJson)
            .temperature(0.2)
            .stream(true)
            .chunking_strategy_auto()
            .timestamp_granularities([TimestampGranularity::Segment, TimestampGranularity::Word])
            .include(TranscriptionInclude::Logprobs)
            .build()
            .expect("valid transcription builder");

        assert_eq!(request.model, "gpt-4o-mini-transcribe");
        assert_eq!(request.language.as_deref(), Some("en"));
        assert!(matches!(
            request.timestamp_granularities,
            Some(grans) if grans.contains(&TimestampGranularity::Word)
        ));
        assert!(matches!(
            request.chunking_strategy,
            Some(TranscriptionChunkingStrategy::TextVariant(_))
        ));
        assert!(matches!(
            request.include,
            Some(values) if values.contains(&TranscriptionInclude::Logprobs)
        ));
    }

    #[test]
    fn transcription_temperature_validation() {
        let err = TranscriptionBuilder::new("audio.wav", "gpt-4o-mini-transcribe")
            .temperature(1.2)
            .build()
            .expect_err("temperature outside range");
        assert!(matches!(err, Error::InvalidRequest(_)));
    }

    #[test]
    fn builds_translation_request() {
        let request = TranslationBuilder::new("clip.mp3", "gpt-4o-mini-transcribe")
            .prompt("Keep humour")
            .response_format(AudioResponseFormat::Text)
            .temperature(0.3)
            .build()
            .expect("valid translation builder");

        assert_eq!(request.model, "gpt-4o-mini-transcribe");
        assert_eq!(request.response_format, Some(AudioResponseFormat::Text));
    }

    #[test]
    fn translation_temperature_validation() {
        let err = TranslationBuilder::new("clip.mp3", "gpt-4o-mini-transcribe")
            .temperature(1.5)
            .build()
            .expect_err("temperature outside range");
        assert!(matches!(err, Error::InvalidRequest(_)));
    }
}
