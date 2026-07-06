//! Step 1: read the OAuth tokens `codex login` already stored, so we can
//! reuse the existing Codex CLI (ChatGPT subscription) session instead of a
//! pay-per-token API key.

use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

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
    let codex_home = env::var("CODEX_HOME").map(PathBuf::from).unwrap_or_else(|_| {
        let home = env::var("HOME").expect("HOME not set");
        PathBuf::from(home).join(".codex")
    });
    let path = codex_home.join("auth.json");
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {} — run `codex login` first", path.display()))?;
    let parsed: AuthDotJson = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(parsed.tokens)
}

fn main() -> Result<()> {
    let tokens = load_codex_tokens()?;
    println!("s01: Agent Loop (Rust, via Codex subscription)");
    println!("account_id = {}", tokens.account_id);
    println!("access_token length = {}", tokens.access_token.len());
    Ok(())
}
