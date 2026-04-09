use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
pub struct GraphQLQuery {
    pub query: String,
    pub variables: serde_json::Value,
    #[serde(rename = "operationName")]
    pub operation_name: Option<String>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Language {
    Python,
    Rust,
}

impl Language {
    pub fn to_lang_slug(&self) -> &'static str {
        match self {
            Language::Python => "python3",
            Language::Rust => "rust",
        }
    }

    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "py" => Language::Python,
            "rs" => Language::Rust,
            _ => Language::Python,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct QuestionSnippet {
    #[serde(rename = "langSlug")]
    pub lang_slug: String,
    pub code: String,
}

#[derive(Deserialize, Debug)]
pub struct Question {
    #[serde(rename = "questionId")]
    pub question_id: String,
    #[serde(rename = "titleSlug")]
    pub title_slug: String,
    pub title: String,
    pub content: String,
    #[serde(rename = "codeSnippets")]
    pub code_snippets: Vec<QuestionSnippet>,
}

#[derive(Debug, Clone)]
pub struct ProblemSummary {
    pub id: u64,
    pub title: String,
    pub slug: String,
    pub difficulty: u8, // 1 = Easy, 2 = Medium, 3 = Hard
}

// ==========================================
// Submission Models
// ==========================================

#[derive(Serialize, Debug)]
pub struct SubmitPayload {
    pub lang: String,
    pub question_id: String,
    pub typed_code: String,
}

#[derive(Deserialize, Debug)]
pub struct SubmitResponse {
    pub submission_id: u64,
}

#[derive(Deserialize, Debug)]
pub struct SubmissionCheckResult {
    pub state: String,              // "PENDING", "STARTED", "SUCCESS"
    pub status_msg: Option<String>, // "Accepted", "Wrong Answer", "Compile Error"
    pub compile_error: Option<String>,
    pub run_success: Option<bool>,
    pub total_correct: Option<u32>,
    pub total_testcases: Option<u32>,
    pub status_memory: Option<String>,
    pub status_runtime: Option<String>,
    pub expected_output: Option<String>,
    pub code_output: Option<String>,
    pub finished: Option<bool>,
    pub input_formatted: Option<String>,
    pub input: Option<String>,
    pub last_testcase: Option<String>,
    pub memory_percentile: Option<f64>,
    pub runtime_percentile: Option<f64>,
}
