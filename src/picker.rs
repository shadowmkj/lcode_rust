use crate::error::{EngineError, Result};
use crate::models::{Identifier, ProblemSummary};
use crate::{client::LeetCodeClient, models::Language};
use std::path::Path;
use std::{fs, process};

#[derive(Clone)]
pub struct Picker {
    pub client: LeetCodeClient,
}

impl Picker {
    pub fn new(client: LeetCodeClient) -> Self {
        Picker { client }
    }

    pub fn get_data_path() -> String {
        let project_dirs = directories::ProjectDirs::from("com", "shadowmkj", "leetrs").unwrap();
        let data_dir = project_dirs.data_dir();
        if !data_dir.exists()
            && let Err(e) = fs::create_dir_all(data_dir) {
                eprintln!("❌ Failed to create data directory: {}", e);
                process::exit(1);
            }
        data_dir.join("data.json").to_str().unwrap().to_string()
    }

    pub async fn pick(
        &self,
        identifier: &Identifier,
        language: &Option<Language>,
    ) -> Result<(String, String)> {
        let language = match language {
            Some(lang) => lang,
            None => {
                println!("🔤 No language specified, defaulting to Python.");
                &Language::Python
            }
        };

        if let Identifier::String(ident) = identifier {
            let snake_slug = ident.replace("-", "_");
            let code_filename = match language {
                Language::Python => format!("{}.py", snake_slug),
                Language::Rust => format!("{}.rs", snake_slug),
            };
            let desc_filename = format!("{}.md", snake_slug);
            if Path::new(&code_filename).exists() && Path::new(&desc_filename).exists() {
                return Ok((code_filename, desc_filename));
            }
        }

        let question = match identifier {
            Identifier::Number(num) => {
                println!("🔍 Fetching problem ID: {}...", num);
                self.client.get_question_by_id(*num).await.unwrap()
            }
            Identifier::String(identifier) => {
                println!("🔍 Fetching problem: {}...", identifier);
                self.client.get_question_by_slug(identifier).await.unwrap()
            }
        };

        // Convert LeetCode's raw HTML into wrapped terminal text (80 columns wide)
        let formatted_content = html2md::parse_html(&question.content);
        let md_content = format!("# {}\n\n{}", question.title, formatted_content);

        //  determine filenames (converting kebab-case to snake_case)
        let snake_slug = question.title_slug.replace("-", "_");
        let code_filename = match language {
            Language::Rust => format!("{}.rs", snake_slug),
            Language::Python => format!("{}.py", snake_slug),
        };
        let desc_filename = format!("{}.md", snake_slug);

        // write both files to disk
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

        Ok((code_filename, desc_filename))
    }

    pub async fn test_submit(&self, file: &String) {
        let code = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("❌ Failed to read file '{}': {}", file, e);
                return;
            }
        };

        // Extract the slug from the filename (e.g., "two_sum.rs" -> "two-sum")
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

        println!("🚀 Submitting {}...", file);
        let interpret_id = match self
            .client
            .test_code(
                &slug,
                &question.question_id,
                language.to_lang_slug(),
                &code,
                &question.example_test_cases,
            )
            .await
        {
            Ok(id) => id,
            Err(e) => {
                eprintln!("❌ Test Submission failed: {}", e);
                return;
            }
        };

        println!("⏳ Code queued. Waiting for execution results...");
        let result = match self.client.check_test_submission(interpret_id).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("❌ Failed to check test submission status: {}", e);
                return;
            }
        };

        println!("\n==================================================");

        let status = result.correct_answer.unwrap_or(false);

        if status {
            println!("  ✅ All test cases passed");
        } else {
            println!("  ❌ Test Failed");
        }

        println!("==================================================\n");

        if let (Some(correct), Some(total)) = (result.total_correct, result.total_testcases) {
            println!("🧪 Testcases: {} / {} passed", correct, total);
        }

        let status_msg = result.status_msg.unwrap_or("Unknown".to_string());
        if status && status_msg == "Accepted" {
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
        } else if status_msg == "Accepted" {
            if let (Some(code_answers), Some(expected)) = (
                result.code_answer.as_ref(),
                result.expected_code_answer.as_ref(),
            ) {
                println!("Expected");
                println!("{}", expected.join("\t"));
                println!("Output");
                println!("{}", code_answers.join("\t"));
            }
        } else if status_msg == "Runtime Error"
            && let Some(runtime_error) = result.full_runtime_error {
                println!("❌ Error\n{}", runtime_error);
            }
    }

    pub async fn submit(&self, file: &String) {
        let code = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("❌ Failed to read file '{}': {}", file, e);
                return;
            }
        };

        // Extract the slug from the filename (e.g., "two_sum.rs" -> "two-sum")
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

        println!("⏳ Code queued. Waiting for execution results...");
        let result = match self.client.check_submission(submission_id).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("❌ Failed to check submission status: {}", e);
                return;
            }
        };

        println!("\n==================================================");

        let status = result.status_msg.unwrap_or_else(|| "Unknown".to_string());

        if status == "Accepted" {
            println!("  ✅ {}", status);
        } else {
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
        } else if status == "Runtime Error"
            && let Some(runtime_error) = result.full_runtime_error {
                println!("❌ Error\n{}", runtime_error);
            }
    }

    pub async fn list_problems(&self) -> anyhow::Result<Vec<ProblemSummary>> {
        let data = match fs::read_to_string(Picker::get_data_path()) {
            Ok(v) => {
                // Fetch data in the background and update data.json for next time
                let client_clone = self.client.clone();
                tokio::spawn(async move {
                    let problems = client_clone
                        .get_problem_list()
                        .await
                        .expect("Failed to fetch problem list");
                    let data =
                        serde_json::to_string(&problems).expect("Failed to serialize problem list");
                    fs::write(Picker::get_data_path(), data).expect("Unable to write json to file");
                });
                v
            }
            Err(_) => {
                let problems = self
                    .client
                    .get_problem_list()
                    .await
                    .expect("Failed to fetch problem list");
                let data =
                    serde_json::to_string(&problems).expect("Failed to serialize problem list");
                fs::write(Picker::get_data_path(), data.clone())
                    .expect("Unable to write json to file");
                data
            }
        };
        let problems: Vec<ProblemSummary> = serde_json::from_str(&data).map_err(|e| {
            eprintln!("Failed to parse problem list: {}", e);
            eprintln!("Try running `leetrs tui` again to refresh the cache.");
            // Remove the data file since it's corrupted, so the next run will fetch fresh data
            // from the API
            if let Err(err) = fs::remove_file(Picker::get_data_path()) {
                eprintln!("Failed to remove corrupted cache file: {}", err);
            }
            e
        })?;
        Ok(problems)
    }
}
