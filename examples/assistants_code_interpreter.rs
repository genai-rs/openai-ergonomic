#![allow(clippy::uninlined_format_args)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::unnecessary_wraps)]
//! Assistants API Code Interpreter Example
//!
//! This example demonstrates how to use the OpenAI Assistants API with the Code Interpreter tool
//! to perform data analysis, mathematical computations, and code execution tasks.
//!
//! ## Features Demonstrated
//!
//! - Creating assistants with code interpreter capabilities
//! - Uploading data files for analysis
//! - Running code execution tasks
//! - Handling code interpreter outputs (text, images, data files)
//! - Best practices for data analysis workflows
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
//! cargo run --example assistants_code_interpreter
//! ```
//!
//! ## Note on Implementation Status
//!
//! This example demonstrates the intended API design for code interpreter functionality.
//! The code shows how the ergonomic builders would work with the actual OpenAI API.

use openai_ergonomic::{
    builders::assistants::{
        assistant_with_instructions, simple_run, simple_thread, tool_code_interpreter,
        AssistantBuilder,
    },
    Client, Error,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 OpenAI Ergonomic - Code Interpreter Assistant Example\n");

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

    // Demonstrate different code interpreter use cases
    run_data_analysis_example()?;
    run_mathematical_computation_example()?;
    run_visualization_example()?;
    run_file_processing_example()?;

    println!("\n🎉 Code Interpreter examples completed successfully!");
    Ok(())
}

/// Example 1: Data Analysis with CSV Processing
fn run_data_analysis_example() -> Result<(), Error> {
    println!("📈 Example 1: Data Analysis with CSV Processing");
    println!("{}", "=".repeat(60));

    // Create an assistant specifically for data analysis
    let assistant = assistant_with_instructions(
        "gpt-4-1106-preview", // Model that supports code interpreter
        "Data Analysis Assistant",
        "You are a data analysis expert. Help users analyze datasets, create visualizations, and derive insights from data. Always explain your methodology and findings clearly.",
    )
    .description("A specialized assistant for data analysis tasks")
    .add_tool(tool_code_interpreter());

    println!("🤖 Created data analysis assistant:");
    println!("   Model: {}", assistant.model());
    println!("   Name: {}", assistant.name_ref().unwrap_or("unnamed"));
    println!(
        "   Description: {}",
        assistant.description_ref().unwrap_or("no description")
    );

    // Create a thread for the data analysis conversation
    let _thread = simple_thread().metadata("purpose", "data-analysis");

    println!("\n📝 Created thread with metadata:");
    println!("   Purpose: data-analysis");

    // Simulate data analysis workflow
    println!("\n💭 Analysis Request:");
    println!("   'I have sales data from the last quarter. Please analyze trends, identify top-performing products, and create visualizations showing monthly performance.'");

    println!("\n🔄 Code Interpreter Workflow:");
    println!("   1. 📁 Assistant receives and processes CSV data");
    println!("   2. 🐍 Executes Python code for data analysis");
    println!("   3. 📊 Generates visualizations (charts, graphs)");
    println!("   4. 📈 Calculates key metrics and trends");
    println!("   5. 📋 Provides summary report with insights");

    println!("\n✨ Expected Outputs:");
    println!("   • Data summary statistics");
    println!("   • Trend analysis charts");
    println!("   • Top product performance metrics");
    println!("   • Monthly comparison visualizations");
    println!("   • Actionable business insights");

    Ok(())
}

/// Example 2: Mathematical Computations and Modeling
fn run_mathematical_computation_example() -> Result<(), Error> {
    println!("\n🔢 Example 2: Mathematical Computations and Modeling");
    println!("{}", "=".repeat(60));

    // Create an assistant for mathematical tasks
    let math_assistant = AssistantBuilder::new("gpt-4-1106-preview")
        .name("Mathematics Professor")
        .description("Expert in mathematical computations, modeling, and problem solving")
        .instructions("You are a mathematics expert. Solve complex mathematical problems, create models, perform numerical analysis, and explain mathematical concepts clearly. Always show your work step by step.")
        .add_tool(tool_code_interpreter());

    println!("🧮 Created mathematics assistant:");
    println!("   Name: {}", math_assistant.name_ref().unwrap());
    println!("   Focus: Complex mathematical computations");

    // Create thread for mathematical discussion
    let _math_thread = simple_thread()
        .metadata("type", "mathematics")
        .metadata("complexity", "advanced");

    println!("\n📝 Mathematics Problem:");
    println!("   'Solve the differential equation dy/dx = x*y with initial condition y(0) = 1.'");
    println!("   'Then plot the solution and analyze its behavior.'");

    println!("\n🔬 Code Interpreter Mathematics Workflow:");
    println!("   1. 📐 Parse the differential equation");
    println!("   2. 🧮 Apply analytical or numerical methods");
    println!("   3. 💻 Implement solution in Python/SymPy");
    println!("   4. 📊 Generate solution plots");
    println!("   5. 📝 Provide step-by-step explanation");

    // Simulate creating a run for mathematical computation
    let math_run = simple_run("assistant-math-123")
        .instructions("Focus on providing clear mathematical explanations alongside code execution")
        .temperature(0.1); // Lower temperature for mathematical precision

    println!("\n🎯 Run Configuration:");
    println!("   Assistant ID: {}", math_run.assistant_id());
    println!(
        "   Temperature: {:?} (low for precision)",
        math_run.temperature_ref()
    );

    println!("\n✨ Expected Mathematical Outputs:");
    println!("   • Step-by-step solution derivation");
    println!("   • Python code for numerical verification");
    println!("   • Interactive plots showing solution behavior");
    println!("   • Analysis of solution properties (growth rate, asymptotes)");
    println!("   • Verification of initial conditions");

    Ok(())
}

/// Example 3: Data Visualization and Chart Generation
fn run_visualization_example() -> Result<(), Error> {
    println!("\n📊 Example 3: Data Visualization and Chart Generation");
    println!("{}", "=".repeat(60));

    // Create visualization-focused assistant
    let _viz_assistant = assistant_with_instructions(
        "gpt-4-1106-preview",
        "Visualization Specialist",
        "You are a data visualization expert. Create compelling, informative charts and graphs that effectively communicate data insights. Always consider best practices for visual design and choose appropriate chart types for the data."
    )
    .description("Creates professional data visualizations and charts")
    .add_tool(tool_code_interpreter());

    println!("📈 Created visualization assistant:");
    println!("   Specialty: Data visualization and chart creation");

    println!("\n📝 Visualization Request:");
    println!("   'Create a comprehensive dashboard showing website traffic data:'");
    println!("   • Monthly visitor trends (line chart)");
    println!("   • Traffic sources breakdown (pie chart)");
    println!("   • Page performance heatmap");
    println!("   • Conversion funnel visualization");

    println!("\n🎨 Code Interpreter Visualization Workflow:");
    println!("   1. 📋 Analyze data structure and requirements");
    println!("   2. 🎯 Select appropriate visualization types");
    println!("   3. 🐍 Generate Python code using matplotlib/seaborn/plotly");
    println!("   4. 🎨 Apply professional styling and color schemes");
    println!("   5. 📊 Create interactive or static visualizations");
    println!("   6. 💾 Export charts in various formats (PNG, SVG, HTML)");

    println!("\n✨ Expected Visualization Outputs:");
    println!("   • Professional-quality charts and graphs");
    println!("   • Interactive dashboards (when using plotly)");
    println!("   • Downloadable image files");
    println!("   • Chart customization code");
    println!("   • Data insights derived from visualizations");

    Ok(())
}

/// Example 4: File Processing and Analysis
fn run_file_processing_example() -> Result<(), Error> {
    println!("\n📁 Example 4: File Processing and Analysis");
    println!("{}", "=".repeat(60));

    // Create file processing assistant
    let _file_assistant = AssistantBuilder::new("gpt-4-1106-preview")
        .name("File Processing Expert")
        .description("Processes various file formats and performs analysis")
        .instructions(
            "You are a file processing expert. Handle various file formats (CSV, JSON, Excel, text files), clean and transform data, and perform comprehensive analysis. Always validate data integrity and handle edge cases."
        )
        .add_tool(tool_code_interpreter());

    println!("📄 Created file processing assistant:");
    println!("   Capabilities: Multi-format file processing and analysis");

    println!("\n📋 File Processing Tasks:");
    println!("   • Process uploaded CSV files with sales data");
    println!("   • Clean and validate data integrity");
    println!("   • Transform data formats (CSV → JSON → Excel)");
    println!("   • Generate summary statistics");
    println!("   • Create processed output files");

    println!("\n⚙️ Code Interpreter File Processing Workflow:");
    println!("   1. 📁 Accept and validate uploaded files");
    println!("   2. 🔍 Inspect file structure and content");
    println!("   3. 🧹 Clean and preprocess data");
    println!("   4. 🔄 Transform between formats");
    println!("   5. 📊 Perform statistical analysis");
    println!("   6. 📤 Generate processed output files");
    println!("   7. 📋 Provide processing summary and quality report");

    // Demonstrate error handling for file processing
    println!("\n🛡️ Error Handling for File Processing:");
    println!("   • File format validation");
    println!("   • Data type checking and conversion");
    println!("   • Missing value handling");
    println!("   • Memory-efficient processing for large files");
    println!("   • Graceful handling of corrupted data");

    println!("\n✨ Expected File Processing Outputs:");
    println!("   • Cleaned and validated datasets");
    println!("   • Multiple output formats (CSV, JSON, Excel)");
    println!("   • Data quality reports");
    println!("   • Processing logs and statistics");
    println!("   • Transformed data ready for analysis");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_analysis_assistant_creation() {
        let assistant = assistant_with_instructions(
            "gpt-4-1106-preview",
            "Test Data Analyst",
            "Test instructions for data analysis",
        )
        .add_tool(tool_code_interpreter());

        assert_eq!(assistant.model(), "gpt-4-1106-preview");
        assert_eq!(assistant.name_ref(), Some("Test Data Analyst"));
        assert_eq!(
            assistant.instructions_ref(),
            Some("Test instructions for data analysis")
        );
    }

    #[test]
    fn test_math_assistant_builder() {
        let assistant = AssistantBuilder::new("gpt-4")
            .name("Math Assistant")
            .description("Mathematics expert")
            .instructions("Solve math problems")
            .add_tool(tool_code_interpreter());

        assert_eq!(assistant.model(), "gpt-4");
        assert_eq!(assistant.name_ref(), Some("Math Assistant"));
        assert_eq!(assistant.description_ref(), Some("Mathematics expert"));
    }

    #[test]
    fn test_thread_metadata() {
        let thread = simple_thread()
            .metadata("purpose", "testing")
            .metadata("type", "unit-test");

        assert_eq!(thread.metadata_ref().len(), 2);
        assert_eq!(
            thread.metadata_ref().get("purpose"),
            Some(&"testing".to_string())
        );
        assert_eq!(
            thread.metadata_ref().get("type"),
            Some(&"unit-test".to_string())
        );
    }

    #[test]
    fn test_run_configuration() {
        let run = simple_run("test-assistant")
            .temperature(0.1)
            .stream(true)
            .instructions("Custom instructions for testing");

        assert_eq!(run.assistant_id(), "test-assistant");
        assert_eq!(run.temperature_ref(), Some(0.1));
        assert!(run.is_streaming());
        assert_eq!(
            run.instructions_ref(),
            Some("Custom instructions for testing")
        );
    }
}
