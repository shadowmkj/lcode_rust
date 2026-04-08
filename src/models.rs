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
