//! OpenTelemetry Semantic Conventions for Generative AI
//!
//! This module defines constants for OpenTelemetry semantic conventions
//! for generative AI operations, following the official specification:
//! <https://opentelemetry.io/docs/specs/semconv/gen-ai/>
//!
//! We use the official attribute names from the opentelemetry-semantic-conventions
//! crate and define our own operation values until they are officially available.

// Re-export the official GenAI attribute constants from the crate
pub use opentelemetry_semantic_conventions::attribute::{
    GEN_AI_OPERATION_NAME, GEN_AI_REQUEST_MODEL, GEN_AI_SYSTEM,
};

/// Standard operation names for `OpenAI` APIs
pub mod operation_names {
    /// Chat completion operation (`ChatGPT`, `GPT-4`, etc.)
    pub const CHAT: &str = "chat";

    /// Responses operation (structured outputs API)
    pub const RESPONSES: &str = "responses";

    /// Create embeddings operation
    pub const EMBEDDINGS: &str = "embeddings";

    /// Text completion operation (legacy)
    pub const TEXT_COMPLETION: &str = "text_completion";

    /// Image generation operation (`DALL-E`)
    pub const IMAGE_GENERATION: &str = "image_generation";

    /// Image edit operation
    pub const IMAGE_EDIT: &str = "image_edit";

    /// Image variation operation
    pub const IMAGE_VARIATION: &str = "image_variation";

    /// Audio transcription operation (`Whisper`)
    pub const AUDIO_TRANSCRIPTION: &str = "audio_transcription";

    /// Audio translation operation
    pub const AUDIO_TRANSLATION: &str = "audio_translation";

    /// Text-to-speech operation
    pub const AUDIO_SPEECH: &str = "audio_speech";

    /// Content moderation operation
    pub const MODERATION: &str = "moderation";

    /// File upload operation
    pub const FILE_UPLOAD: &str = "file_upload";
    /// File retrieve operation
    pub const FILE_RETRIEVE: &str = "file_retrieve";
    /// File list operation
    pub const FILE_LIST: &str = "file_list";
    /// File delete operation
    pub const FILE_DELETE: &str = "file_delete";
    /// File download operation
    pub const FILE_DOWNLOAD: &str = "file_download";

    /// Assistant create operation
    pub const ASSISTANT_CREATE: &str = "assistant_create";
    /// Assistant list operation
    pub const ASSISTANT_LIST: &str = "assistant_list";
    /// Assistant retrieve operation
    pub const ASSISTANT_RETRIEVE: &str = "assistant_retrieve";
    /// Assistant update operation
    pub const ASSISTANT_UPDATE: &str = "assistant_update";
    /// Assistant delete operation
    pub const ASSISTANT_DELETE: &str = "assistant_delete";

    /// Thread create operation
    pub const THREAD_CREATE: &str = "thread_create";

    /// Message create operation
    pub const MESSAGE_CREATE: &str = "message_create";
    /// Message list operation
    pub const MESSAGE_LIST: &str = "message_list";
    /// Message retrieve operation
    pub const MESSAGE_RETRIEVE: &str = "message_retrieve";

    /// Run create operation
    pub const RUN_CREATE: &str = "run_create";
    /// Run list operation
    pub const RUN_LIST: &str = "run_list";
    /// Run retrieve operation
    pub const RUN_RETRIEVE: &str = "run_retrieve";
    /// Run cancel operation
    pub const RUN_CANCEL: &str = "run_cancel";
    /// Run submit tool outputs operation
    pub const RUN_SUBMIT_TOOL_OUTPUTS: &str = "run_submit_tool_outputs";

    /// Run step list operation
    pub const RUN_STEP_LIST: &str = "run_step_list";
    /// Run step retrieve operation
    pub const RUN_STEP_RETRIEVE: &str = "run_step_retrieve";

    /// Vector store create operation
    pub const VECTOR_STORE_CREATE: &str = "vector_store_create";
    /// Vector store list operation
    pub const VECTOR_STORE_LIST: &str = "vector_store_list";
    /// Vector store retrieve operation
    pub const VECTOR_STORE_RETRIEVE: &str = "vector_store_retrieve";
    /// Vector store update operation
    pub const VECTOR_STORE_UPDATE: &str = "vector_store_update";
    /// Vector store delete operation
    pub const VECTOR_STORE_DELETE: &str = "vector_store_delete";
    /// Vector store file add operation
    pub const VECTOR_STORE_FILE_ADD: &str = "vector_store_file_add";
    /// Vector store file list operation
    pub const VECTOR_STORE_FILE_LIST: &str = "vector_store_file_list";
    /// Vector store file retrieve operation
    pub const VECTOR_STORE_FILE_RETRIEVE: &str = "vector_store_file_retrieve";
    /// Vector store file delete operation
    pub const VECTOR_STORE_FILE_DELETE: &str = "vector_store_file_delete";
    /// Vector store search operation
    pub const VECTOR_STORE_SEARCH: &str = "vector_store_search";

    /// Upload create operation
    pub const UPLOAD_CREATE: &str = "upload_create";

    /// Batch create operation
    pub const BATCH_CREATE: &str = "batch_create";
    /// Batch list operation
    pub const BATCH_LIST: &str = "batch_list";
    /// Batch retrieve operation
    pub const BATCH_RETRIEVE: &str = "batch_retrieve";
    /// Batch cancel operation
    pub const BATCH_CANCEL: &str = "batch_cancel";

    /// Fine-tuning create operation
    pub const FINE_TUNING_CREATE: &str = "fine_tuning_create";
    /// Fine-tuning list operation
    pub const FINE_TUNING_LIST: &str = "fine_tuning_list";
    /// Fine-tuning retrieve operation
    pub const FINE_TUNING_RETRIEVE: &str = "fine_tuning_retrieve";
    /// Fine-tuning cancel operation
    pub const FINE_TUNING_CANCEL: &str = "fine_tuning_cancel";
    /// Fine-tuning list events operation
    pub const FINE_TUNING_LIST_EVENTS: &str = "fine_tuning_list_events";
    /// Fine-tuning list checkpoints operation
    pub const FINE_TUNING_LIST_CHECKPOINTS: &str = "fine_tuning_list_checkpoints";

    /// Model list operation
    pub const MODEL_LIST: &str = "model_list";
    /// Model retrieve operation
    pub const MODEL_RETRIEVE: &str = "model_retrieve";
    /// Model delete operation
    pub const MODEL_DELETE: &str = "model_delete";

    /// Usage audio speeches operation
    pub const USAGE_AUDIO_SPEECHES: &str = "usage_audio_speeches";
    /// Usage audio transcriptions operation
    pub const USAGE_AUDIO_TRANSCRIPTIONS: &str = "usage_audio_transcriptions";
    /// Usage code interpreter operation
    pub const USAGE_CODE_INTERPRETER: &str = "usage_code_interpreter";
    /// Usage completions operation
    pub const USAGE_COMPLETIONS: &str = "usage_completions";
    /// Usage embeddings operation
    pub const USAGE_EMBEDDINGS: &str = "usage_embeddings";
    /// Usage images operation
    pub const USAGE_IMAGES: &str = "usage_images";
    /// Usage moderations operation
    pub const USAGE_MODERATIONS: &str = "usage_moderations";
    /// Usage vector stores operation
    pub const USAGE_VECTOR_STORES: &str = "usage_vector_stores";
    /// Usage costs operation
    pub const USAGE_COSTS: &str = "usage_costs";
}

/// System/provider name constants
pub mod systems {
    /// `OpenAI` as the `GenAI` provider
    pub const OPENAI: &str = "openai";
}

/// Output type constants
pub mod output_types {
    /// Plain text output
    pub const TEXT: &str = "text";

    /// JSON structured output
    pub const JSON: &str = "json";

    /// Image output
    pub const IMAGE: &str = "image";

    /// Speech/audio output
    pub const SPEECH: &str = "speech";
}

/// Service tier constants
pub mod service_tiers {
    /// Utilize scale tier credits
    pub const AUTO: &str = "auto";

    /// Use default scale tier
    pub const DEFAULT: &str = "default";
}
