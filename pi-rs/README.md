English | [中文](README-zh.md)

# pi-rs

Two independent tracks live here:

1. **The milestone rewrite** (M0-M9, planned in `../docs/en/02-rust-rewrite-plan.md`): the actual production-shaped rewrite of `pi` in Rust, following pi's package spine (`ai → agent → tools → session → tui`). Not started yet.
2. **`sNN_*` staged folders** (this README's subject): a teaching replica of [learn-claude-code](../learn-claude-code/)'s 20-stage curriculum, one crate per stage, each making real model calls (via an existing Codex CLI / ChatGPT-subscription login, not a pay-per-token API key — see each stage's README for why). These are exploratory and run independently of the M0-M9 plan — ideas that prove out here may later get productized into the M0-M9 crates, but nothing here is required to be.

Each stage is its own Cargo package (own `Cargo.toml`, own `README.md`/`README-zh.md`), mirroring how each `learn-claude-code/sNN_*` folder is self-contained.

## Stages

| Stage | Topic | Status |
|---|---|---|
| [s01_agent_loop](s01_agent_loop/) | The core `while tool_use` loop | done |

Run any stage from `pi-rs/`:

```sh
cargo run -p <stage_name>
```

## `AGETNS.md`

The `AGETNS.md` file in this directory was copied from `learn-pi`'s `AGENTS.md` and describes rules for that TypeScript/npm project (packages, changelogs, release process) — it doesn't apply here. Leaving it as-is until it's either replaced with pi-rs-specific rules or removed.
