use crate::structs::LeetCodeProblem;
use regex::Regex;
use reqwest::{
    Client,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue, ORIGIN, REFERER, USER_AGENT},
};
use serde_json::{Value, json};

pub fn extract_slug(url: &str) -> Option<String> {
    let re = Regex::new(r"/problems/([^/]+)").unwrap();
    re.captures(url)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

pub async fn fetch_problem(slug: &str) -> Result<LeetCodeProblem, Box<dyn std::error::Error>> {
    let problem_url: String = format!("https://leetcode.com/problems/{}/", slug);
    let graphql_url: &str = "https://leetcode.com/graphql";

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36")
        .build()?;

    let query: &str = r#"
        query questionData($titleSlug: String!) {
          question(titleSlug: $titleSlug) {
            title
            content
            difficulty
            exampleTestcases
            likes
            dislikes
          }
        }
    "#;

    let body: Value = json!({
        "query": query,
        "variables": { "titleSlug": slug },
        "operationName": "questionData"
    });

    let mut headers: HeaderMap = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(REFERER, HeaderValue::from_str(&problem_url)?);
    headers.insert(ORIGIN, HeaderValue::from_static("https://leetcode.com"));
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36"));

    let resp = client
        .post(graphql_url)
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await?;

    if status.is_success() && text.contains("\"question\"") {
        let data: Value = serde_json::from_str(&text)?;
        let question = &data["data"]["question"];

        Ok(LeetCodeProblem {
            title: question["title"].as_str().unwrap_or("Unknown").to_string(),
            difficulty: question["difficulty"]
                .as_str()
                .unwrap_or("Unknown")
                .to_string(),
            description: question["content"].as_str().unwrap_or("").to_string(),
            example_testcases: question["exampleTestcases"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            likes: question["likes"].as_u64().unwrap_or(0),
            dislikes: question["dislikes"].as_u64().unwrap_or(0),
        })
    } else {
        Err("Не удалось получить задачу".into())
    }
}
