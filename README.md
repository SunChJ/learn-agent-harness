English | [中文](./README-zh.md)

# Learn Agent Harness — Build Your Own Coding Agent in Rust

**A learning path, not a framework.** You start in Rust immediately, study three real agent harnesses — each playing a distinct role — and rebuild the smallest, cleanest one ([pi](https://github.com/badlogic/pi-mono)) in Rust, milestone by milestone. By the end you own three things at once: a working mental model of how coding-agent harnesses actually work, a real Rust codebase you wrote yourself, and the judgment to evaluate any agent framework against implementations that ship.

## Why this exists

An agent product = **model + harness**. The model supplies the intelligence; the harness supplies the loop, the tools, the context management, and the safety rails. Model capability is not something you can build — but the harness is, and it is where almost all engineering decisions in an agent product live.

Most people learn harnesses in one of two inefficient ways: reading one giant codebase top-to-bottom (you drown — Codex is ~1.1M lines of Rust), or following toy tutorials that skip the hard parts (streaming, truncation, compaction) and teach a 2023-era design. This path takes a third route:

> **Build the unified mental model first, then read real implementations against it, then rebuild one of them for real.**

## The four references, four roles

The references are pinned as git submodules — one clone gets you everything (see [Setup](#setup)). Each occupies a distinct niche:

| Repo | Role | What it gives you |
|---|---|---|
| `rust-course/` (Rust, in this repo) | **Textbook** | Small runnable Rust labs for the core harness ideas: Cargo workspace, event streams, the agent loop, tool dispatch, and compaction. Run the matching lab *before* each milestone so the intuition is already in Rust. |
| [pi](https://github.com/badlogic/pi-mono) (TypeScript, 5 packages) | **Spec** | The rewrite target. Hand-written core under ~10k lines, clean injectable seams everywhere, and 33 official design docs. Small enough to hold in your head, real enough to matter. |
| [Codex CLI](https://github.com/openai/codex) (Rust, ~98 crates) | **Industrial reference** | The same problems — loop, tool dispatch, streaming, sandboxing — solved in production Rust. After each milestone, review your code against theirs. |
| Hermes Agent by Nous Research (Python) | **Contrast** | Answers the questions the others don't ask: multi-channel gateway, self-improving memory/skills, cron, ACP. Used to stress-test the completeness of your mental model — skim only. |

**The per-milestone loop:**

```
① Warm up   — run the matching `rust-course` lab (runnable Rust intuition, ~30 min)
② Read      — the pi module + its design doc; write down the design decisions in your own words
③ Build     — implement it in Rust in pi-rs/; make it work first, make it pretty later
④ Compare   — open the Codex code that solves the same problem; review your abstractions
             against theirs (why a trait there? why an enum? what's non-idiomatic in mine?)
⑤ Refactor  — and add a "what the comparison taught me" section to your notes
```

## The curriculum

| Doc | What it is |
|---|---|
| [00 — The Harness Mental Model](docs/en/00-harness-mental-model.md) | The map before the jungle: a harness decomposed into 8 layers (provider → tools → turn loop → session → safety → TUI → extensions → run modes), the design questions each layer must answer, and how all four references answer them. Ends with a self-check quiz. |
| [01 — Cross-Reference Reading Map](docs/en/01-reading-map.md) | What to read, in what order, down to file paths — including what to deliberately *skip* so you don't drown. |
| [02 — The Rust Rewrite Plan (M0–M9)](docs/en/02-rust-rewrite-plan.md) | Ten milestones along pi's dependency spine (ai → agent → tools → session → tui), each with warm-up stage, porting targets, Rust learning goals, Codex comparison points, and acceptance criteria. |
| [03 — Reviewing External Advice](docs/en/03-advice-review.md) | A worked example of fact-checking AI-generated architecture advice against real codebases — what to adopt, what to reject, and how to tell a 2023-era agent design from a current one. |

### Milestones at a glance

| | Milestone | You gain |
|---|---|---|
| M0 | Hello LLM | Cargo workspace, first API call |
| M1 | Message model + event stream | Tagged enums, serde, the internal event protocol |
| M2 | Streaming Anthropic provider | SSE, cancellation, **errors as in-stream events** — the first hard fight |
| M3 | **The agent loop** | The heart. Tool trait, prepare/execute/finalize pipeline. *After M3 you own a real coding agent.* |
| M4 | Built-in tools + truncation | read/edit/write/grep/find/ls; head-vs-tail truncation with actionable hints |
| M5 | Session persistence | Append-only JSONL session *tree*, resume, in-place branching |
| M6 | Compaction | Threshold-triggered history summarization that never splits a tool call from its result |
| M7 | TUI | Two routes: ratatui (fast) or a hand-rolled differential renderer à la pi (deep) |
| M8 | Electives | Second provider, skills, RPC mode, approval + sandboxing, MCP |
| M9 | Harness-OS extensions | Hooks, subagents, plan tool, file-based memory — the test of whether your kernel abstraction actually holds |

M0–M3 form the **kernel**: loop + context assembly + tool dispatch. Everything after is engineering layered on top of it.

## Setup

```bash
# The reference repos come along as pinned submodules:
git clone --recurse-submodules https://github.com/SunChJ/learn-agent-harness.git
cd learn-agent-harness

# (already cloned without submodules? run: git submodule update --init)

# Start in Rust, no API key required:
cargo run -p rust-course --bin m0_hello_rust
cargo run -p rust-course --bin m3_agent_loop
```

Your production rewrite lives in `pi-rs/` — you `cargo init` it yourself in M0; that's part of the exercise. The `rust-course/` crate is only the lab bench.

## Who this is for

- You can program, and want to *actually understand* agent harnesses instead of collecting framework opinions.
- You want to learn Rust through a real project rather than exercises. Prior Rust experience is not required — the milestones sequence the language's difficulty curve deliberately (ownership in M1, async in M2, trait objects in M3, UTF-8 discipline in M4).
- You'd rather spend 2–3 months building durable judgment than 2 days skimming.

## How to start

1. Read [the mental model](docs/en/00-harness-mental-model.md) (~1 hour). Map before jungle.
2. Do "step zero" of [the reading map](docs/en/01-reading-map.md): run the Rust bare-loop lab (~half a day).
3. Start [M0](docs/en/02-rust-rewrite-plan.md).

One rule: after each milestone, write a note ([template](docs/en/notes/TEMPLATE.md)) answering that milestone's design questions **in your own words**. Being able to answer the design questions is the actual deliverable; working code is the by-product.

## Credits

- [pi / pi-mono](https://github.com/badlogic/pi-mono) by Mario Zechner — the rewrite spec.
- [Codex CLI](https://github.com/openai/codex) by OpenAI — the industrial Rust reference.
- [learn-claude-code](https://github.com/shareAI-lab/learn-claude-code) by shareAI-lab — inspiration for the staged teaching structure; this repo now reworks the starting path into Rust labs.
- Hermes Agent by Nous Research — the contrast reference.

## Contributing

Corrections to the reading maps (file paths drift as upstreams evolve), better milestone acceptance criteria, and translations are all welcome. Open an issue or PR.
