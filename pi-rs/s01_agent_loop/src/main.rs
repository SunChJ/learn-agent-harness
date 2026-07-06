//! s01_agent_loop — the entire secret of a coding agent in one pattern:
//!
//!     while has_tool_calls {
//!         response = LLM(input, tools)
//!         execute tools
//!         append results
//!     }
//!
//! Rust port of learn-claude-code/s01_agent_loop/code.py, but calling OpenAI's
//! Responses API through an existing Codex CLI (ChatGPT subscription) login
//! instead of a pay-per-token Anthropic API key. Auth: reads the OAuth tokens
//! `codex login` already stored at `$CODEX_HOME/auth.json` (default
//! `~/.codex/auth.json`) — this stage does not implement the OAuth flow or
//! token refresh itself; see README.md for why.
//!
//! Step 8/8: wraps agent_loop() (Step 7) in a REPL — `input` now persists
//! across turns instead of being rebuilt per-call, so the model keeps
//! context from earlier questions in the same session.

use std::env;
use std::fs;
use std::io::{self, Write};
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

// ── Wire types for the Responses API ──────────────────────────────────────
// Flat "input" item list (not role-grouped messages like Anthropic). One
// enum covers both what we serialize into the request and what we parse out
// of `response.output_item.done` events.

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
    // reasoning items, local_shell_call, etc. — not modeled, dropped from
    // history. Fine for a single bash tool demo; would need handling to
    // preserve reasoning continuity in a production agent.
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

/// The endpoint always streams SSE; we buffer the whole body and parse it in
/// one pass rather than rendering incrementally (true streaming is s02+/M2
/// material, not this stage's concern).
fn parse_sse_events(text: &str) -> Vec<SseEvent> {
    text.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .filter_map(|data| serde_json::from_str::<SseEvent>(data).ok())
        .collect()
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
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let tokens = load_codex_tokens()?;
    let model = env::var("CODEX_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let cwd = env::current_dir()?;
    let instructions = format!(
        "You are a coding agent at {}. Use bash to solve tasks. Act, don't explain.",
        cwd.display()
    );

    let cfg = Config {
        client: Client::new(),
        access_token: tokens.access_token,
        account_id: tokens.account_id,
        model,
        instructions,
    };

    println!("s01: Agent Loop (Rust, via Codex subscription)");
    println!("输入问题，回车发送。输入 q 退出。\n");

    let stdin = io::stdin();
    let mut input: Vec<InputItem> = Vec::new();

    loop {
        print!("\x1b[36ms01 >> \x1b[0m");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break; // EOF (Ctrl-D)
        }
        let query = line.trim();
        if query.is_empty() || query.eq_ignore_ascii_case("q") || query.eq_ignore_ascii_case("exit")
        {
            break;
        }

        input.push(InputItem::Message {
            role: "user".to_string(),
            content: vec![ContentPart::InputText {
                text: query.to_string(),
            }],
        });

        if let Err(e) = agent_loop(&cfg, &mut input).await {
            eprintln!("Error: {e}");
        }
        println!();
    }

    Ok(())
}
