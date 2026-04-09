use std::fs;

use crate::error::{EngineError, Result};
use crate::{client::LeetCodeClient, models::Language};

pub struct Picker {
    pub client: LeetCodeClient,
}

impl Picker {
    pub fn new(client: LeetCodeClient) -> Self {
        Picker { client }
    }

    pub async fn pick(
        &self,
        identifier: &String,
        language: &Option<Language>,
    ) -> Result<(String, String)> {
        let language = match language {
            Some(lang) => lang,
            None => {
                println!("🔤 No language specified, defaulting to Python.");
                &Language::Python
            }
        };

        let question = if identifier.chars().all(char::is_numeric) {
            let id: u64 = identifier.parse().unwrap();
            println!("🔍 Fetching problem ID: {}...", id);
            self.client.get_question_by_id(id).await.unwrap()
        } else {
            println!("🔍 Fetching problem: {}...", identifier);
            self.client.get_question_by_slug(&identifier).await.unwrap()
        };

        // Convert LeetCode's raw HTML into wrapped terminal text (80 columns wide)
        let formatted_content = html2md::parse_html(&question.content);
        let md_content = format!("# {}\n\n{}", question.title, formatted_content);
        // 2. determine filenames (converting kebab-case to snake_case)
        let snake_slug = question.title_slug.replace("-", "_");
        let code_filename = match language {
            Language::Rust => format!("{}.rs", snake_slug),
            Language::Python => format!("{}.py", snake_slug),
        };
        let desc_filename = format!("{}.md", snake_slug);

        // 3. write both files to disk
        let snippet = question
            .code_snippets
            .into_iter()
            .find(|s| s.lang_slug == language.to_lang_slug());

        let meta = match language {
            Language::Python => {
                format!(
                    "# id={} slug={} lang={}",
                    question.question_id,
                    question.title_slug,
                    language.to_lang_slug()
                )
            }
            Language::Rust => format!(
                "// id={} slug={} lang={}",
                question.question_id,
                question.title_slug,
                language.to_lang_slug()
            ),
        };

        if let Some(snippet) = snippet {
            if let Err(e) = fs::write(&code_filename, format!("{}\n\n{}", meta, snippet.code)) {
                eprintln!("❌ failed to write code file: {}", e);
                return Err(EngineError::System);
            }
            if let Err(e) = fs::write(&desc_filename, md_content) {
                eprintln!("❌ failed to write description file: {}", e);
                return Err(EngineError::System);
            }
            println!("✅ files generated successfully.");
        } else {
            eprintln!(
                "⚠️ no {} boilerplate found for this problem.",
                language.to_lang_slug()
            );
            return Err(EngineError::System);
        }

        return Ok((code_filename, desc_filename));
    }

    pub async fn submit(&self, file: &String) {
        // 1. Read the file content
        let code = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("❌ Failed to read file '{}': {}", file, e);
                return;
            }
        };

        // 2. Extract the slug from the filename (e.g., "two_sum.rs" -> "two-sum")
        let path = std::path::Path::new(&file);
        let file_stem = path
            .file_stem()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
        let slug = file_stem.replace("_", "-");
        println!("🔍 Resolving ID for '{}'...", slug);
        let language = Language::from_extension(
            path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or_default(),
        );
        // 3. Fetch the question to get its internal ID
        let question = match self.client.get_question_by_slug(&slug).await {
            Ok(q) => q,
            Err(e) => {
                eprintln!(
                    "❌ Failed to fetch question ID. Does the filename match the problem slug? Error: {}",
                    e
                );
                return;
            }
        };

        // 4. Submit the code
        println!("🚀 Submitting {}...", file);
        let submission_id = match self
            .client
            .submit_code(&slug, &question.question_id, language.to_lang_slug(), &code)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                eprintln!("❌ Submission failed: {}", e);
                return;
            }
        };

        // 5. Poll for results
        println!("⏳ Code queued. Waiting for execution results...");
        let result = match self.client.check_submission(submission_id).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("❌ Failed to check submission status: {}", e);
                return;
            }
        };

        // 6. Display the formatted results
        println!("\n==================================================");

        let status = result.status_msg.unwrap_or_else(|| "Unknown".to_string());

        if status == "Accepted" {
            // Print Accepted in Green
            // println!("  ✅ \x1b[32m{}\x1b[0m", status);
            println!("  ✅ {}", status);
        } else {
            // Print Errors/Wrong Answers in Red
            // println!("  ❌ \x1b[31m{}\x1b[0m", status);
            println!("  ❌ {}", status);
        }

        println!("==================================================\n");

        if let (Some(correct), Some(total)) = (result.total_correct, result.total_testcases) {
            println!("🧪 Testcases: {} / {} passed", correct, total);
        }

        if status == "Accepted" {
            if let Some(runtime) = result.status_runtime {
                println!("⏱️ Runtime: {}", runtime);
            }
            if let Some(memory) = result.status_memory {
                println!("💾 Memory: {}", memory);
            }
            if let Some(memory_percentile) = result.memory_percentile {
                println!("📝 Memory Percentile: {:.2}%", memory_percentile);
            }
            if let Some(runtime_percentile) = result.runtime_percentile {
                println!("⏰ Runtime Percentile: {:.2}%", runtime_percentile);
            }
        } else if status == "Compile Error" {
            if let Some(err_msg) = result.compile_error {
                println!("💥 Compiler Output:\n{}", err_msg);
            }
        } else if status == "Wrong Answer" {
            if let Some(input) = result.input {
                let parts = input.split("\n");
                print!("INPUT: ");
                for part in parts {
                    print!("{}\t", part);
                }
                println!();
            }
            if let Some(expected) = result.expected_output
                && let Some(output) = result.code_output
            {
                println!("Expected: {}\nOutput: {}", expected, output);
            }
        }
    }
}
