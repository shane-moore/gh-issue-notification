use axum::{response::IntoResponse, routing::get, Router};
use chrono::{Duration, Utc};
use dotenv::dotenv;
use reqwest::header;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use tokio;

async fn check_issues() -> Result<impl IntoResponse, String> {
    let github_token = env::var("GITHUB_TOKEN").map_err(|_| "Missing GitHub token")?;
    let telegram_bot_token =
        env::var("TELEGRAM_BOT_TOKEN").map_err(|_| "Missing Telegram bot token")?;
    let telegram_chat_id = env::var("TELEGRAM_CHAT_ID").map_err(|_| "Missing Telegram chat ID")?;

    let three_minutes_ago = (Utc::now() - Duration::minutes(3)).to_rfc3339();
    let client = Client::new();

    let issues: Vec<Issue> = client
        .get("https://api.github.com/repos/paradigmxyz/reth/issues?per_page=10")
        .header(header::USER_AGENT, "Rust Telegram Bot")
        .bearer_auth(github_token)
        .send()
        .await
        .map_err(|e| format!("GitHub request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Parsing failed: {}", e))?;

    let recent_issues: Vec<_> = issues
        .into_iter()
        .filter(|issue| {
            issue.created_at >= three_minutes_ago
                && issue.labels.iter().any(|l| l.name == "D-good-first-issue")
        })
        .collect();

    for issue in recent_issues {
        let telegram_url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            telegram_bot_token
        );

        let text = format!("🚀 New Good First Issue: {}", issue.html_url);

        client
            .post(&telegram_url)
            .json(&serde_json::json!({
                "chat_id": telegram_chat_id,
                "text": text,
            }))
            .send()
            .await
            .map_err(|e| format!("Telegram notification failed: {}", e))?;
    }

    Ok("Issue check complete!".into_response())
}

#[derive(Debug, Deserialize)]
struct Issue {
    html_url: String,
    created_at: String,
    labels: Vec<Label>,
}

#[derive(Debug, Deserialize)]
struct Label {
    name: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let app = Router::new().route("/check-issues", get(check_issues));

    println!("Server running at http://localhost:3000");
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
