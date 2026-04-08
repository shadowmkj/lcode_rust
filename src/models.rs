use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
pub struct GraphQLQuery {
    pub query: String,
    pub variables: serde_json::Value,
    #[serde(rename = "operationName")]
    pub operation_name: Option<String>,
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
    pub title: String,
    pub content: String,
    #[serde(rename = "codeSnippets")]
    pub code_snippets: Vec<QuestionSnippet>,
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
}
