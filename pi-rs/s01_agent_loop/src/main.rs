//! Step 3: parse the raw SSE text from Step 2 into structured events instead
//! of eyeballing a wall of JSON. `#[serde(other)]` is the key trick — it lets
//! the enum skip event/item types we haven't modeled yet (like `reasoning`)
//! instead of failing to parse the whole response.

use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

const CODEX_RESPONSES_URL: &str = "https://chatgpt.com/backend-api/codex/responses";
const DEFAULT_MODEL: &str = "gpt-5.3-codex-spark";

#[derive(Deserialize, Debug)]
struct AuthDotJson {
    tokens: TokenData,
}

#[derive(Deserialize, Debug)]
struct TokenData {
    access_token: String,
    account_id: String,
}

fn load_codex_tokens() -> Result<TokenData> {
    let codex_home = env::var("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("HOME").expect("HOME not set");
            PathBuf::from(home).join(".codex")
        });
    let path = codex_home.join("auth.json");
    let raw = fs::read_to_string(&path).with_context(|| {
        format!(
            "failed to read {} — run `codex login` first",
            path.display()
        )
    })?;
    let parsed: AuthDotJson = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(parsed.tokens)
}

// ── Wire types for the Responses API ──────────────────────────────────────
// Only the "final text answer" shape is modeled so far — no tool calls yet.

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentPart {
    OutputText {
        text: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum InputItem {
    Message {
        role: String,
        content: Vec<ContentPart>,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum SseEvent {
    #[serde(rename = "response.output_item.done")]
    OutputItemDone { item: InputItem },
    #[serde(rename = "response.completed")]
    Completed,
    #[serde(rename = "response.failed")]
    Failed,
    #[serde(rename = "response.incomplete")]
    Incomplete,
    #[serde(other)]
    Other,
}

fn parse_sse_events(text: &str) -> Vec<SseEvent> {
    text.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .filter_map(|data| serde_json::from_str::<SseEvent>(data).ok())
        .collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    let tokens = load_codex_tokens()?;
    let model = env::var("CODEX_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    let body = json!({
        "model": model,
        "instructions": "You are a helpful assistant.",
        "input": [
            {"type": "message", "role": "user", "content": [
                {"type": "input_text", "text": "Say exactly: pong"}
            ]}
        ],
        "tools": [],
        "tool_choice": "auto",
        "parallel_tool_calls": false,
        "reasoning": null,
        "store": false,
        "stream": true,
        "include": [],
    });

    let client = Client::new();
    let resp = client
        .post(CODEX_RESPONSES_URL)
        .bearer_auth(&tokens.access_token)
        .header("ChatGPT-Account-Id", &tokens.account_id)
        .header("originator", "codex_cli_rs")
        .header("Accept", "text/event-stream")
        .json(&body)
        .send()
        .await
        .context("request to Codex backend failed")?;

    let text = resp.text().await?;
    let events = parse_sse_events(&text);
    for event in &events {
        println!("{event:#?}");
    }
    Ok(())
}
