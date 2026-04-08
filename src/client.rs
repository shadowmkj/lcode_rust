use crate::auth::LeetCodeCredentials;
use crate::error::{EngineError, Result};
use crate::models::GraphQLQuery;
use reqwest::Client;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, USER_AGENT};

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
}
