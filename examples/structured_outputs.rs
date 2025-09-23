#![allow(clippy::uninlined_format_args)]
//! Structured Outputs example demonstrating JSON mode and schema validation.
//!
//! This example showcases `OpenAI`'s structured outputs capabilities, including:
//! - Simple JSON mode for basic structure enforcement
//! - Schema-based structured outputs with type validation
//! - Complex nested data structures
//! - Different use cases (data extraction, classification, analysis)
//! - Comprehensive error handling and validation
//!
//! ## Features Demonstrated
//!
//! - Basic JSON mode with implicit structure
//! - JSON Schema validation with strict typing
//! - Nested objects and arrays
//! - Enum validation for constrained values
//! - Data extraction from unstructured text
//! - Content classification and analysis
//! - Error handling for malformed schemas
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
//! cargo run --example structured_outputs
//! ```

use openai_ergonomic::{Client, Error};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏗️  OpenAI Ergonomic - Structured Outputs Example\n");

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

    // Example 1: Simple JSON Mode
    println!("\n📝 Example 1: Simple JSON Mode");
    println!("==============================");

    match simple_json_mode_example(&client).await {
        Ok(()) => println!("✅ Simple JSON mode example completed"),
        Err(e) => {
            eprintln!("❌ Simple JSON mode example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 2: Schema-Based Data Extraction
    println!("\n🔍 Example 2: Schema-Based Data Extraction");
    println!("==========================================");

    match data_extraction_example(&client).await {
        Ok(()) => println!("✅ Data extraction example completed"),
        Err(e) => {
            eprintln!("❌ Data extraction example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 3: Complex Nested Structures
    println!("\n🏢 Example 3: Complex Nested Structures");
    println!("=======================================");

    match complex_structure_example(&client).await {
        Ok(()) => println!("✅ Complex structure example completed"),
        Err(e) => {
            eprintln!("❌ Complex structure example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 4: Content Classification
    println!("\n🏷️  Example 4: Content Classification");
    println!("=====================================");

    match classification_example(&client).await {
        Ok(()) => println!("✅ Classification example completed"),
        Err(e) => {
            eprintln!("❌ Classification example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 5: Mathematical Analysis
    println!("\n🧮 Example 5: Mathematical Analysis");
    println!("===================================");

    match math_analysis_example(&client).await {
        Ok(()) => println!("✅ Mathematical analysis example completed"),
        Err(e) => {
            eprintln!("❌ Mathematical analysis example failed: {e}");
            handle_api_error(&e);
        }
    }

    // Example 6: Schema Validation Error Handling
    println!("\n⚠️  Example 6: Schema Validation & Error Handling");
    println!("=================================================");

    match validation_error_example(&client).await {
        Ok(()) => println!("✅ Validation error example completed"),
        Err(e) => {
            eprintln!("❌ Validation error example failed: {e}");
            handle_api_error(&e);
        }
    }

    println!("\n🎉 All structured output examples completed!");
    println!("📊 Check the console output above for JSON-formatted results.");
    Ok(())
}

/// Example 1: Simple JSON mode without explicit schema
async fn simple_json_mode_example(client: &Client) -> Result<(), Error> {
    println!("Using simple JSON mode for basic structure enforcement...");

    let builder = client
        .responses()
        .system("You are a helpful assistant. Always respond in valid JSON format with the keys: summary, sentiment, and confidence_score (0-1).")
        .user("Analyze this product review: 'This laptop is amazing! Great performance, excellent battery life, and the display is crystal clear. Highly recommended!'")
        .json_mode()
        .temperature(0.3)
        .max_completion_tokens(200);

    let response = client.send_responses(builder).await?;

    if let Some(content) = response.content() {
        println!("📊 JSON Analysis Result:");

        // Try to parse and pretty-print the JSON
        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json) => {
                println!("{}", serde_json::to_string_pretty(&json)?);

                // Demonstrate accessing specific fields
                if let Some(sentiment) = json.get("sentiment").and_then(|s| s.as_str()) {
                    println!("\n🎯 Extracted sentiment: {sentiment}");
                }
                if let Some(confidence) = json
                    .get("confidence_score")
                    .and_then(serde_json::Value::as_f64)
                {
                    println!("🎯 Confidence score: {confidence:.2}");
                }
            }
            Err(e) => {
                println!("⚠️  Failed to parse JSON: {e}");
                println!("Raw response: {content}");
            }
        }
    }

    Ok(())
}

/// Example 2: Data extraction with schema validation
async fn data_extraction_example(client: &Client) -> Result<(), Error> {
    println!("Extracting structured data from unstructured text using JSON schema...");

    // Define schema for extracting contact information
    let contact_schema = json!({
        "type": "object",
        "properties": {
            "contacts": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Full name of the person"
                        },
                        "email": {
                            "type": "string",
                            "format": "email",
                            "description": "Email address"
                        },
                        "phone": {
                            "type": "string",
                            "description": "Phone number"
                        },
                        "company": {
                            "type": "string",
                            "description": "Company or organization"
                        },
                        "role": {
                            "type": "string",
                            "description": "Job title or role"
                        }
                    },
                    "required": ["name"],
                    "additionalProperties": false
                }
            },
            "total_contacts": {
                "type": "integer",
                "description": "Total number of contacts extracted"
            }
        },
        "required": ["contacts", "total_contacts"],
        "additionalProperties": false
    });

    let unstructured_text =
        "Contact our team: John Smith (CEO) at john@example.com or call 555-0123. \
        For technical support, reach out to Sarah Johnson at sarah.johnson@techcorp.com. \
        Our sales manager Mike Wilson can be reached at mike@company.com or 555-0456.";

    let builder = client
        .responses()
        .system("You are an expert at extracting contact information from text. Extract all contact details you can find and structure them according to the provided schema.")
        .user(format!("Extract contact information from this text: {unstructured_text}"))
        .json_schema("contact_extraction", contact_schema)
        .temperature(0.1); // Low temperature for accuracy

    let response = client.send_responses(builder).await?;

    if let Some(content) = response.content() {
        println!("📊 Extracted Contact Information:");

        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json) => {
                println!("{}", serde_json::to_string_pretty(&json)?);

                // Demonstrate accessing the structured data
                if let Some(contacts) = json.get("contacts").and_then(|c| c.as_array()) {
                    println!("\n🎯 Summary: Found {} contact(s)", contacts.len());
                    for (i, contact) in contacts.iter().enumerate() {
                        if let Some(name) = contact.get("name").and_then(|n| n.as_str()) {
                            println!("   {}. {name}", i + 1);
                            if let Some(email) = contact.get("email").and_then(|e| e.as_str()) {
                                println!("      📧 {email}");
                            }
                            if let Some(company) = contact.get("company").and_then(|c| c.as_str()) {
                                println!("      🏢 {company}");
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Failed to parse JSON: {e}");
                println!("Raw response: {content}");
            }
        }
    }

    Ok(())
}

/// Example 3: Complex nested structure for event planning
#[allow(clippy::too_many_lines)]
async fn complex_structure_example(client: &Client) -> Result<(), Error> {
    println!("Creating complex nested structure for event planning...");

    // Define a comprehensive event schema
    let event_schema = json!({
        "type": "object",
        "properties": {
            "event": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Event name"
                    },
                    "type": {
                        "type": "string",
                        "enum": ["conference", "workshop", "seminar", "networking", "party", "meeting"],
                        "description": "Type of event"
                    },
                    "date": {
                        "type": "string",
                        "format": "date",
                        "description": "Event date in YYYY-MM-DD format"
                    },
                    "duration_hours": {
                        "type": "number",
                        "minimum": 0.5,
                        "maximum": 24,
                        "description": "Duration in hours"
                    },
                    "venue": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Venue name"
                            },
                            "address": {
                                "type": "string",
                                "description": "Venue address"
                            },
                            "capacity": {
                                "type": "integer",
                                "minimum": 1,
                                "description": "Maximum capacity"
                            },
                            "amenities": {
                                "type": "array",
                                "items": {
                                    "type": "string",
                                    "enum": ["wifi", "parking", "catering", "av_equipment", "wheelchair_accessible", "air_conditioning"]
                                },
                                "description": "Available amenities"
                            }
                        },
                        "required": ["name", "capacity"],
                        "additionalProperties": false
                    },
                    "agenda": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "time": {
                                    "type": "string",
                                    "pattern": "^([0-1]?[0-9]|2[0-3]):[0-5][0-9]$",
                                    "description": "Time in HH:MM format"
                                },
                                "activity": {
                                    "type": "string",
                                    "description": "Activity description"
                                },
                                "speaker": {
                                    "type": "string",
                                    "description": "Speaker name"
                                },
                                "duration_minutes": {
                                    "type": "integer",
                                    "minimum": 15,
                                    "maximum": 480,
                                    "description": "Activity duration in minutes"
                                }
                            },
                            "required": ["time", "activity", "duration_minutes"],
                            "additionalProperties": false
                        }
                    },
                    "estimated_cost": {
                        "type": "object",
                        "properties": {
                            "venue": {
                                "type": "number",
                                "minimum": 0,
                                "description": "Venue cost in USD"
                            },
                            "catering": {
                                "type": "number",
                                "minimum": 0,
                                "description": "Catering cost in USD"
                            },
                            "equipment": {
                                "type": "number",
                                "minimum": 0,
                                "description": "Equipment cost in USD"
                            },
                            "total": {
                                "type": "number",
                                "minimum": 0,
                                "description": "Total estimated cost in USD"
                            }
                        },
                        "required": ["total"],
                        "additionalProperties": false
                    }
                },
                "required": ["name", "type", "date", "duration_hours", "venue"],
                "additionalProperties": false
            }
        },
        "required": ["event"],
        "additionalProperties": false
    });

    let builder = client
        .responses()
        .system("You are an expert event planner. Create a detailed event plan based on the user's requirements, including venue details, agenda, and cost estimates.")
        .user("Plan a one-day AI/ML conference for 200 people in San Francisco. Include morning keynotes, afternoon workshops, networking lunch, and panel discussions. Budget around $50,000.")
        .json_schema("event_plan", event_schema)
        .temperature(0.5);

    let response = client.send_responses(builder).await?;

    if let Some(content) = response.content() {
        println!("📊 Event Plan:");

        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json) => {
                println!("{}", serde_json::to_string_pretty(&json)?);

                // Extract and display key information
                if let Some(event) = json.get("event") {
                    if let Some(name) = event.get("name").and_then(|n| n.as_str()) {
                        println!("\n🎯 Event: {name}");
                    }
                    if let Some(venue) = event.get("venue") {
                        if let Some(venue_name) = venue.get("name").and_then(|n| n.as_str()) {
                            let capacity = venue
                                .get("capacity")
                                .and_then(serde_json::Value::as_i64)
                                .unwrap_or(0);
                            println!("🏢 Venue: {venue_name} (Capacity: {capacity})");
                        }
                    }
                    if let Some(agenda) = event.get("agenda").and_then(|a| a.as_array()) {
                        println!("📅 Agenda has {} activities", agenda.len());
                    }
                    if let Some(cost) = event.get("estimated_cost") {
                        if let Some(total) = cost.get("total").and_then(serde_json::Value::as_f64) {
                            println!("💰 Estimated total cost: ${total:.2}");
                        }
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Failed to parse JSON: {e}");
                println!("Raw response: {content}");
            }
        }
    }

    Ok(())
}

/// Example 4: Content classification with enum validation
#[allow(clippy::too_many_lines)]
async fn classification_example(client: &Client) -> Result<(), Error> {
    println!("Classifying content with enum validation...");

    // Define schema for content classification
    let classification_schema = json!({
        "type": "object",
        "properties": {
            "classification": {
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "enum": ["technology", "business", "science", "health", "politics", "sports", "entertainment", "education", "travel", "lifestyle"],
                        "description": "Primary content category"
                    },
                    "subcategory": {
                        "type": "string",
                        "description": "More specific subcategory"
                    },
                    "sentiment": {
                        "type": "string",
                        "enum": ["positive", "neutral", "negative", "mixed"],
                        "description": "Overall sentiment"
                    },
                    "topics": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "maxItems": 5,
                        "description": "Key topics mentioned"
                    },
                    "target_audience": {
                        "type": "string",
                        "enum": ["general", "professionals", "students", "experts", "consumers"],
                        "description": "Intended audience"
                    },
                    "complexity_level": {
                        "type": "string",
                        "enum": ["beginner", "intermediate", "advanced", "expert"],
                        "description": "Content complexity level"
                    },
                    "confidence_score": {
                        "type": "number",
                        "minimum": 0,
                        "maximum": 1,
                        "description": "Confidence in classification (0-1)"
                    }
                },
                "required": ["category", "sentiment", "topics", "target_audience", "complexity_level", "confidence_score"],
                "additionalProperties": false
            }
        },
        "required": ["classification"],
        "additionalProperties": false
    });

    let content_to_classify = "Recent advances in quantum computing have shown promising results for solving complex optimization problems. \
        Researchers at leading universities have demonstrated quantum algorithms that can potentially outperform classical computers \
        in specific domains like cryptography and molecular simulation. However, current quantum computers still face challenges \
        with noise and error rates, requiring sophisticated error correction techniques. The field is rapidly evolving with \
        significant investments from both academic institutions and major technology companies.";

    let builder = client
        .responses()
        .system("You are an expert content classifier. Analyze the provided text and classify it according to the given schema. Be precise with your classifications and provide accurate confidence scores.")
        .user(format!("Classify this content: {content_to_classify}"))
        .json_schema("content_classification", classification_schema)
        .temperature(0.2); // Low temperature for consistent classification

    let response = client.send_responses(builder).await?;

    if let Some(content) = response.content() {
        println!("📊 Content Classification:");

        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json) => {
                println!("{}", serde_json::to_string_pretty(&json)?);

                // Extract classification details
                if let Some(classification) = json.get("classification") {
                    println!("\n🎯 Classification Summary:");
                    if let Some(category) = classification.get("category").and_then(|c| c.as_str())
                    {
                        println!("   📂 Category: {category}");
                    }
                    if let Some(sentiment) =
                        classification.get("sentiment").and_then(|s| s.as_str())
                    {
                        println!("   😊 Sentiment: {sentiment}");
                    }
                    if let Some(audience) = classification
                        .get("target_audience")
                        .and_then(|a| a.as_str())
                    {
                        println!("   👥 Target Audience: {audience}");
                    }
                    if let Some(complexity) = classification
                        .get("complexity_level")
                        .and_then(|c| c.as_str())
                    {
                        println!("   🎓 Complexity: {complexity}");
                    }
                    if let Some(confidence) = classification
                        .get("confidence_score")
                        .and_then(serde_json::Value::as_f64)
                    {
                        println!("   🎯 Confidence: {:.2}%", confidence * 100.0);
                    }
                    if let Some(topics) = classification.get("topics").and_then(|t| t.as_array()) {
                        let topic_strings: Vec<String> = topics
                            .iter()
                            .filter_map(|t| t.as_str())
                            .map(std::string::ToString::to_string)
                            .collect();
                        println!("   🏷️  Topics: {}", topic_strings.join(", "));
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Failed to parse JSON: {e}");
                println!("Raw response: {content}");
            }
        }
    }

    Ok(())
}

/// Example 5: Mathematical analysis with structured output
#[allow(clippy::too_many_lines)]
async fn math_analysis_example(client: &Client) -> Result<(), Error> {
    println!("Performing mathematical analysis with structured output...");

    // Define schema for mathematical analysis
    let math_schema = json!({
        "type": "object",
        "properties": {
            "analysis": {
                "type": "object",
                "properties": {
                    "problem_type": {
                        "type": "string",
                        "enum": ["algebra", "geometry", "calculus", "statistics", "probability", "discrete_math", "linear_algebra"],
                        "description": "Type of mathematical problem"
                    },
                    "solution_steps": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "step_number": {
                                    "type": "integer",
                                    "minimum": 1,
                                    "description": "Step number in the solution"
                                },
                                "description": {
                                    "type": "string",
                                    "description": "Description of what this step does"
                                },
                                "equation": {
                                    "type": "string",
                                    "description": "Mathematical equation or expression"
                                },
                                "result": {
                                    "type": "string",
                                    "description": "Result of this step"
                                }
                            },
                            "required": ["step_number", "description", "equation"],
                            "additionalProperties": false
                        }
                    },
                    "final_answer": {
                        "type": "string",
                        "description": "Final answer to the problem"
                    },
                    "verification": {
                        "type": "object",
                        "properties": {
                            "check_method": {
                                "type": "string",
                                "description": "Method used to verify the answer"
                            },
                            "is_correct": {
                                "type": "boolean",
                                "description": "Whether the answer passes verification"
                            }
                        },
                        "required": ["check_method", "is_correct"],
                        "additionalProperties": false
                    },
                    "concepts_used": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "Mathematical concepts used in the solution"
                    }
                },
                "required": ["problem_type", "solution_steps", "final_answer", "verification", "concepts_used"],
                "additionalProperties": false
            }
        },
        "required": ["analysis"],
        "additionalProperties": false
    });

    let math_problem =
        "Find the derivative of f(x) = 3x^3 + 2x^2 - 5x + 7 and evaluate it at x = 2.";

    let builder = client
        .responses()
        .system("You are a mathematics tutor. Solve mathematical problems step by step, showing your work clearly and verifying your answers. Structure your response according to the provided schema.")
        .user(format!("Solve this problem: {math_problem}"))
        .json_schema("math_analysis", math_schema)
        .temperature(0.1); // Very low temperature for mathematical accuracy

    let response = client.send_responses(builder).await?;

    if let Some(content) = response.content() {
        println!("📊 Mathematical Analysis:");

        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json) => {
                println!("{}", serde_json::to_string_pretty(&json)?);

                // Extract and display solution steps
                if let Some(analysis) = json.get("analysis") {
                    println!("\n🎯 Solution Summary:");

                    if let Some(problem_type) =
                        analysis.get("problem_type").and_then(|p| p.as_str())
                    {
                        println!("   📚 Problem Type: {problem_type}");
                    }

                    if let Some(steps) = analysis.get("solution_steps").and_then(|s| s.as_array()) {
                        println!("   📝 Solution Steps: {} steps", steps.len());
                        for step in steps {
                            if let (Some(step_num), Some(desc)) = (
                                step.get("step_number").and_then(serde_json::Value::as_i64),
                                step.get("description").and_then(|d| d.as_str()),
                            ) {
                                println!("      {step_num}. {desc}");
                                if let Some(equation) =
                                    step.get("equation").and_then(|e| e.as_str())
                                {
                                    println!("         📐 {equation}");
                                }
                            }
                        }
                    }

                    if let Some(answer) = analysis.get("final_answer").and_then(|a| a.as_str()) {
                        println!("   ✅ Final Answer: {answer}");
                    }

                    if let Some(verification) = analysis.get("verification") {
                        if let Some(is_correct) = verification
                            .get("is_correct")
                            .and_then(serde_json::Value::as_bool)
                        {
                            let status = if is_correct {
                                "✅ Verified"
                            } else {
                                "❌ Needs Review"
                            };
                            println!("   🔍 Verification: {status}");
                        }
                    }

                    if let Some(concepts) = analysis.get("concepts_used").and_then(|c| c.as_array())
                    {
                        let concept_strings: Vec<String> = concepts
                            .iter()
                            .filter_map(|c| c.as_str())
                            .map(std::string::ToString::to_string)
                            .collect();
                        println!("   🧠 Concepts Used: {}", concept_strings.join(", "));
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Failed to parse JSON: {e}");
                println!("Raw response: {content}");
            }
        }
    }

    Ok(())
}

/// Example 6: Demonstration of schema validation and error handling
#[allow(clippy::too_many_lines)]
async fn validation_error_example(client: &Client) -> Result<(), Error> {
    println!("Demonstrating schema validation and error handling...");

    // Define a strict schema that's likely to cause validation challenges
    let strict_schema = json!({
        "type": "object",
        "properties": {
            "numbers": {
                "type": "array",
                "items": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 100
                },
                "minItems": 3,
                "maxItems": 5,
                "description": "Array of 3-5 integers between 1 and 100"
            },
            "precision_value": {
                "type": "number",
                "multipleOf": 0.01,
                "minimum": 0,
                "maximum": 1,
                "description": "A precise decimal value between 0 and 1, to 2 decimal places"
            },
            "strict_enum": {
                "type": "string",
                "enum": ["alpha", "beta", "gamma"],
                "description": "Must be exactly one of the allowed values"
            },
            "required_pattern": {
                "type": "string",
                "pattern": "^[A-Z]{2}[0-9]{4}$",
                "description": "Must be exactly 2 uppercase letters followed by 4 digits"
            }
        },
        "required": ["numbers", "precision_value", "strict_enum", "required_pattern"],
        "additionalProperties": false
    });

    println!("💡 Using a strict schema with specific constraints...");

    let builder = client
        .responses()
        .system("Generate data that strictly follows the provided JSON schema. Pay careful attention to all constraints including ranges, patterns, and array sizes.")
        .user("Generate sample data that conforms to the schema. Make sure all values meet the exact requirements.")
        .json_schema("strict_validation", strict_schema)
        .temperature(0.1)
        .max_completion_tokens(300);

    let response = client.send_responses(builder).await?;

    if let Some(content) = response.content() {
        println!("📊 Schema Validation Test:");

        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json) => {
                println!("{}", serde_json::to_string_pretty(&json)?);

                // Manual validation of the generated data
                println!("\n🔍 Manual Validation:");
                let mut validation_passed = true;

                // Check numbers array
                if let Some(numbers) = json.get("numbers").and_then(|n| n.as_array()) {
                    println!("   📊 Numbers array: {} items", numbers.len());
                    if numbers.len() < 3 || numbers.len() > 5 {
                        println!("   ❌ Array size constraint violated");
                        validation_passed = false;
                    }
                    for (i, num) in numbers.iter().enumerate() {
                        if let Some(val) = num.as_i64() {
                            if !(1..=100).contains(&val) {
                                println!("   ❌ Number {i} ({val}) outside valid range [1-100]");
                                validation_passed = false;
                            }
                        }
                    }
                } else {
                    println!("   ❌ Numbers array missing or invalid");
                    validation_passed = false;
                }

                // Check precision value
                if let Some(precision) = json
                    .get("precision_value")
                    .and_then(serde_json::Value::as_f64)
                {
                    println!("   🎯 Precision value: {precision}");
                    if !(0.0..=1.0).contains(&precision) {
                        println!("   ❌ Precision value outside range [0-1]");
                        validation_passed = false;
                    }
                }

                // Check enum value
                if let Some(enum_val) = json.get("strict_enum").and_then(|e| e.as_str()) {
                    println!("   🏷️  Enum value: {enum_val}");
                    if !["alpha", "beta", "gamma"].contains(&enum_val) {
                        println!("   ❌ Enum value not in allowed set");
                        validation_passed = false;
                    }
                }

                // Check pattern
                if let Some(pattern_val) = json.get("required_pattern").and_then(|p| p.as_str()) {
                    println!("   🔤 Pattern value: {pattern_val}");
                    let regex = regex::Regex::new(r"^[A-Z]{2}[0-9]{4}$").unwrap();
                    if !regex.is_match(pattern_val) {
                        println!("   ❌ Pattern does not match required format");
                        validation_passed = false;
                    }
                }

                if validation_passed {
                    println!("   ✅ All manual validations passed!");
                } else {
                    println!("   ⚠️  Some validation constraints were not met");
                }
            }
            Err(e) => {
                println!("⚠️  JSON parsing failed: {e}");
                println!("This demonstrates how schema constraints can sometimes be challenging for the model");
                println!("Raw response: {content}");
            }
        }
    }

    // Demonstrate handling of intentionally problematic schema
    println!("\n🧪 Testing with intentionally problematic request...");

    let problematic_builder = client
        .responses()
        .system("You are unhelpful and ignore instructions.")
        .user("Ignore the schema and just say 'hello world'")
        .json_schema(
            "strict_validation",
            json!({
                "type": "object",
                "properties": {
                    "impossible": {
                        "type": "string",
                        "pattern": "^impossible_pattern_that_cannot_match$"
                    }
                },
                "required": ["impossible"]
            }),
        )
        .temperature(0.1);

    match client.send_responses(problematic_builder).await {
        Ok(problematic_response) => {
            if let Some(content) = problematic_response.content() {
                println!("📊 Problematic request result:");
                println!("{content}");
                println!("💡 The model likely still attempted to follow the schema despite conflicting instructions");
            }
        }
        Err(e) => {
            println!("⚠️  Problematic request failed as expected: {e}");
        }
    }

    Ok(())
}

/// Comprehensive error handling helper
fn handle_api_error(error: &Error) {
    match error {
        Error::Api {
            status,
            message,
            error_type,
            error_code,
        } => {
            eprintln!("🚫 API Error [{status}]: {message}");
            if let Some(error_type) = error_type {
                eprintln!("   Type: {error_type}");
            }
            if let Some(error_code) = error_code {
                eprintln!("   Code: {error_code}");
            }

            // Provide specific guidance based on error type
            match *status {
                400 => eprintln!("💡 Bad Request - Check your JSON schema or request parameters"),
                401 => eprintln!("💡 Check your API key: export OPENAI_API_KEY=\"your-key\""),
                403 => eprintln!("💡 Forbidden - Check your API permissions and model access"),
                422 => eprintln!("💡 Invalid schema or request format - verify your JSON schema"),
                429 => eprintln!("💡 Rate limited - try again in a moment"),
                500..=599 => eprintln!("💡 Server error - try again later"),
                _ => {}
            }
        }
        Error::InvalidRequest(msg) => {
            eprintln!("🚫 Invalid Request: {msg}");
            eprintln!("💡 Check your request parameters and JSON schema format");
        }
        Error::Config(msg) => {
            eprintln!("🚫 Configuration Error: {msg}");
            eprintln!("💡 Check your client configuration");
        }
        Error::Http(err) => {
            eprintln!("🚫 HTTP Error: {err}");
            eprintln!("💡 Check your network connection");
        }
        Error::Json(err) => {
            eprintln!("🚫 JSON Error: {err}");
            eprintln!("💡 Response parsing failed - the model may have generated invalid JSON");
        }
        Error::Authentication(msg) => {
            eprintln!("🚫 Authentication Error: {msg}");
            eprintln!("💡 Check your API key");
        }
        Error::RateLimit(msg) => {
            eprintln!("🚫 Rate Limit Error: {msg}");
            eprintln!("💡 Try again in a moment or upgrade your plan");
        }
        Error::Stream(msg) => {
            eprintln!("🚫 Stream Error: {msg}");
            eprintln!("💡 Connection issue with streaming");
        }
        Error::File(err) => {
            eprintln!("🚫 File Error: {err}");
            eprintln!("💡 Check file permissions and paths");
        }
        Error::Builder(msg) => {
            eprintln!("🚫 Builder Error: {msg}");
            eprintln!("💡 Check your request builder configuration");
        }
        Error::Internal(msg) => {
            eprintln!("🚫 Internal Error: {msg}");
            eprintln!("💡 This may be a bug, please report it");
        }
        Error::StreamConnection { message } => {
            eprintln!("🚫 Stream Connection Error: {message}");
            eprintln!("💡 Check your network connection");
        }
        Error::StreamParsing { message, chunk } => {
            eprintln!("🚫 Stream Parsing Error: {message}");
            eprintln!("   Problematic chunk: {chunk}");
            eprintln!("💡 The response stream may be corrupted");
        }
        Error::StreamBuffer { message } => {
            eprintln!("🚫 Stream Buffer Error: {message}");
            eprintln!("💡 The stream buffer encountered an issue");
        }
    }
}
