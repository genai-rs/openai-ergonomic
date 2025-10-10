#![allow(clippy::uninlined_format_args)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::useless_vec)]
#![allow(clippy::manual_let_else)]
//! Assistants API File Search Example (RAG Patterns)
//!
//! This example demonstrates how to use the OpenAI Assistants API with the File Search tool
//! for Retrieval-Augmented Generation (RAG) applications. It shows how to create knowledge-aware
//! assistants that can search through uploaded documents to provide accurate, contextual responses.
//!
//! ## Features Demonstrated
//!
//! - Creating assistants with file search capabilities
//! - Uploading and managing knowledge documents
//! - Vector store integration for semantic search
//! - RAG workflows: retrieval + generation
//! - Citation and source attribution
//! - Best practices for knowledge base management
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
//! cargo run --example assistants_file_search
//! ```
//!
//! ## Note on Implementation Status
//!
//! This example demonstrates the intended API design for file search and RAG functionality.
//! The code shows how the ergonomic builders work with vector stores and file search tools.

use openai_ergonomic::{
    builders::{
        assistants::{
            assistant_with_instructions, simple_thread, tool_file_search, AssistantBuilder,
        },
        vector_stores::{
            search_vector_store_with_limit, simple_vector_store, temporary_vector_store,
            vector_store_with_files, VectorStoreBuilder,
        },
    },
    Client, Error,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(" OpenAI Ergonomic - File Search Assistant Example (RAG)\n");

    // Initialize client from environment variables
    let _client = match Client::from_env() {
        Ok(client_builder) => {
            println!(" Client initialized successfully");
            client_builder.build()
        }
        Err(e) => {
            eprintln!(" Failed to initialize client: {e}");
            eprintln!(" Make sure OPENAI_API_KEY is set in your environment");
            return Err(e.into());
        }
    };

    // Demonstrate different RAG use cases
    run_knowledge_base_example()?;
    run_document_qa_example()?;
    run_research_assistant_example()?;
    run_citation_example()?;
    run_multi_document_analysis_example()?;

    println!("\n File Search RAG examples completed successfully!");
    Ok(())
}

/// Example 1: Building a Knowledge Base Assistant
fn run_knowledge_base_example() -> Result<(), Error> {
    println!(" Example 1: Building a Knowledge Base Assistant");
    println!("{}", "=".repeat(60));

    // Create a knowledge base vector store
    let knowledge_store = simple_vector_store("Company Knowledge Base")
        .metadata("type", "internal_docs")
        .metadata("department", "engineering")
        .expires_after_days(365); // Expire after 1 year

    println!(" Created knowledge base vector store:");
    println!("   Name: {}", knowledge_store.name_ref().unwrap());
    println!("   Type: Internal documentation");
    println!("   Expiration: 365 days");

    // Simulate adding documents to the knowledge base
    let file_ids = vec![
        "file-api-docs-123".to_string(),
        "file-coding-standards-456".to_string(),
        "file-deployment-guide-789".to_string(),
        "file-troubleshooting-101".to_string(),
    ];

    let populated_store = vector_store_with_files("Engineering Knowledge Base", file_ids.clone());

    println!("\n Knowledge Base Contents:");
    for (i, file_id) in populated_store.file_ids_ref().iter().enumerate() {
        println!("   {}. {}", i + 1, file_id);
    }
    println!("   Total files: {}", populated_store.file_count());

    // Create an assistant with file search capabilities
    let kb_assistant = assistant_with_instructions(
        "gpt-4-1106-preview",
        "Engineering Knowledge Assistant",
        "You are an engineering knowledge assistant. Help developers find relevant information from our internal documentation, coding standards, and deployment guides. Always provide accurate citations and suggest related resources when appropriate."
    )
    .description("Internal knowledge base assistant for engineering teams")
    .add_tool(tool_file_search());

    println!("\n Created knowledge base assistant:");
    println!("   Name: {}", kb_assistant.name_ref().unwrap());
    println!("   Capability: File search through engineering documents");

    println!("\n Example Knowledge Base Query:");
    println!("   'What are our coding standards for API documentation?'");

    println!("\n RAG Workflow for Knowledge Base:");
    println!("   1.  Search vector store for relevant documents");
    println!("   2.  Retrieve matching sections from coding standards");
    println!("   3.  Generate response based on retrieved content");
    println!("   4.  Provide citations to specific documents");
    println!("   5.  Suggest related topics or documents");

    println!("\n Expected Knowledge Base Response:");
    println!("   • Specific coding standards for API documentation");
    println!("   • Citations: 'coding-standards-456.md, section 3.2'");
    println!("   • Examples from deployment guide");
    println!("   • Related resources: troubleshooting guide");

    Ok(())
}

/// Example 2: Document Q&A Assistant
fn run_document_qa_example() -> Result<(), Error> {
    println!("\n Example 2: Document Q&A Assistant");
    println!("{}", "=".repeat(60));

    // Create specialized document Q&A assistant
    let _qa_assistant = AssistantBuilder::new("gpt-4-1106-preview")
        .name("Document Q&A Specialist")
        .description("Answers questions based on uploaded documents with high accuracy")
        .instructions(
            "You are a document Q&A specialist. Answer questions by searching through the provided documents. Always cite your sources, indicate confidence levels, and acknowledge when information is not available in the documents."
        )
        .add_tool(tool_file_search());

    println!(" Created Document Q&A assistant:");
    println!("   Specialty: Precise question answering from documents");

    // Create a temporary vector store for this Q&A session
    let qa_store = temporary_vector_store("Q&A Session Store", 7) // Expires in 7 days
        .add_file("file-research-paper-001")
        .add_file("file-user-manual-002")
        .add_file("file-technical-spec-003")
        .metadata("session_id", "qa-session-123")
        .metadata("user", "researcher-001");

    println!("\n Q&A Document Store:");
    println!("   Files: {} documents loaded", qa_store.file_count());
    println!("   Expiration: 7 days (temporary session)");
    println!("   Session ID: qa-session-123");

    println!("\n Example Q&A Queries:");
    let queries = vec![
        "What is the maximum throughput mentioned in the technical specifications?",
        "How do I configure the authentication system according to the user manual?",
        "What are the key findings from the research paper regarding performance?",
        "Are there any known limitations discussed in these documents?",
    ];

    for (i, query) in queries.iter().enumerate() {
        println!("   {}. {}", i + 1, query);
    }

    println!("\n Document Q&A RAG Workflow:");
    println!("   1.  Process user question");
    println!("   2.  Generate search queries for vector store");
    println!("   3.  Retrieve relevant document sections");
    println!("   4.  Rank and filter results by relevance");
    println!("   5.  Generate answer from retrieved context");
    println!("   6.  Add citations and confidence indicators");

    println!("\n Expected Q&A Response Format:");
    println!("    Direct answer to the question");
    println!("    Citations: [technical-spec-003.pdf, page 15]");
    println!("    Confidence: High (90%)");
    println!("    Related information: See also user-manual-002.pdf, section 4.3");
    println!("     Limitations: No information found about edge cases");

    Ok(())
}

/// Example 3: Research Assistant with Advanced RAG
fn run_research_assistant_example() -> Result<(), Error> {
    println!("\n Example 3: Research Assistant with Advanced RAG");
    println!("{}", "=".repeat(60));

    // Create research-focused assistant
    let _research_assistant = assistant_with_instructions(
        "gpt-4-1106-preview",
        "Research Assistant",
        "You are a research assistant specializing in literature review and analysis. Help researchers find relevant information, identify patterns across documents, synthesize findings, and suggest research directions. Always provide comprehensive citations and acknowledge research limitations."
    )
    .add_tool(tool_file_search());

    println!(" Created research assistant:");
    println!("   Focus: Literature review and cross-document analysis");

    // Create a comprehensive research vector store
    let _research_store = VectorStoreBuilder::new()
        .name("Research Literature Database")
        .add_file("file-paper-ai-ethics-001")
        .add_file("file-paper-ml-bias-002")
        .add_file("file-paper-fairness-003")
        .add_file("file-survey-responsible-ai-004")
        .add_file("file-whitepaper-governance-005")
        .metadata("domain", "AI Ethics")
        .metadata("papers_count", "50")
        .metadata("date_range", "2020-2024");

    println!("\n Research Literature Database:");
    println!("   Domain: AI Ethics and Responsible AI");
    println!("   Papers: 5 documents loaded (representing 50 papers)");
    println!("   Date range: 2020-2024");

    println!("\n Research Query:");
    println!(
        "   'What are the current approaches to addressing algorithmic bias in machine learning?'"
    );
    println!("   'Please provide a comprehensive overview with citations.'");

    println!("\n Advanced RAG Research Workflow:");
    println!("   1.  Query analysis and decomposition");
    println!("   2.  Multi-faceted search across all documents");
    println!("   3.  Semantic clustering of results");
    println!("   4.  Cross-reference analysis between papers");
    println!("   5.  Identify trends and patterns");
    println!("   6.  Synthesize comprehensive overview");
    println!("   7.  Provide detailed citations and references");

    // Demonstrate search refinement
    let refined_search =
        search_vector_store_with_limit("research-db-123", "algorithmic bias mitigation", 20)
            .filter("category", "methodology")
            .filter("confidence", "high");

    println!("\n Search Refinement:");
    println!("   Query: algorithmic bias mitigation");
    println!("   Limit: {} results", refined_search.limit_ref().unwrap());
    println!("   Filters: category=methodology, confidence=high");

    println!("\n Expected Research Response:");
    println!("    Executive Summary:");
    println!("      • Overview of current bias mitigation approaches");
    println!("      • Key methodological categories identified");
    println!("      • Emerging trends and best practices");
    println!("   ");
    println!("    Detailed Analysis:");
    println!("      • Pre-processing techniques (data augmentation, sampling)");
    println!("      • In-processing methods (fairness constraints, adversarial training)");
    println!("      • Post-processing approaches (threshold optimization, calibration)");
    println!("   ");
    println!("    Comprehensive Citations:");
    println!("      • [Smith et al., 2023] - Fairness constraints in ML training");
    println!("      • [Johnson & Lee, 2024] - Bias detection in neural networks");
    println!("      • [Research Survey, 2024] - Comprehensive bias mitigation review");
    println!("   ");
    println!("    Future Research Directions:");
    println!("      • Intersectional bias analysis");
    println!("      • Real-time bias monitoring");
    println!("      • Domain-specific mitigation strategies");

    Ok(())
}

/// Example 4: Citation and Source Attribution
fn run_citation_example() -> Result<(), Error> {
    println!("\n Example 4: Citation and Source Attribution");
    println!("{}", "=".repeat(60));

    // Create citation-focused assistant
    let _citation_assistant = AssistantBuilder::new("gpt-4-1106-preview")
        .name("Citation Specialist")
        .description("Provides detailed source attribution and citation management")
        .instructions(
            "You are a citation specialist. Always provide accurate, detailed citations for any information retrieved from documents. Use proper academic citation formats, include page numbers when available, and distinguish between direct quotes and paraphrased content."
        )
        .add_tool(tool_file_search());

    println!(" Created citation specialist assistant:");
    println!("   Focus: Accurate source attribution and citation formatting");

    // Create thread for citation-heavy work
    let _citation_thread = simple_thread()
        .metadata("citation_style", "APA")
        .metadata("requirement", "academic_standards")
        .metadata("verification", "enabled");

    println!("\n Citation Requirements:");
    println!("   Style: APA format");
    println!("   Standards: Academic-level accuracy");
    println!("   Verification: Enabled for all sources");

    println!("\n Citation Query:");
    println!("   'Provide a summary of the key arguments about privacy in AI systems,'");
    println!("   'with detailed citations for each point made.'");

    println!("\n Citation-Focused RAG Workflow:");
    println!("   1.  Search for relevant content across documents");
    println!("   2.  Extract content with precise location data");
    println!("   3.  Generate response with inline citations");
    println!("   4.  Verify citation accuracy and completeness");
    println!("   5.  Format citations according to specified style");
    println!("   6.  Cross-check for citation consistency");

    // Demonstrate different citation formats
    println!("\n Citation Format Examples:");
    println!("    Direct Quote:");
    println!(
        "      \"Privacy-preserving AI requires careful balance between utility and protection\""
    );
    println!("      (Johnson, 2024, p. 15)");
    println!("   ");
    println!("    Paraphrased Content:");
    println!("      Recent research indicates that differential privacy methods show promise");
    println!("      for protecting individual data in ML training (Smith & Lee, 2023).");
    println!("   ");
    println!("    Multiple Source Synthesis:");
    println!("      Several studies have demonstrated the effectiveness of federated learning");
    println!("      approaches (Chen et al., 2023; Rodriguez, 2024; Brown & Wilson, 2023).");

    println!("\n Expected Citation Response:");
    println!("    Structured Summary with Citations:");
    println!("      • Privacy challenges in AI systems (Anderson, 2024, pp. 23-25)");
    println!("      • Technical solutions: differential privacy (Johnson et al., 2023)");
    println!("      • Regulatory considerations (Privacy Commission Report, 2024, §3.2)");
    println!("   ");
    println!("    Reference List:");
    println!(
        "      Anderson, M. (2024). AI Privacy Challenges. Tech Ethics Journal, 15(3), 20-30."
    );
    println!("      Johnson, P., Smith, R., & Lee, K. (2023). Differential privacy in practice.");
    println!("      Privacy Commission. (2024). AI governance guidelines (Report #2024-AI-001).");

    Ok(())
}

/// Example 5: Multi-Document Analysis and Synthesis
fn run_multi_document_analysis_example() -> Result<(), Error> {
    println!("\n Example 5: Multi-Document Analysis and Synthesis");
    println!("{}", "=".repeat(60));

    // Create multi-document analysis assistant
    let _analysis_assistant = assistant_with_instructions(
        "gpt-4-1106-preview",
        "Document Analysis Specialist",
        "You are a document analysis expert. Compare and contrast information across multiple documents, identify contradictions or gaps, synthesize information from diverse sources, and provide comprehensive analysis that considers multiple perspectives."
    )
    .add_tool(tool_file_search());

    println!(" Created document analysis assistant:");
    println!("   Specialty: Cross-document comparison and synthesis");

    // Create comprehensive document store for analysis
    let _analysis_store = VectorStoreBuilder::new()
        .name("Multi-Document Analysis Store")
        .add_file("file-policy-proposal-v1")
        .add_file("file-policy-proposal-v2")
        .add_file("file-stakeholder-feedback-001")
        .add_file("file-legal-review-002")
        .add_file("file-technical-assessment-003")
        .add_file("file-cost-benefit-analysis-004")
        .metadata("analysis_type", "policy_comparison")
        .metadata("documents", "6")
        .metadata("perspectives", "multiple");

    println!("\n Multi-Document Analysis Setup:");
    println!("   Documents: 6 files representing different perspectives");
    println!("   Analysis type: Policy proposal comparison");
    println!("   Perspectives: Technical, legal, financial, stakeholder");

    println!("\n Multi-Document Analysis Query:");
    println!("   'Compare the two policy proposals and analyze how stakeholder feedback'");
    println!("   'has been incorporated. Identify any conflicts between the legal review'");
    println!("   'and technical assessment.'");

    println!("\n Multi-Document RAG Analysis Workflow:");
    println!("   1.  Identify key comparison dimensions");
    println!("   2.  Search each document type systematically");
    println!("   3.  Create comparison matrix across documents");
    println!("   4.  Identify conflicts and contradictions");
    println!("   5.  Find connections and dependencies");
    println!("   6.  Synthesize comprehensive analysis");
    println!("   7.  Provide recommendations based on synthesis");

    // Demonstrate advanced search patterns
    let _comparative_search =
        search_vector_store_with_limit("analysis-store-456", "risk assessment comparison", 15)
            .filter("document_type", "technical,legal")
            .filter("section", "risks");

    println!("\n Advanced Search Pattern:");
    println!("   Query: risk assessment comparison");
    println!("   Target documents: technical + legal reviews");
    println!("   Focus section: risk analysis sections");

    println!("\n Expected Multi-Document Analysis:");
    println!("    Comparative Analysis:");
    println!("      Policy Proposal Comparison:");
    println!("      • V1 focuses on immediate implementation (Technical Assessment)");
    println!("      • V2 incorporates phased approach (Stakeholder Feedback)");
    println!("   ");
    println!("    Identified Conflicts:");
    println!("      • Legal review flags compliance issues with V1 approach");
    println!("      • Technical assessment questions feasibility of V2 timeline");
    println!("      • Cost analysis shows budget misalignment between proposals");
    println!("   ");
    println!("    Stakeholder Integration:");
    println!("      • 73% of feedback incorporated in V2 (Stakeholder Feedback doc)");
    println!("      • Privacy concerns addressed through technical modifications");
    println!("      • Cost concerns partially resolved via phased implementation");
    println!("   ");
    println!("    Synthesis Recommendations:");
    println!("      • Hybrid approach combining V1 technical framework with V2 timeline");
    println!("      • Address legal compliance through additional technical review");
    println!("      • Require budget revision to align with stakeholder expectations");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_base_assistant() {
        let assistant = assistant_with_instructions(
            "gpt-4",
            "Test KB Assistant",
            "Test knowledge base assistant",
        )
        .add_tool(tool_file_search());

        assert_eq!(assistant.model(), "gpt-4");
        assert_eq!(assistant.name_ref(), Some("Test KB Assistant"));
    }

    #[test]
    fn test_vector_store_creation() {
        let store = simple_vector_store("Test Store")
            .metadata("test", "true")
            .expires_after_days(30);

        assert_eq!(store.name_ref(), Some("Test Store"));
        assert!(store.expires_after_ref().is_some());
        assert_eq!(store.expires_after_ref().unwrap().days, 30);
        assert_eq!(store.metadata_ref().get("test"), Some(&"true".to_string()));
    }

    #[test]
    fn test_vector_store_with_files() {
        let file_ids = vec!["file-1".to_string(), "file-2".to_string()];
        let store = vector_store_with_files("Files Store", file_ids.clone());

        assert_eq!(store.name_ref(), Some("Files Store"));
        assert_eq!(store.file_count(), 2);
        assert_eq!(store.file_ids_ref(), file_ids.as_slice());
        assert!(store.has_files());
    }

    #[test]
    fn test_search_builder() {
        let search = search_vector_store_with_limit("store-123", "test query", 10)
            .filter("category", "docs");

        assert_eq!(search.vector_store_id(), "store-123");
        assert_eq!(search.query(), "test query");
        assert_eq!(search.limit_ref(), Some(10));
        assert_eq!(search.filter_ref().len(), 1);
    }

    #[test]
    fn test_temporary_vector_store() {
        let store = temporary_vector_store("Temp Store", 7);

        assert_eq!(store.name_ref(), Some("Temp Store"));
        assert!(store.expires_after_ref().is_some());
        assert_eq!(store.expires_after_ref().unwrap().days, 7);
    }

    #[test]
    fn test_thread_metadata_for_citations() {
        let thread = simple_thread()
            .metadata("citation_style", "APA")
            .metadata("verification", "enabled");

        assert_eq!(thread.metadata_ref().len(), 2);
        assert_eq!(
            thread.metadata_ref().get("citation_style"),
            Some(&"APA".to_string())
        );
        assert_eq!(
            thread.metadata_ref().get("verification"),
            Some(&"enabled".to_string())
        );
    }
}
