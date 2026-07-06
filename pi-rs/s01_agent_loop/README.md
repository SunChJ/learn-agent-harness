English | [中文](README-zh.md)

# s01: Agent Loop — One Loop Is All You Need

Rust port of [learn-claude-code/s01_agent_loop](../../learn-claude-code/s01_agent_loop/). Independent teaching-replica track, run stage by stage inside `pi-rs/`; see [pi-rs/README.md](../README.md) for how this relates to the milestone-driven rewrite plan in `docs/en/02-rust-rewrite-plan.md`.

`s01` → s02 → s03 → ... → s20 (stages added as we go)

---

## The Problem

You ask the model: "List the files in my directory and run XXX.py."

The model can output a bash command, but once it's done outputting, it stops — it won't execute the command on its own, and it won't keep reasoning based on the result. Automating that hand-off is what this stage builds.

## The Solution

A loop: keep going while the model asks for a tool, stop when it doesn't.

| Signal | Meaning | Loop action |
|---|---|---|
| turn produced ≥1 function-call item | Model raises hand: "I need a tool" | Execute each → feed result back → continue |
| turn produced 0 function-call items | Model says: "I'm done" | Exit loop |

## Why This Isn't the Anthropic Messages API

The Python original and this project's first Rust draft both called the Anthropic Messages API with an `ANTHROPIC_API_KEY`. This version instead reuses an existing **Codex CLI login** (a ChatGPT subscription, not pay-per-token billing) and calls **OpenAI's Responses API** through the same backend Codex CLI itself uses. That changes more than the URL:

- **Auth**: no API key. Codex CLI's OAuth login (`codex login`) already wrote `access_token` and `account_id` to `$CODEX_HOME/auth.json` (default `~/.codex/auth.json`). This stage just reads that file — it does **not** implement the OAuth/PKCE login flow or token refresh itself. If a call comes back `401`, the fix is to run `codex login` again; teaching that properly (proactive refresh, JWT expiry checks) is out of scope here.
- **Endpoint & shape**: `POST https://chatgpt.com/backend-api/codex/responses`, headers `Authorization: Bearer <access_token>`, `ChatGPT-Account-Id: <account_id>`, `originator: codex_cli_rs`. The request body is OpenAI's Responses API shape, not Anthropic's Messages API shape:
  - `input` is a **flat list of items** (not role-grouped `messages`): `{"type":"message","role":"user","content":[{"type":"input_text",...}]}`, `{"type":"function_call","name","arguments","call_id"}`, `{"type":"function_call_output","call_id","output"}`.
  - Tool schema uses `parameters` (Anthropic uses `input_schema`).
  - There's no `stop_reason`. "Done" is signaled implicitly: a turn either produced `function_call` items (keep going) or it didn't (final answer).
- **Streaming is mandatory**: this backend only serves Server-Sent Events (`stream: true` is not optional). To keep this stage's *loop* logic simple, we buffer the entire SSE body and parse it in one pass (`parse_sse_events`) instead of rendering token-by-token — real incremental streaming is `docs/en/02-rust-rewrite-plan.md`'s M2 territory, pulled forward here only as much as the API forces.
- **Lossy history**: reasoning items and other exotic output-item types aren't modeled and get dropped from the conversation history (`InputItem::Other`). Fine for a one-tool demo; a production agent would need to preserve them.

Reverse-engineered from `learn-codex` (the Rust `codex` CLI source) — see `codex-rs/model-provider-info/src/lib.rs`, `codex-rs/tools/src/responses_api.rs`, and `codex-rs/codex-api/src/sse/responses.rs` if you want to trace the wire format yourself.

**Model note**: the default (`gpt-5.5`) is hardcoded to match this account's `model =` entry in `~/.codex/config.toml`. Check your own config or override with `CODEX_MODEL` if your account/plan uses a different one — trying an unsupported model returns a `400` naming what went wrong.

## Try It

**Prerequisite**: `codex login` must already have been run once (this stage doesn't do the login dance itself).

**Run** (from `pi-rs/`):

```sh
cargo run -p s01_agent_loop
```

Try these prompts:

1. `Create a file called hello.py that prints "Hello, World!"`
2. `List all files in this directory`
3. `What is the current git branch?`

Watch for: when does the model call a tool (loop continues), and when does it not (loop ends)?

> **Demo warning**: this executes shell commands the model generates. Run it in a scratch directory. There is no permission system yet — that's a later stage.

## What's Next

The model only has bash right now — reading a file means `cat`, writing means `echo ... >`. s02 gives it real tools.
