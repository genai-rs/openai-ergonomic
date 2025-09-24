//! Comprehensive embeddings example demonstrating vector embeddings with testing patterns.
//!
//! This example showcases the `OpenAI` embeddings API, including:
//! - Basic embedding generation with different models
//! - Batch processing of multiple texts
//! - Dimension reduction capabilities
//! - Similarity comparisons between embeddings
//! - Testing patterns for embeddings
//! - Comprehensive error handling and documentation
//!
//! ## Features Demonstrated
//!
//! - **Multiple Models**: text-embedding-3-small, text-embedding-3-large, ada-002
//! - **Batch Processing**: Process multiple texts efficiently in a single request
//! - **Dimension Control**: Reduce dimensions for optimized storage and performance
//! - **Similarity Metrics**: Cosine similarity calculations between vectors
//! - **Error Handling**: Robust error handling for various failure scenarios
//! - **Testing Patterns**: Mock-friendly design for unit testing
//!
//! ## Prerequisites
//!
//! Set your `OpenAI` API key:
//! ```bash
//! export OPENAI_API_KEY="your-key-here"
//! ```
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example embeddings
//! ```
//!
//! Note: This example uses simulated responses to keep the example runnable without
//! real `OpenAI` credentials. Replace the simulated sections with
//! `client.embeddings().create(...)` calls to interact with the live API.

#![allow(clippy::uninlined_format_args)]
#![allow(clippy::no_effect_underscore_binding)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::unused_async)]
#![allow(dead_code)]

use openai_ergonomic::Client;
use std::collections::HashMap;

/// Embedding models supported by `OpenAI`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingModel {
    /// text-embedding-3-small - Latest small embedding model
    TextEmbedding3Small,
    /// text-embedding-3-large - Latest large embedding model
    TextEmbedding3Large,
    /// text-embedding-ada-002 - Legacy ada model
    Ada002,
}

impl EmbeddingModel {
    /// Get the model name string
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::TextEmbedding3Small => "text-embedding-3-small",
            Self::TextEmbedding3Large => "text-embedding-3-large",
            Self::Ada002 => "text-embedding-ada-002",
        }
    }

    /// Get default dimensions for the model
    pub const fn default_dimensions(&self) -> usize {
        match self {
            Self::TextEmbedding3Large => 3072,
            Self::TextEmbedding3Small | Self::Ada002 => 1536,
        }
    }

    /// Check if the model supports dimension reduction
    pub const fn supports_dimensions(&self) -> bool {
        matches!(self, Self::TextEmbedding3Small | Self::TextEmbedding3Large)
    }
}

/// Represents an embedding vector with metadata
#[derive(Debug, Clone)]
pub struct Embedding {
    /// The vector values
    pub vector: Vec<f32>,
    /// The input text that generated this embedding
    pub text: String,
    /// The model used to generate this embedding
    pub model: EmbeddingModel,
    /// Number of tokens in the input text
    pub token_count: Option<usize>,
}

impl Embedding {
    /// Create a new embedding
    pub const fn new(vector: Vec<f32>, text: String, model: EmbeddingModel) -> Self {
        Self {
            vector,
            text,
            model,
            token_count: None,
        }
    }

    /// Get the dimensionality of this embedding
    pub fn dimensions(&self) -> usize {
        self.vector.len()
    }

    /// Calculate cosine similarity with another embedding
    pub fn cosine_similarity(&self, other: &Self) -> Result<f32, EmbeddingError> {
        if self.vector.len() != other.vector.len() {
            return Err(EmbeddingError::DimensionMismatch {
                expected: self.vector.len(),
                actual: other.vector.len(),
            });
        }

        let dot_product: f32 = self
            .vector
            .iter()
            .zip(&other.vector)
            .map(|(a, b)| a * b)
            .sum();

        let norm_a: f32 = self.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.vector.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return Err(EmbeddingError::ZeroVector);
        }

        Ok(dot_product / (norm_a * norm_b))
    }

    /// Calculate Euclidean distance with another embedding
    pub fn euclidean_distance(&self, other: &Self) -> Result<f32, EmbeddingError> {
        if self.vector.len() != other.vector.len() {
            return Err(EmbeddingError::DimensionMismatch {
                expected: self.vector.len(),
                actual: other.vector.len(),
            });
        }

        let distance: f32 = self
            .vector
            .iter()
            .zip(&other.vector)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt();

        Ok(distance)
    }
}

/// Custom error types for embedding operations
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    /// Dimension mismatch between vectors
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected dimension count
        expected: usize,
        /// Actual dimension count
        actual: usize,
    },

    /// Cannot calculate similarity with zero vector
    #[error("Cannot calculate similarity with zero vector")]
    ZeroVector,

    /// Model does not support dimension reduction
    #[error("Model {model} does not support dimension reduction")]
    DimensionReductionNotSupported {
        /// Model name that doesn't support dimension reduction
        model: String,
    },

    /// Invalid dimensions specified
    #[error("Invalid dimensions: {dimensions} (must be between 1 and {max})")]
    InvalidDimensions {
        /// Requested dimensions
        dimensions: usize,
        /// Maximum allowed dimensions
        max: usize,
    },

    /// Batch processing failed
    #[error("Batch processing failed: {message}")]
    BatchProcessingFailed {
        /// Error message
        message: String,
    },
}

/// Embedding request configuration
#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    /// Texts to embed
    pub inputs: Vec<String>,
    /// Model to use
    pub model: EmbeddingModel,
    /// Optional dimension reduction
    pub dimensions: Option<usize>,
    /// User identifier for abuse monitoring
    pub user: Option<String>,
}

impl EmbeddingRequest {
    /// Create a new embedding request for a single text
    pub fn new(text: impl Into<String>, model: EmbeddingModel) -> Self {
        Self {
            inputs: vec![text.into()],
            model,
            dimensions: None,
            user: None,
        }
    }

    /// Create a new embedding request for multiple texts
    pub const fn batch(texts: Vec<String>, model: EmbeddingModel) -> Self {
        Self {
            inputs: texts,
            model,
            dimensions: None,
            user: None,
        }
    }

    /// Set dimension reduction (only for supported models)
    pub fn with_dimensions(mut self, dimensions: usize) -> Result<Self, EmbeddingError> {
        if !self.model.supports_dimensions() {
            return Err(EmbeddingError::DimensionReductionNotSupported {
                model: self.model.as_str().to_string(),
            });
        }

        let max_dims = self.model.default_dimensions();
        if dimensions == 0 || dimensions > max_dims {
            return Err(EmbeddingError::InvalidDimensions {
                dimensions,
                max: max_dims,
            });
        }

        self.dimensions = Some(dimensions);
        Ok(self)
    }

    /// Set user identifier for abuse monitoring
    #[must_use]
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }
}

/// Embedding response containing results and metadata
#[derive(Debug, Clone)]
pub struct EmbeddingResponse {
    /// Generated embeddings
    pub embeddings: Vec<Embedding>,
    /// Model used
    pub model: EmbeddingModel,
    /// Token usage information
    pub usage: EmbeddingUsage,
}

/// Token usage information for embedding requests
#[derive(Debug, Clone)]
pub struct EmbeddingUsage {
    /// Number of tokens in the input
    pub prompt_tokens: usize,
    /// Total tokens processed
    pub total_tokens: usize,
}

/// Similarity search result
#[derive(Debug, Clone)]
pub struct SimilarityResult {
    /// The embedding that was matched
    pub embedding: Embedding,
    /// Similarity score (higher is more similar)
    pub score: f32,
    /// Index in the original collection
    pub index: usize,
}

/// Collection of embeddings for similarity search
#[derive(Debug, Clone)]
pub struct EmbeddingCollection {
    embeddings: Vec<Embedding>,
    metadata: HashMap<usize, serde_json::Value>,
}

impl EmbeddingCollection {
    /// Create a new embedding collection
    pub fn new() -> Self {
        Self {
            embeddings: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add an embedding to the collection
    pub fn add(&mut self, embedding: Embedding) -> usize {
        let index = self.embeddings.len();
        self.embeddings.push(embedding);
        index
    }

    /// Add an embedding with metadata
    pub fn add_with_metadata(
        &mut self,
        embedding: Embedding,
        metadata: serde_json::Value,
    ) -> usize {
        let index = self.add(embedding);
        self.metadata.insert(index, metadata);
        index
    }

    /// Find the most similar embeddings to a query
    pub fn find_similar(
        &self,
        query: &Embedding,
        top_k: usize,
    ) -> Result<Vec<SimilarityResult>, EmbeddingError> {
        let mut results = Vec::new();

        for (index, embedding) in self.embeddings.iter().enumerate() {
            let score = query.cosine_similarity(embedding)?;
            results.push(SimilarityResult {
                embedding: embedding.clone(),
                score,
                index,
            });
        }

        // Sort by similarity score (descending)
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top k results
        results.truncate(top_k);
        Ok(results)
    }

    /// Get metadata for an embedding by index
    pub fn get_metadata(&self, index: usize) -> Option<&serde_json::Value> {
        self.metadata.get(&index)
    }

    /// Get the number of embeddings in the collection
    pub fn len(&self) -> usize {
        self.embeddings.len()
    }

    /// Check if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.embeddings.is_empty()
    }
}

impl Default for EmbeddingCollection {
    fn default() -> Self {
        Self::new()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 OpenAI Ergonomic - Comprehensive Embeddings Example\n");

    // Initialize client from environment variables
    let client = match Client::from_env() {
        Ok(client) => {
            println!("✅ Client initialized successfully");
            client
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize client: {e}");
            eprintln!("💡 Make sure OPENAI_API_KEY is set in your environment");
            return Err(e.into());
        }
    };

    // Example 1: Basic Embedding Generation
    println!("\n📝 Example 1: Basic Embedding Generation");
    println!("=========================================");

    match basic_embedding_example(&client).await {
        Ok(()) => println!("✅ Basic embedding example completed"),
        Err(e) => {
            eprintln!("❌ Basic embedding example failed: {e}");
            handle_embedding_error(e.as_ref());
        }
    }

    // Example 2: Model Comparison
    println!("\n🔬 Example 2: Model Comparison");
    println!("===============================");

    match model_comparison_example(&client).await {
        Ok(()) => println!("✅ Model comparison example completed"),
        Err(e) => {
            eprintln!("❌ Model comparison example failed: {e}");
            handle_embedding_error(e.as_ref());
        }
    }

    // Example 3: Batch Processing
    println!("\n📦 Example 3: Batch Processing");
    println!("===============================");

    match batch_processing_example(&client).await {
        Ok(()) => println!("✅ Batch processing example completed"),
        Err(e) => {
            eprintln!("❌ Batch processing example failed: {e}");
            handle_embedding_error(e.as_ref());
        }
    }

    // Example 4: Dimension Reduction
    println!("\n📏 Example 4: Dimension Reduction");
    println!("==================================");

    match dimension_reduction_example(&client).await {
        Ok(()) => println!("✅ Dimension reduction example completed"),
        Err(e) => {
            eprintln!("❌ Dimension reduction example failed: {e}");
            handle_embedding_error(e.as_ref());
        }
    }

    // Example 5: Similarity Search
    println!("\n🔍 Example 5: Similarity Search");
    println!("================================");

    match similarity_search_example(&client).await {
        Ok(()) => println!("✅ Similarity search example completed"),
        Err(e) => {
            eprintln!("❌ Similarity search example failed: {e}");
            handle_embedding_error(e.as_ref());
        }
    }

    // Example 6: Testing Patterns
    println!("\n🧪 Example 6: Testing Patterns");
    println!("===============================");

    match testing_patterns_example().await {
        Ok(()) => println!("✅ Testing patterns example completed"),
        Err(e) => {
            eprintln!("❌ Testing patterns example failed: {e}");
            handle_embedding_error(e.as_ref());
        }
    }

    println!("\n🎉 All examples completed! Check the console output above for results.");
    println!("\nNote: This example simulates API responses. Swap the simulated sections with");
    println!("real `client.embeddings().create(...)` calls when you're ready to hit the API.");

    Ok(())
}

/// Example 1: Basic embedding generation with a single text
async fn basic_embedding_example(_client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating embeddings for a simple text...");

    // This would be the intended API usage:
    // let builder = client
    //     .embeddings()
    //     .text("text-embedding-3-small", "The quick brown fox jumps over the lazy dog");
    // let response = client.embeddings().create(builder).await?;

    // For now, we'll simulate the response
    let text = "The quick brown fox jumps over the lazy dog";
    let model = EmbeddingModel::TextEmbedding3Small;

    println!("📝 Input text: \"{}\"", text);
    println!("🤖 Model: {}", model.as_str());
    println!("📊 Expected dimensions: {}", model.default_dimensions());

    // Simulate embedding generation
    let simulated_embedding = simulate_embedding(text, model);

    println!(
        "✅ Generated embedding with {} dimensions",
        simulated_embedding.dimensions()
    );
    println!(
        "📈 First 5 values: {:?}",
        &simulated_embedding.vector[..5.min(simulated_embedding.vector.len())]
    );

    if let Some(token_count) = simulated_embedding.token_count {
        println!("🎯 Token count: {}", token_count);
    }

    Ok(())
}

/// Example 2: Compare different embedding models
async fn model_comparison_example(_client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("Comparing embeddings across different models...");

    let text = "Artificial intelligence is transforming the world";
    let models = [
        EmbeddingModel::TextEmbedding3Small,
        EmbeddingModel::TextEmbedding3Large,
        EmbeddingModel::Ada002,
    ];

    println!("📝 Input text: \"{}\"", text);
    println!();

    for model in models {
        println!("Testing model: {}", model.as_str());

        // Simulate embedding generation for each model
        let embedding = simulate_embedding(text, model);

        println!("  📊 Dimensions: {}", embedding.dimensions());
        println!(
            "  🎯 Supports dimension reduction: {}",
            model.supports_dimensions()
        );
        println!(
            "  📈 Vector norm: {:.6}",
            calculate_vector_norm(&embedding.vector)
        );
        println!();
    }

    println!("💡 Different models produce embeddings with different characteristics:");
    println!("   - text-embedding-3-small: Balanced performance and cost");
    println!("   - text-embedding-3-large: Higher quality, more expensive");
    println!("   - ada-002: Legacy model, still widely used");

    Ok(())
}

/// Example 3: Batch processing multiple texts
async fn batch_processing_example(_client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("Processing multiple texts in batch...");

    let texts = vec![
        "The weather is sunny today".to_string(),
        "I love reading science fiction books".to_string(),
        "Machine learning algorithms are fascinating".to_string(),
        "Pizza is my favorite food".to_string(),
        "The ocean is vast and mysterious".to_string(),
    ];

    println!("📦 Processing {} texts in batch:", texts.len());
    for (i, text) in texts.iter().enumerate() {
        println!("  {}. \"{}\"", i + 1, text);
    }

    // This would be the intended API usage:
    // let builder = client
    //     .embeddings()
    //     .builder("text-embedding-3-small")
    //     .input_texts(texts.clone());
    // let response = client.embeddings().create(builder).await?;

    // Simulate batch processing
    let mut embeddings = Vec::new();
    let mut total_tokens = 0;

    for text in &texts {
        let embedding = simulate_embedding(text, EmbeddingModel::TextEmbedding3Small);
        if let Some(tokens) = embedding.token_count {
            total_tokens += tokens;
        }
        embeddings.push(embedding);
    }

    println!("\n✅ Generated {} embeddings", embeddings.len());
    println!("🎯 Total tokens used: {}", total_tokens);
    #[allow(clippy::cast_precision_loss)]
    {
        println!(
            "📊 Average tokens per text: {:.1}",
            total_tokens as f32 / texts.len() as f32
        );
    }

    // Show some statistics
    #[allow(clippy::cast_precision_loss)]
    let avg_norm: f32 = embeddings
        .iter()
        .map(|e| calculate_vector_norm(&e.vector))
        .sum::<f32>()
        / embeddings.len() as f32;

    println!("📈 Average vector norm: {:.6}", avg_norm);

    println!("\n💡 Batch processing is more efficient for multiple texts:");
    println!("   - Reduced API calls and latency");
    println!("   - Better throughput for large datasets");
    println!("   - Cost-effective for bulk operations");

    Ok(())
}

/// Example 4: Dimension reduction for optimized storage
async fn dimension_reduction_example(_client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("Demonstrating dimension reduction capabilities...");

    let text = "Vector databases enable semantic search at scale";
    let model = EmbeddingModel::TextEmbedding3Small;
    let original_dims = model.default_dimensions();
    let reduced_dims = [512, 256, 128];

    println!("📝 Input text: \"{}\"", text);
    println!(
        "🤖 Model: {} (default: {} dimensions)",
        model.as_str(),
        original_dims
    );

    // Generate original embedding
    let original_embedding = simulate_embedding(text, model);
    println!(
        "\n📊 Original embedding: {} dimensions",
        original_embedding.dimensions()
    );

    // Test different dimension reductions
    for &dims in &reduced_dims {
        // This would be the intended API usage:
        // let builder = client
        //     .embeddings()
        //     .text(model.as_str(), text)
        //     .dimensions(dims as i32);
        // let response = client.embeddings().create(builder).await?;

        // Simulate dimension reduction
        let reduced_embedding = simulate_reduced_embedding(text, model, dims).unwrap();

        println!("📏 Reduced to {} dimensions:", dims);

        // Calculate similarity between original and reduced
        if let Ok(similarity) = original_embedding.cosine_similarity(&reduced_embedding) {
            println!("   🔗 Similarity to original: {:.4}", similarity);
        }

        #[allow(clippy::cast_precision_loss)]
        let compression_ratio = dims as f32 / original_dims as f32;
        println!("   📦 Compression ratio: {:.1}%", compression_ratio * 100.0);

        let storage_savings = (1.0 - compression_ratio) * 100.0;
        println!("   💾 Storage savings: {:.1}%", storage_savings);
    }

    println!("\n💡 Dimension reduction trade-offs:");
    println!("   ✅ Pros: Reduced storage, faster search, lower memory usage");
    println!("   ⚠️  Cons: Some semantic information loss");
    println!("   🎯 Best practice: Test different dimensions for your use case");

    Ok(())
}

/// Example 5: Similarity search and comparison
async fn similarity_search_example(_client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("Demonstrating similarity search and comparison...");

    // Create a collection of text documents
    let documents = vec![
        "The cat sat on the mat",
        "A feline rested on the rug",
        "Dogs are loyal companions",
        "Canines make great pets",
        "The weather is sunny today",
        "It's a beautiful clear day",
        "Machine learning is fascinating",
        "AI algorithms are powerful tools",
    ];

    let model = EmbeddingModel::TextEmbedding3Small;
    println!("📚 Document collection ({} items):", documents.len());
    for (i, doc) in documents.iter().enumerate() {
        println!("  {}. \"{}\"", i + 1, doc);
    }

    // Create embeddings for all documents
    let mut collection = EmbeddingCollection::new();
    for doc in &documents {
        let embedding = simulate_embedding(doc, model);
        let metadata = serde_json::json!({
            "text": doc,
            "length": doc.len(),
            "word_count": doc.split_whitespace().count()
        });
        collection.add_with_metadata(embedding, metadata);
    }

    println!(
        "\n✅ Created embedding collection with {} items",
        collection.len()
    );

    // Query examples
    let queries = vec![
        "A cat sitting down",
        "Dog pets",
        "Nice weather",
        "Artificial intelligence",
    ];

    for query in queries {
        println!("\n🔍 Query: \"{}\"", query);

        let query_embedding = simulate_embedding(query, model);
        let results = collection.find_similar(&query_embedding, 3)?;

        println!("   Top 3 similar documents:");
        for (rank, result) in results.iter().enumerate() {
            println!(
                "     {}. \"{}\" (similarity: {:.4})",
                rank + 1,
                result.embedding.text,
                result.score
            );

            if let Some(metadata) = collection.get_metadata(result.index) {
                if let Some(word_count) = metadata["word_count"].as_u64() {
                    println!("        Words: {}", word_count);
                }
            }
        }
    }

    println!("\n💡 Similarity search applications:");
    println!("   🔍 Semantic search engines");
    println!("   📝 Document retrieval systems");
    println!("   🤖 Recommendation engines");
    println!("   🎯 Content deduplication");

    Ok(())
}

/// Example 6: Testing patterns for embeddings
async fn testing_patterns_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Demonstrating testing patterns for embeddings...");

    // Test 1: Embedding properties
    println!("\n🧪 Test 1: Embedding Properties");
    let text = "Test embedding generation";
    let model = EmbeddingModel::TextEmbedding3Small;
    let embedding = simulate_embedding(text, model);

    assert_eq!(embedding.dimensions(), model.default_dimensions());
    assert_eq!(embedding.text, text);
    assert_eq!(embedding.model, model);
    println!("   ✅ Embedding properties test passed");

    // Test 2: Similarity calculations
    println!("\n🧪 Test 2: Similarity Calculations");
    let text1 = "Hello world";
    let text2 = "Hello world"; // Identical
    let text3 = "Goodbye world"; // Different

    let embed1 = simulate_embedding(text1, model);
    let embed2 = simulate_embedding(text2, model);
    let embed3 = simulate_embedding(text3, model);

    let identical_similarity = embed1.cosine_similarity(&embed2)?;
    let different_similarity = embed1.cosine_similarity(&embed3)?;

    assert!(
        identical_similarity > 0.99,
        "Identical texts should have high similarity"
    );
    assert!(
        different_similarity < identical_similarity,
        "Different texts should have lower similarity"
    );
    println!("   ✅ Similarity calculation test passed");
    println!(
        "      Identical texts similarity: {:.4}",
        identical_similarity
    );
    println!(
        "      Different texts similarity: {:.4}",
        different_similarity
    );

    // Test 3: Dimension mismatch error
    println!("\n🧪 Test 3: Error Handling");
    let small_embed =
        simulate_reduced_embedding("test", EmbeddingModel::TextEmbedding3Small, 256).unwrap();
    let large_embed = simulate_embedding("test", EmbeddingModel::TextEmbedding3Large);

    match small_embed.cosine_similarity(&large_embed) {
        Err(EmbeddingError::DimensionMismatch { expected, actual }) => {
            println!("   ✅ Dimension mismatch error handled correctly");
            println!("      Expected: {}, Actual: {}", expected, actual);
        }
        Ok(_) => panic!("Should have failed with dimension mismatch"),
        Err(e) => panic!("Unexpected error: {}", e),
    }

    // Test 4: Collection operations
    println!("\n🧪 Test 4: Collection Operations");
    let mut collection = EmbeddingCollection::new();
    assert!(collection.is_empty());

    let test_embedding = simulate_embedding("test", model);
    let index = collection.add(test_embedding);
    assert_eq!(index, 0);
    assert_eq!(collection.len(), 1);
    assert!(!collection.is_empty());

    println!("   ✅ Collection operations test passed");

    // Test 5: Model capabilities
    println!("\n🧪 Test 5: Model Capabilities");
    assert!(EmbeddingModel::TextEmbedding3Small.supports_dimensions());
    assert!(EmbeddingModel::TextEmbedding3Large.supports_dimensions());
    assert!(!EmbeddingModel::Ada002.supports_dimensions());

    println!("   ✅ Model capabilities test passed");

    println!("\n💡 Testing best practices:");
    println!("   🎯 Test embedding properties and dimensions");
    println!("   🔍 Validate similarity calculations");
    println!("   🚫 Test error conditions and edge cases");
    println!("   📊 Test with known similar/dissimilar text pairs");
    println!("   🤖 Use deterministic test data for reproducible results");

    Ok(())
}

/// Simulate embedding generation (for demonstration purposes)
fn simulate_embedding(text: &str, model: EmbeddingModel) -> Embedding {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let dimensions = model.default_dimensions();

    // Create a deterministic "embedding" based on text hash
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    model.as_str().hash(&mut hasher);
    let seed = hasher.finish();

    let mut rng = XorShift64Star::new(seed);
    let mut vector = Vec::with_capacity(dimensions);

    // Generate random-ish values that sum to create a unit vector
    for _ in 0..dimensions {
        vector.push(rng.next_f32() - 0.5);
    }

    // Normalize to unit vector
    let norm = calculate_vector_norm(&vector);
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }

    let token_count = text.split_whitespace().count().max(1);

    let mut embedding = Embedding::new(vector, text.to_string(), model);
    embedding.token_count = Some(token_count);

    embedding
}

/// Simulate embedding with reduced dimensions
fn simulate_reduced_embedding(
    text: &str,
    model: EmbeddingModel,
    dimensions: usize,
) -> Result<Embedding, Box<dyn std::error::Error>> {
    if !model.supports_dimensions() {
        return Err(EmbeddingError::DimensionReductionNotSupported {
            model: model.as_str().to_string(),
        }
        .into());
    }

    let mut original = simulate_embedding(text, model);

    // Truncate to desired dimensions and renormalize
    original.vector.truncate(dimensions);
    let norm = calculate_vector_norm(&original.vector);
    if norm > 0.0 {
        for value in &mut original.vector {
            *value /= norm;
        }
    }

    Ok(original)
}

/// Calculate the Euclidean norm of a vector
fn calculate_vector_norm(vector: &[f32]) -> f32 {
    vector.iter().map(|x| x * x).sum::<f32>().sqrt()
}

/// Simple PRNG for deterministic "random" embeddings
struct XorShift64Star {
    state: u64,
}

impl XorShift64Star {
    const fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    #[allow(clippy::cast_precision_loss)]
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Handle embedding-specific errors with helpful context
fn handle_embedding_error(error: &dyn std::error::Error) {
    // This is simplified - in a real implementation you'd match on specific error types
    eprintln!("🚫 Embedding Error: {}", error);

    if let Some(source) = error.source() {
        eprintln!("   Caused by: {}", source);
    }

    // Provide context-specific guidance
    eprintln!("💡 Troubleshooting tips:");
    eprintln!("   - Check your API key and network connection");
    eprintln!("   - Verify text input is not empty");
    eprintln!("   - Ensure model supports requested features");
    eprintln!("   - Check dimension parameters are valid");
}
