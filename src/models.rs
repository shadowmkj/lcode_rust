use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
pub struct GraphQLQuery {
    pub query: String,
    pub variables: Option<serde_json::Value>,
    #[serde(rename = "operationName")]
    pub operation_name: Option<String>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Language {
    Python,
    Rust,
    Pandas,
    Mysql,
    Postgres,
}

#[derive(Debug, Clone)]
pub enum Identifier {
    Number(u64),
    String(String),
}

impl From<String> for Language {
    fn from(value: String) -> Self {
        match value.as_str() {
            "python3" => Self::Python,
            "rust" => Self::Rust,
            "pythondata" => Self::Pandas,
            "mysql" => Self::Mysql,
            "postgresql" => Self::Postgres,
            _ => Self::Mysql,
        }
    }
}

impl Language {
    pub fn to_lang_slug(&self) -> &'static str {
        match self {
            Language::Python => "python3",
            Language::Rust => "rust",
            Language::Mysql => "mysql",
            Language::Pandas => "pythondata",
            Language::Postgres => "postgresql",
        }
    }

    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "py" => Language::Python,
            "rs" => Language::Rust,
            "sql" => Language::Mysql,
            _ => Language::Mysql,
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
    #[serde(rename = "exampleTestcases")]
    pub example_test_cases: String,
    #[serde(rename = "codeSnippets")]
    pub code_snippets: Vec<QuestionSnippet>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProblemSummary {
    pub id: u64,
    pub acceptance: f64,
    pub accepted: u64,
    pub difficulty: u8, // 1 = Easy, 2 = Medium, 3 = Hard
    pub slug: String,
    pub status: Option<String>,
    pub submitted: u64,
    pub title: String,
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Topic {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuestionTopics {
    pub name: String,
    pub id: String,
    pub slug: String,
    #[serde(rename = "translatedName")]
    pub translated_name: Option<String>,
    #[serde(rename = "questionIds")]
    pub question_ids: Vec<u64>,
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

#[derive(Serialize, Debug)]
pub struct TestPayload {
    pub lang: String,
    pub question_id: String,
    pub typed_code: String,
    pub data_input: String,
}

#[derive(Deserialize, Debug)]
pub struct TestSubmitResponse {
    pub interpret_id: String,
    pub test_case: String,
}

#[derive(Deserialize, Debug)]
pub struct SubmitResponse {
    pub submission_id: u64,
}

#[derive(Deserialize, Debug)]
pub struct TestSubmissionCheckResult {
    pub code_answer: Option<Vec<String>>,
    pub code_output: Option<Vec<String>>,
    pub correct_answer: Option<bool>,
    pub expected_code_answer: Option<Vec<String>>,
    pub full_runtime_error: Option<String>,
    pub lang: Option<String>,
    pub memory_percentile: Option<f64>,
    pub run_success: Option<bool>,
    pub runtime_percentile: Option<f64>,
    pub state: String,
    pub status_memory: Option<String>,
    pub status_msg: Option<String>,
    pub status_runtime: Option<String>,
    pub total_correct: Option<u32>,
    pub total_testcases: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct SubmissionCheckResult {
    pub code_output: Option<String>,
    pub compile_error: Option<String>,
    pub expected_output: Option<String>,
    pub finished: Option<bool>,
    pub full_runtime_error: Option<String>,
    pub input: Option<String>,
    pub input_formatted: Option<String>,
    pub last_testcase: Option<String>,
    pub memory_percentile: Option<f64>,
    pub run_success: Option<bool>,
    pub runtime_percentile: Option<f64>,
    pub state: String, // "PENDING", "STARTED", "SUCCESS"
    pub status_memory: Option<String>,
    pub status_msg: Option<String>, // "Accepted", "Wrong Answer", "Compile Error"
    pub status_runtime: Option<String>,
    pub total_correct: Option<u32>,
    pub total_testcases: Option<u32>,
}
