//! Step 7: the actual agent loop — Steps 2-6 wrapped in `loop`, feeding each
//! tool's output back as a `function_call_output` item and re-sending the
//! full `input` history until a turn produces zero function calls. This is
//! the whole lesson of s01: a single-shot call becomes an agent by making
//! this loop instead of returning after one request.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

// `input` is now serialized back into the request too (it's a flat item
// list, appended to turn after turn), so these derive Serialize as well.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentPart {
    InputText {
        text: String,
    },
    OutputText {
        text: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum InputItem {
    Message {
        role: String,
        content: Vec<ContentPart>,
    },
    FunctionCall {
        name: String,
        arguments: String,
        call_id: String,
    },
    FunctionCallOutput {
        call_id: String,
        output: String,
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

fn bash_tool_schema() -> Value {
    json!({
        "type": "function",
        "name": "bash",
        "description": "Run a shell command.",
        "strict": false,
        "parameters": {
            "type": "object",
            "properties": {"command": {"type": "string"}},
            "required": ["command"],
        },
    })
}

const DANGEROUS: [&str; 5] = ["rm -rf /", "sudo", "shutdown", "reboot", "> /dev/"];

async fn run_bash(command: &str) -> String {
    if DANGEROUS.iter().any(|d| command.contains(d)) {
        return "Error: Dangerous command blocked".to_string();
    }

    let child = tokio::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let child = match child {
        Ok(c) => c,
        Err(e) => return format!("Error: {e}"),
    };

    match tokio::time::timeout(Duration::from_secs(120), child.wait_with_output()).await {
        Ok(Ok(output)) => {
            let mut out = String::from_utf8_lossy(&output.stdout).into_owned();
            out.push_str(&String::from_utf8_lossy(&output.stderr));
            let out = out.trim();
            if out.is_empty() {
                "(no output)".to_string()
            } else {
                out.chars().take(50_000).collect()
            }
        }
        Ok(Err(e)) => format!("Error: {e}"),
        Err(_) => "Error: Timeout (120s)".to_string(),
    }
}

struct Config {
    client: Client,
    access_token: String,
    account_id: String,
    model: String,
    instructions: String,
}

/// The core pattern: send `input`, execute any function calls the model
/// asked for, append results, repeat until a turn produces no function calls.
async fn agent_loop(cfg: &Config, input: &mut Vec<InputItem>) -> Result<()> {
    loop {
        let body = json!({
            "model": cfg.model,
            "instructions": cfg.instructions,
            "input": input,
            "tools": [bash_tool_schema()],
            "tool_choice": "auto",
            "parallel_tool_calls": false,
            "reasoning": null,
            "store": false,
            "stream": true,
            "include": [],
        });

        let resp = cfg
            .client
            .post(CODEX_RESPONSES_URL)
            .bearer_auth(&cfg.access_token)
            .header("ChatGPT-Account-Id", &cfg.account_id)
            .header("originator", "codex_cli_rs")
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .context("request to Codex backend failed")?;

        let status = resp.status();
        let text = resp.text().await?;
        if status.as_u16() == 401 {
            bail!(
                "401 Unauthorized — your Codex session may have expired, run `codex login` again"
            );
        }
        if !status.is_success() {
            bail!("Codex API error {status}: {text}");
        }

        let events = parse_sse_events(&text);
        let mut calls_to_run: Vec<(String, String, String)> = Vec::new(); // (name, arguments, call_id)
        let mut final_text = String::new();

        for event in &events {
            match event {
                SseEvent::OutputItemDone { item } => match item {
                    InputItem::FunctionCall {
                        name,
                        arguments,
                        call_id,
                    } => {
                        calls_to_run.push((name.clone(), arguments.clone(), call_id.clone()));
                        input.push(item.clone());
                    }
                    InputItem::Message { content, .. } => {
                        for part in content {
                            if let ContentPart::OutputText { text } = part {
                                final_text.push_str(text);
                            }
                        }
                        input.push(item.clone());
                    }
                    _ => {}
                },
                SseEvent::Failed | SseEvent::Incomplete => {
                    bail!("Codex response did not complete successfully");
                }
                _ => {}
            }
        }

        if calls_to_run.is_empty() {
            if !final_text.is_empty() {
                println!("{final_text}");
            }
            return Ok(());
        }

        for (name, arguments, call_id) in calls_to_run {
            if name == "bash" {
                let command = serde_json::from_str::<Value>(&arguments)
                    .ok()
                    .and_then(|v| v.get("command").and_then(Value::as_str).map(str::to_string))
                    .unwrap_or_default();
                println!("\x1b[33m$ {command}\x1b[0m");
                let output = run_bash(&command).await;
                let preview: String = output.chars().take(200).collect();
                println!("{preview}");
                input.push(InputItem::FunctionCallOutput { call_id, output });
            }
        }
        // loop continues: next iteration re-sends `input`, now including the
        // function_call + function_call_output pair we just appended
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let tokens = load_codex_tokens()?;
    let model = env::var("CODEX_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    let cfg = Config {
        client: Client::new(),
        access_token: tokens.access_token,
        account_id: tokens.account_id,
        model,
        instructions: "You are a coding agent. Use bash to solve tasks.".to_string(),
    };

    // Still one hardcoded query — this step is about the loop, not the REPL
    // (that's Step 8). Pick a prompt that needs more than one tool call.
    let mut input = vec![InputItem::Message {
        role: "user".to_string(),
        content: vec![ContentPart::InputText {
            text: "Create a file called hello.txt containing 'hi', then read it back to me."
                .to_string(),
        }],
    }];

    agent_loop(&cfg, &mut input).await
}
