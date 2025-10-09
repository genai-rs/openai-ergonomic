#![allow(clippy::uninlined_format_args)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::useless_vec)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::single_char_pattern)]
#![allow(clippy::inefficient_to_string)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::redundant_closure_for_method_calls)]
//! Vector Stores API Example
//!
//! This example demonstrates how to use the OpenAI Vector Stores API for semantic search,
//! document management, and knowledge base operations. Vector stores enable efficient
//! similarity-based document retrieval for RAG applications and knowledge management systems.
//!
//! ## Features Demonstrated
//!
//! - Creating and configuring vector stores
//! - File management and batch operations
//! - Semantic search and similarity queries
//! - Vector store lifecycle management
//! - Advanced search filtering and ranking
//! - Integration with assistants and RAG workflows
//!
//! ## Prerequisites
//!
//! Set your OpenAI API key:
//! ```bash
//! export OPENAI_API_KEY="your-key-here"
//! ```
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example vector_stores
//! ```
//!
//! ## Note on Implementation Status
//!
//! This example demonstrates the intended API design for vector store operations.
//! The code shows how the ergonomic builders provide a streamlined interface for
//! complex vector operations and document management.

use openai_ergonomic::{
    builders::vector_stores::{
        add_file_to_vector_store, search_vector_store, search_vector_store_with_limit,
        simple_vector_store, temporary_vector_store, vector_store_with_files, VectorStoreBuilder,
        VectorStoreSearchBuilder,
    },
    Client, Error,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗄️ OpenAI Ergonomic - Vector Stores Example\n");

    // Initialize client from environment variables
    let _client = match Client::from_env() {
        Ok(client_builder) => {
            println!("✅ Client initialized successfully");
            client_builder.build()
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize client: {e}");
            eprintln!("💡 Make sure OPENAI_API_KEY is set in your environment");
            return Err(e.into());
        }
    };

    // Demonstrate different vector store use cases
    run_basic_vector_store_example()?;
    run_document_management_example()?;
    run_semantic_search_example()?;
    run_enterprise_knowledge_base_example()?;
    run_vector_store_lifecycle_example()?;
    run_advanced_search_patterns_example()?;

    println!("\n🎉 Vector Stores examples completed successfully!");
    Ok(())
}

/// Example 1: Basic Vector Store Operations
fn run_basic_vector_store_example() -> Result<(), Error> {
    println!("🔧 Example 1: Basic Vector Store Operations");
    println!("{}", "=".repeat(60));

    // Create a simple vector store
    let basic_store = simple_vector_store("Getting Started Vector Store")
        .metadata("purpose", "tutorial")
        .metadata("created_by", "openai_ergonomic_example");

    println!("📁 Created basic vector store:");
    println!("   Name: {}", basic_store.name_ref().unwrap());
    println!(
        "   Purpose: {}",
        basic_store.metadata_ref().get("purpose").unwrap()
    );
    println!("   Files: {}", basic_store.file_count());

    // Add files to the vector store
    let store_with_files = basic_store
        .add_file("file-welcome-doc-001")
        .add_file("file-getting-started-002")
        .add_file("file-basic-examples-003");

    println!("\n📄 Added files to vector store:");
    for (i, file_id) in store_with_files.file_ids_ref().iter().enumerate() {
        println!("   {}. {}", i + 1, file_id);
    }
    println!("   Total files: {}", store_with_files.file_count());

    // Demonstrate vector store properties
    println!("\n📊 Vector Store Properties:");
    println!("   Has files: {}", store_with_files.has_files());
    println!(
        "   Metadata entries: {}",
        store_with_files.metadata_ref().len()
    );
    println!(
        "   Expires: {}",
        if store_with_files.expires_after_ref().is_some() {
            "Yes"
        } else {
            "No"
        }
    );

    println!("\n🔄 Basic Operations:");
    println!("   ✅ Create vector store");
    println!("   ✅ Add metadata");
    println!("   ✅ Add files");
    println!("   ✅ Query properties");
    println!("   🔄 Ready for search operations");

    Ok(())
}

/// Example 2: Document Management and Batch Operations
fn run_document_management_example() -> Result<(), Error> {
    println!("\n📚 Example 2: Document Management and Batch Operations");
    println!("{}", "=".repeat(60));

    // Simulate a large document collection
    let document_collection = vec![
        "file-product-docs-001",
        "file-product-docs-002",
        "file-api-reference-003",
        "file-user-guide-004",
        "file-troubleshooting-005",
        "file-changelog-006",
        "file-best-practices-007",
        "file-integration-guide-008",
    ];

    // Create vector store with batch file addition
    let doc_store = vector_store_with_files(
        "Product Documentation Store",
        document_collection.iter().map(|s| s.to_string()).collect(),
    )
    .metadata("category", "documentation")
    .metadata("product", "api_platform")
    .metadata("version", "v2.1")
    .expires_after_days(180); // 6 months retention

    println!("📚 Created documentation vector store:");
    println!("   Name: {}", doc_store.name_ref().unwrap());
    println!("   Documents: {} files", doc_store.file_count());
    println!(
        "   Category: {}",
        doc_store.metadata_ref().get("category").unwrap()
    );
    println!("   Retention: 180 days");

    // Demonstrate individual file operations
    let individual_file_op = add_file_to_vector_store("doc-store-123", "file-new-feature-009");

    println!("\n📄 Individual File Operations:");
    println!("   Adding file: {}", individual_file_op.file_id());
    println!("   To store: {}", individual_file_op.vector_store_id());

    // Simulate file organization strategies
    println!("\n🗂️ Document Organization Strategies:");

    let categorized_stores = vec![
        (
            "API Documentation",
            vec!["file-api-ref", "file-endpoints", "file-auth"],
        ),
        (
            "User Guides",
            vec!["file-quickstart", "file-tutorials", "file-howtos"],
        ),
        (
            "Technical Specs",
            vec!["file-architecture", "file-protocols", "file-security"],
        ),
        (
            "Release Notes",
            vec!["file-changelog", "file-migration", "file-breaking-changes"],
        ),
    ];

    for (category, files) in &categorized_stores {
        let category_store = vector_store_with_files(
            format!("{} Vector Store", category),
            files.iter().map(|s| s.to_string()).collect(),
        )
        .metadata("category", category.to_lowercase().replace(" ", "_"))
        .metadata("auto_managed", "true");

        println!("   📁 {}: {} files", category, category_store.file_count());
    }

    println!("\n🔄 Document Management Workflow:");
    println!("   1. 📥 Batch upload documents by category");
    println!("   2. 🏷️ Apply consistent metadata tagging");
    println!("   3. ⏰ Set appropriate retention policies");
    println!("   4. 🔄 Enable automatic organization");
    println!("   5. 📊 Monitor storage usage and performance");

    Ok(())
}

/// Example 3: Semantic Search and Similarity Queries
fn run_semantic_search_example() -> Result<(), Error> {
    println!("\n🔍 Example 3: Semantic Search and Similarity Queries");
    println!("{}", "=".repeat(60));

    // Create a search-optimized vector store
    let search_store = simple_vector_store("Semantic Search Demo Store")
        .add_file("file-ml-concepts-001")
        .add_file("file-nlp-techniques-002")
        .add_file("file-deep-learning-003")
        .add_file("file-computer-vision-004")
        .add_file("file-ai-ethics-005")
        .metadata("domain", "machine_learning")
        .metadata("search_optimized", "true");

    println!("🔍 Created search-optimized vector store:");
    println!("   Name: {}", search_store.name_ref().unwrap());
    println!("   Domain: Machine Learning");
    println!("   Documents: {} files", search_store.file_count());

    // Demonstrate various search patterns
    println!("\n🎯 Search Query Examples:");

    // Basic semantic search
    let basic_search = search_vector_store("search-store-123", "neural network architectures");
    println!("   1. Basic Search:");
    println!("      Query: '{}'", basic_search.query());
    println!("      Store: {}", basic_search.vector_store_id());

    // Limited result search
    let limited_search = search_vector_store_with_limit(
        "search-store-123",
        "natural language processing techniques",
        5,
    );
    println!("   2. Limited Results:");
    println!("      Query: '{}'", limited_search.query());
    println!(
        "      Limit: {} results",
        limited_search.limit_ref().unwrap()
    );

    // Advanced filtered search
    let filtered_search =
        search_vector_store_with_limit("search-store-123", "computer vision applications", 10)
            .filter("category", "practical_applications")
            .filter("difficulty", "intermediate");

    println!("   3. Filtered Search:");
    println!("      Query: '{}'", filtered_search.query());
    println!(
        "      Filters: {} applied",
        filtered_search.filter_ref().len()
    );
    for (key, value) in filtered_search.filter_ref() {
        println!("         {}={}", key, value);
    }

    // Demonstrate search result processing
    println!("\n📊 Search Result Processing:");
    println!("   🎯 Semantic similarity ranking");
    println!("   📄 Document excerpt extraction");
    println!("   🔢 Relevance score calculation");
    println!("   📍 Source location identification");
    println!("   🔗 Related content suggestions");

    // Show different query types
    println!("\n🧠 Query Type Examples:");
    let query_examples = vec![
        (
            "Conceptual",
            "What is machine learning?",
            "Broad conceptual understanding",
        ),
        (
            "Technical",
            "How to implement backpropagation?",
            "Specific technical implementation",
        ),
        (
            "Comparative",
            "LSTM vs Transformer architectures",
            "Comparative analysis",
        ),
        (
            "Problem-solving",
            "Overfitting in neural networks",
            "Problem identification and solutions",
        ),
        (
            "Application",
            "Computer vision in healthcare",
            "Domain-specific applications",
        ),
    ];

    for (query_type, query, description) in query_examples {
        println!("   🎯 {}: '{}'", query_type, query);
        println!("      Purpose: {}", description);
    }

    Ok(())
}

/// Example 4: Enterprise Knowledge Base
fn run_enterprise_knowledge_base_example() -> Result<(), Error> {
    println!("\n🏢 Example 4: Enterprise Knowledge Base");
    println!("{}", "=".repeat(60));

    // Create enterprise-scale vector stores
    let enterprise_stores = create_enterprise_knowledge_base()?;

    println!("🏢 Enterprise Knowledge Base Architecture:");
    for (department, store) in enterprise_stores {
        println!("   📂 {}", department);
        println!("      Files: {} documents", store.file_count());
        println!(
            "      Retention: {} days",
            store
                .expires_after_ref()
                .map_or("permanent".to_string(), |exp| exp.days.to_string())
                .as_str()
        );

        // Show metadata structure
        for (key, value) in store.metadata_ref() {
            println!("      {}: {}", key, value);
        }
        println!();
    }

    // Demonstrate cross-departmental search
    println!("🔍 Cross-Departmental Search Examples:");

    let cross_searches = vec![
        (
            "Security Compliance",
            "GDPR data handling procedures",
            vec!["legal", "engineering", "hr"],
        ),
        (
            "Product Launch",
            "Q4 release planning and coordination",
            vec!["product", "engineering", "marketing"],
        ),
        (
            "Budget Planning",
            "Annual technology investment strategy",
            vec!["finance", "engineering", "executive"],
        ),
        (
            "Process Improvement",
            "Remote work productivity guidelines",
            vec!["hr", "operations", "it"],
        ),
    ];

    for (topic, query, departments) in cross_searches {
        println!("   📊 {}: '{}'", topic, query);
        println!("      Search scope: {}", departments.join(", "));
    }

    println!("\n🛡️ Enterprise Features:");
    println!("   🔐 Role-based access control");
    println!("   📊 Usage analytics and monitoring");
    println!("   🔄 Automated content lifecycle management");
    println!("   📈 Search performance optimization");
    println!("   💾 Backup and disaster recovery");
    println!("   🏷️ Compliance and audit trails");

    Ok(())
}

/// Example 5: Vector Store Lifecycle Management
fn run_vector_store_lifecycle_example() -> Result<(), Error> {
    println!("\n♻️ Example 5: Vector Store Lifecycle Management");
    println!("{}", "=".repeat(60));

    // Demonstrate different lifecycle patterns
    println!("⏰ Vector Store Lifecycle Patterns:");

    // Temporary stores for sessions
    let session_store = temporary_vector_store("User Session Store", 1)
        .add_file("file-session-context-001")
        .metadata("session_id", "sess_12345")
        .metadata("user_id", "user_67890");

    println!("   🕐 Session-based (1 day):");
    println!("      Purpose: Temporary user context");
    println!("      Files: {}", session_store.file_count());
    println!("      Auto-cleanup: ✅");

    // Project stores
    let project_store = temporary_vector_store("Project Alpha Documentation", 90)
        .add_file("file-project-spec-001")
        .add_file("file-meeting-notes-002")
        .add_file("file-progress-reports-003")
        .metadata("project_id", "proj_alpha_2024")
        .metadata("phase", "development");

    println!("   📅 Project-based (90 days):");
    println!("      Purpose: Project lifecycle documentation");
    println!("      Files: {}", project_store.file_count());
    println!("      Cleanup: After project completion");

    // Long-term knowledge stores
    let knowledge_store = simple_vector_store("Institutional Knowledge Base")
        .add_file("file-company-history-001")
        .add_file("file-best-practices-002")
        .add_file("file-lessons-learned-003")
        .metadata("retention", "permanent")
        .metadata("backup", "enabled")
        .metadata("compliance", "required");

    println!("   🏛️ Institutional (permanent):");
    println!("      Purpose: Long-term organizational knowledge");
    println!("      Files: {}", knowledge_store.file_count());
    println!("      Cleanup: Manual review only");

    // Demonstrate lifecycle events
    println!("\n🔄 Lifecycle Event Handling:");
    println!("   📥 Creation: Automatic indexing and optimization");
    println!("   🔄 Updates: Incremental re-indexing of modified files");
    println!("   📊 Monitoring: Usage tracking and performance metrics");
    println!("   ⚠️ Warnings: Expiration notifications and alerts");
    println!("   🗑️ Cleanup: Automatic or manual deletion processes");
    println!("   💾 Archival: Long-term storage for compliance");

    // Show cost optimization strategies
    println!("\n💰 Cost Optimization Strategies:");
    println!("   🎯 Smart expiration policies based on usage");
    println!("   📊 Analytics-driven storage optimization");
    println!("   🗜️ Automatic compression for archived content");
    println!("   🔄 Tiered storage (hot, warm, cold)");
    println!("   📈 Usage-based scaling recommendations");

    Ok(())
}

/// Example 6: Advanced Search Patterns and Optimization
fn run_advanced_search_patterns_example() -> Result<(), Error> {
    println!("\n🚀 Example 6: Advanced Search Patterns and Optimization");
    println!("{}", "=".repeat(60));

    // Create optimized search store
    let optimized_store = VectorStoreBuilder::new()
        .name("Advanced Search Optimization Store")
        .add_file("file-technical-docs-001")
        .add_file("file-user-feedback-002")
        .add_file("file-performance-data-003")
        .add_file("file-best-practices-004")
        .metadata("search_optimized", "true")
        .metadata("indexing", "enhanced")
        .metadata("caching", "enabled");

    println!("🚀 Created advanced search store:");
    println!("   Optimization: Enhanced indexing");
    println!("   Caching: Enabled");
    println!("   Files: {} documents", optimized_store.file_count());

    // Demonstrate advanced search patterns
    println!("\n🧠 Advanced Search Patterns:");

    // Multi-stage search
    println!("   1. 🎯 Multi-stage Search:");
    println!("      Stage 1: Broad semantic search (100 results)");
    println!("      Stage 2: Filtered refinement (20 results)");
    println!("      Stage 3: Relevance re-ranking (5 top results)");

    let multi_stage_search =
        VectorStoreSearchBuilder::new("advanced-store-789", "machine learning best practices")
            .limit(100)
            .filter("category", "best_practices")
            .filter("verified", "true");

    println!("      Query: '{}'", multi_stage_search.query());
    println!(
        "      Initial limit: {}",
        multi_stage_search.limit_ref().unwrap()
    );

    // Contextual search
    println!("   2. 🎭 Contextual Search:");
    println!("      Context: User role, project phase, domain expertise");
    println!("      Adaptation: Results tailored to user context");

    let _contextual_search =
        search_vector_store_with_limit("advanced-store-789", "deployment strategies", 15)
            .filter("audience", "senior_engineer")
            .filter("complexity", "advanced")
            .filter("domain", "cloud_infrastructure");

    println!("      Audience: senior_engineer");
    println!("      Complexity: advanced");
    println!("      Domain: cloud_infrastructure");

    // Hybrid search approaches
    println!("   3. 🔀 Hybrid Search Approaches:");
    println!("      Semantic similarity + keyword matching");
    println!("      Vector search + traditional full-text search");
    println!("      AI-enhanced query understanding");

    // Search performance optimization
    println!("\n⚡ Search Performance Optimization:");
    println!("   🎯 Query optimization and caching");
    println!("   📊 Result pre-computation for common queries");
    println!("   🔄 Incremental index updates");
    println!("   📈 Load balancing across vector stores");
    println!("   🧠 Machine learning-based relevance tuning");

    // Quality metrics and monitoring
    println!("\n📊 Search Quality Metrics:");
    println!("   🎯 Relevance scores and user feedback");
    println!("   ⏱️ Query response time analysis");
    println!("   🔍 Search success rate tracking");
    println!("   📈 Usage pattern analysis");
    println!("   🛠️ Continuous improvement recommendations");

    Ok(())
}

/// Helper function to create enterprise knowledge base structure
fn create_enterprise_knowledge_base() -> Result<Vec<(String, VectorStoreBuilder)>, Error> {
    let departments = vec![
        ("Engineering".to_string(), create_engineering_store()),
        ("Legal".to_string(), create_legal_store()),
        ("HR".to_string(), create_hr_store()),
        ("Marketing".to_string(), create_marketing_store()),
        ("Finance".to_string(), create_finance_store()),
        ("Operations".to_string(), create_operations_store()),
    ];

    Ok(departments)
}

fn create_engineering_store() -> VectorStoreBuilder {
    VectorStoreBuilder::new()
        .name("Engineering Knowledge Base")
        .add_file("file-architecture-docs-001")
        .add_file("file-coding-standards-002")
        .add_file("file-deployment-guides-003")
        .add_file("file-api-documentation-004")
        .metadata("department", "engineering")
        .metadata("access_level", "engineering_team")
        .metadata("update_frequency", "weekly")
        .expires_after_days(365)
}

fn create_legal_store() -> VectorStoreBuilder {
    VectorStoreBuilder::new()
        .name("Legal Documentation Store")
        .add_file("file-contracts-templates-001")
        .add_file("file-compliance-guides-002")
        .add_file("file-policy-documents-003")
        .metadata("department", "legal")
        .metadata("access_level", "legal_team")
        .metadata("confidentiality", "high")
        .expires_after_days(2555) // 7 years for legal retention
}

fn create_hr_store() -> VectorStoreBuilder {
    VectorStoreBuilder::new()
        .name("Human Resources Knowledge Base")
        .add_file("file-employee-handbook-001")
        .add_file("file-benefits-guide-002")
        .add_file("file-performance-templates-003")
        .metadata("department", "hr")
        .metadata("access_level", "hr_managers")
        .metadata("privacy", "employee_data")
        .expires_after_days(1095) // 3 years
}

fn create_marketing_store() -> VectorStoreBuilder {
    VectorStoreBuilder::new()
        .name("Marketing Materials Store")
        .add_file("file-brand-guidelines-001")
        .add_file("file-campaign-templates-002")
        .add_file("file-market-research-003")
        .metadata("department", "marketing")
        .metadata("access_level", "marketing_team")
        .metadata("content_type", "creative_assets")
        .expires_after_days(365)
}

fn create_finance_store() -> VectorStoreBuilder {
    VectorStoreBuilder::new()
        .name("Finance Documentation Store")
        .add_file("file-budget-templates-001")
        .add_file("file-financial-policies-002")
        .add_file("file-audit-procedures-003")
        .metadata("department", "finance")
        .metadata("access_level", "finance_team")
        .metadata("compliance", "required")
        .expires_after_days(2555) // 7 years for financial records
}

fn create_operations_store() -> VectorStoreBuilder {
    VectorStoreBuilder::new()
        .name("Operations Procedures Store")
        .add_file("file-standard-procedures-001")
        .add_file("file-incident-response-002")
        .add_file("file-vendor-management-003")
        .metadata("department", "operations")
        .metadata("access_level", "operations_team")
        .metadata("criticality", "high")
        .expires_after_days(730) // 2 years
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_vector_store() {
        let store = simple_vector_store("Test Store")
            .metadata("test", "true")
            .add_file("test-file-1");

        assert_eq!(store.name_ref(), Some("Test Store"));
        assert_eq!(store.file_count(), 1);
        assert!(store.has_files());
        assert_eq!(store.metadata_ref().get("test"), Some(&"true".to_string()));
    }

    #[test]
    fn test_vector_store_with_files() {
        let files = vec![
            "file-1".to_string(),
            "file-2".to_string(),
            "file-3".to_string(),
        ];
        let store = vector_store_with_files("Bulk Store", files.clone());

        assert_eq!(store.name_ref(), Some("Bulk Store"));
        assert_eq!(store.file_count(), 3);
        assert_eq!(store.file_ids_ref(), files.as_slice());
        assert!(store.has_files());
    }

    #[test]
    fn test_temporary_vector_store() {
        let store = temporary_vector_store("Temp Store", 30);

        assert_eq!(store.name_ref(), Some("Temp Store"));
        assert!(store.expires_after_ref().is_some());
        assert_eq!(store.expires_after_ref().unwrap().days, 30);
    }

    #[test]
    fn test_add_file_operation() {
        let file_op = add_file_to_vector_store("store-123", "file-456");

        assert_eq!(file_op.vector_store_id(), "store-123");
        assert_eq!(file_op.file_id(), "file-456");
    }

    #[test]
    fn test_search_operations() {
        let basic_search = search_vector_store("store-123", "test query");
        assert_eq!(basic_search.vector_store_id(), "store-123");
        assert_eq!(basic_search.query(), "test query");
        assert!(basic_search.limit_ref().is_none());

        let limited_search = search_vector_store_with_limit("store-456", "limited query", 10);
        assert_eq!(limited_search.limit_ref(), Some(10));
    }

    #[test]
    fn test_filtered_search() {
        let search = search_vector_store_with_limit("store-789", "filtered query", 5)
            .filter("category", "docs")
            .filter("priority", "high");

        assert_eq!(search.filter_ref().len(), 2);
        assert_eq!(
            search.filter_ref().get("category"),
            Some(&"docs".to_string())
        );
        assert_eq!(
            search.filter_ref().get("priority"),
            Some(&"high".to_string())
        );
    }

    #[test]
    fn test_enterprise_store_creation() {
        let eng_store = create_engineering_store();
        assert_eq!(eng_store.name_ref(), Some("Engineering Knowledge Base"));
        assert!(eng_store.has_files());
        assert!(eng_store.expires_after_ref().is_some());
        assert_eq!(
            eng_store.metadata_ref().get("department"),
            Some(&"engineering".to_string())
        );
    }

    #[test]
    fn test_vector_store_builder_fluent_interface() {
        let store = VectorStoreBuilder::new()
            .name("Fluent Store")
            .add_file("file-1")
            .add_file("file-2")
            .metadata("key1", "value1")
            .metadata("key2", "value2")
            .expires_after_days(60);

        assert_eq!(store.name_ref(), Some("Fluent Store"));
        assert_eq!(store.file_count(), 2);
        assert_eq!(store.metadata_ref().len(), 2);
        assert!(store.expires_after_ref().is_some());
        assert_eq!(store.expires_after_ref().unwrap().days, 60);
    }
}
