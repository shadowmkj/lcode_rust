use crate::auth::LeetCodeCredentials;
use crate::error::{EngineError, Result};
use crate::models::{GraphQLQuery, Question};
use reqwest::Client;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::json;

pub struct LeetCodeClient {
    http_client: Client,
    csrf_token: String,
}

impl LeetCodeClient {
    pub fn new(creds: LeetCodeCredentials) -> Result<Self> {
        let mut headers = HeaderMap::new();

        // Construct the cookie string
        let cookie_str = format!(
            "LEETCODE_SESSION={}; csrftoken={}",
            creds.session_cookie, creds.csrf_token
        );

        headers.insert(COOKIE, HeaderValue::from_str(&cookie_str).unwrap());
        headers.insert(
            "x-csrftoken",
            HeaderValue::from_str(&creds.csrf_token).unwrap(),
        );
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
            ),
        );
        headers.insert("Referer", HeaderValue::from_static("https://leetcode.com/"));

        let client = Client::builder()
            .default_headers(headers)
            .cookie_store(true)
            .build()?;

        Ok(Self {
            http_client: client,
            csrf_token: creds.csrf_token,
        })
    }

    pub async fn execute_graphql<T: serde::de::DeserializeOwned>(
        &self,
        query: GraphQLQuery,
    ) -> Result<T> {
        let response = self
            .http_client
            .post("https://leetcode.com/graphql")
            .json(&query)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(EngineError::Network(
                response.error_for_status().unwrap_err(),
            ));
        }

        let json_data: serde_json::Value = response.json().await?;

        if let Some(errors) = json_data.get("errors") {
            return Err(EngineError::GraphQL(errors.to_string()));
        }

        let data = json_data
            .get("data")
            .ok_or_else(|| EngineError::GraphQL("Missing data field".into()))?;

        serde_json::from_value(data.clone()).map_err(EngineError::from)
    }

    /// Fetch a specific problem's details and boilerplate using its URL slug
    pub async fn get_question_by_slug(&self, title_slug: &str) -> Result<Question> {
        // 1. Define the exact GraphQL query LeetCode expects
        let query_string = r#"
            query questionData($titleSlug: String!) {
                question(titleSlug: $titleSlug) {
                    questionId
                    title
                    titleSlug
                    content
                    codeSnippets {
                        langSlug
                        code
                    }
                }
            }
        "#;

        // 2. Construct the payload with the dynamic slug variable
        let query = GraphQLQuery {
            query: query_string.to_string(),
            variables: json!({ "titleSlug": title_slug }),
            operation_name: Some("questionData".to_string()),
        };

        // 3. Create a temporary wrapper to handle the nested JSON response
        // LeetCode returns: { "data": { "question": { ... } } }
        // Our execute_graphql method strips the "data" layer, so we catch the "question" layer here.
        #[derive(serde::Deserialize)]
        struct QuestionWrapper {
            question: Question,
        }

        // 4. Execute the query and unwrap the final Question struct
        let response: QuestionWrapper = self.execute_graphql(query).await?;

        Ok(response.question)
    }
    /// Fetch a specific problem's details and boilerplate using its numerical ID
    pub async fn get_question_by_id(&self, id: u64) -> Result<Question> {
        let url = "https://leetcode.com/api/problems/all/";

        let response = self.http_client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(EngineError::GraphQL(format!(
                "Failed to fetch problem list: {}",
                response.status()
            )));
        }

        let json_data: serde_json::Value = response.json().await?;
        let mut target_slug = None;

        // Traverse the generic JSON tree to find the matching ID
        if let Some(pairs) = json_data
            .get("stat_status_pairs")
            .and_then(|v| v.as_array())
        {
            for pair in pairs {
                if let Some(stat) = pair.get("stat") {
                    let current_id = stat.get("frontend_question_id").and_then(|v| v.as_u64());

                    if current_id == Some(id) {
                        if let Some(slug) =
                            stat.get("question__title_slug").and_then(|v| v.as_str())
                        {
                            target_slug = Some(slug.to_string());
                            break;
                        }
                    }
                }
            }
        }

        let slug = target_slug
            .ok_or_else(|| EngineError::GraphQL(format!("Problem with ID {} not found", id)))?;

        // Chain directly into the slug fetcher, returning Result<Question>
        self.get_question_by_slug(&slug).await
    }
}
