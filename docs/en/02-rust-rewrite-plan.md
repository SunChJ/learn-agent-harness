English | [中文](../zh/02-rust-rewrite-plan.md)

# 02 — pi-rs: A Milestone-Driven Rust Rewrite Plan

Rewrite pi in Rust under `pi-rs/`. The milestones follow the spine of pi's package dependencies: **ai → agent → tools → session → tui**.
Each milestone includes: a Rust warm-up lab, a goal, what to port (files in pi), Rust learning points, codex cross-references, and acceptance criteria.

**The kernel framing** (a useful mental anchor): M0–M3 together form the harness's "minimal kernel" — **loop + context assembly + tool dispatch**, those three things. Everything after M4 (truncation, sessions, compaction, TUI, permissions) is an engineering decision layered on top of the kernel. Once M3 passes acceptance, the kernel stands.

**Ground rules**:
- pi is a spec, not scripture — when you hit TS-specific idioms (declaration merging, duck-typed injection), stop and think through the Rust equivalent (traits, generics, enums). That's exactly where the learning happens.
- Before starting each milestone, run the matching `rust-course` lab first. For advanced topics without a lab yet, write a 50–150 line Rust spike in `rust-course` before productizing it in `pi-rs/`.
- At the end of each milestone, run `cargo clippy && cargo fmt && cargo test` and write a note (template in `notes/TEMPLATE.md`).
- Don't abstract prematurely. pi itself was "make it run first, abstract later."

**Pitfalls to dodge up front** (hard-won consensus — every AGENTS.md and every piece of outside advice points at the same list):
- ❌ Building multi-agent from day one — before the kernel stands, it will inevitably spin out of control (don't touch it before M9)
- ❌ Hoarding tools — M3 needs exactly one (bash), and M4 only has 7; 2–3 tools are enough to validate every abstraction
- ❌ Reaching for embedding / RAG / vector memory too early — real harness memory is "compacted message history + file-based memory," not a vector store
- ❌ Skipping streaming — streaming is a first-class hard problem (M2 is a hard fight); skip it and what you build is a toy

Recommended base dependencies: `tokio` (async runtime), `reqwest` (HTTP + streaming), `serde`/`serde_json`, `thiserror` + `anyhow` (errors), `schemars` (JSON schema generation, the typebox counterpart), `clap` (CLI), `crossterm` (terminal).

---

## M0 · Hello LLM (1–3 days)

**Goal**: cargo workspace skeleton + one non-streaming Anthropic Messages API call, printing the reply.

- Structure: start with a single `pi-ai` crate in the workspace (mirroring pi's `packages/ai`); later milestones add crates one at a time, reproducing pi's package boundaries.
- **Rust learning points**: cargo workspace, `Result`/`?`, serde derive, tokio `#[tokio::main]`, reqwest basics.
- **Acceptance**: `cargo run -p pi-ai --example hello` gets a reply from Claude; the API key is read from an environment variable.

## M1 · Message Model + Event Stream (3–7 days)

**Goal**: port pi-ai's core type system and `EventStream`.

- What to port: from `packages/ai/src/types.ts` — `Message` (User/Assistant/ToolResult), content blocks (Text/Thinking/Image/ToolCall), `Usage`, `StopReason`, `AssistantMessageEvent`; plus `utils/event-stream.ts`.
- Key design decisions (make them yourself; record the reasoning in your notes):
  - TS tagged union → Rust `enum` + `#[serde(tag = "type")]`
  - `EventStream` → wrap a `tokio::sync::mpsc` channel, or implement `futures::Stream`? (Suggestion: channel first, reassess before M7)
- **Rust learning points**: modeling with enums, serde tagged enums, how ownership plays out in message passing, when `Arc` is actually needed.
- **codex cross-reference**: `ResponseItem`/`EventMsg` in `protocol/src/protocol.rs` — see how industrial-grade Rust expresses the same kind of protocol with enums.
- **Acceptance**: type round-trip tests (serialize → deserialize == original); one consumer test against a mock event stream.

## M2 · Streaming Anthropic Provider (1–2 weeks, the first hard fight)

**Rust warm-up**: `cargo run -p rust-course --bin m2_stream_errors` (how errors get encoded into the stream instead of blowing up).
**Goal**: call Anthropic over SSE, translate raw events into M1's `AssistantMessageEvent`, including incremental assembly of tool-call fragments.

- What to port: `packages/ai/src/api/anthropic-messages.ts` (including the partial-JSON assembly logic).
- Core discipline: **once the stream starts, never return Err** — errors are encoded as a terminal `error` event (pi's in-stream error encoding), and `stopReason` becomes `Error/Aborted`.
- Define the provider trait (pi's `ProviderStreams` counterpart): `fn stream(&self, ctx: Context) -> AssistantMessageEventStream`.
- **Rust learning points**: the deep end of async (line-by-line SSE parsing, `tokio::select!`, cancellation / `AbortSignal` → `CancellationToken`), layered error design with `thiserror`, trait object vs generics.
- **codex cross-reference**: `process_responses_event` in `codex-api/src/sse/responses.rs`, and `codex-client/src/retry.rs`.
- **Acceptance**: streamed replies print character by character; with a fake tool wired in, a complete `ToolCall` gets assembled; network loss / an invalid key produces an `error` event, not a panic; `Ctrl+C` yields `aborted`.

## M3 · Agent Loop (1–2 weeks, the cognitive core of the whole project)

**Rust warm-up**: `cargo run -p rust-course --bin m3_agent_loop` (if you already ran it in step zero, just reread the loop in `rust-course/src/lib.rs`).
**Goal**: port `agent-loop.ts`; create the `pi-agent` crate.

- What to port: `packages/agent/src/agent-loop.ts` (the nested inner/outer loop, the prepare/execute/finalize three-stage tool pipeline, serial/parallel execution, `terminate` semantics) + `AgentMessage` and the agent event protocol from `types.ts`.
- Define the tool trait (pi's `AgentTool` counterpart): schema (generated with `schemars`) + `async fn execute(id, params, cancel, on_update) -> AgentToolResult`.
- Implement just one `bash` tool first (`tokio::process::Command`, streamed output, timeouts) to punch through the whole pipeline end to end.
- Reproduce the injection points with generics/closure fields: `stream_fn`, `convert_to_llm`, `should_stop_after_turn`.
- **Rust learning points**: trait objects (`Box<dyn Tool>`), async traits, parallel execution with `JoinSet`, and the classic borrow-checker conflict of mutating history inside a loop (you will hit it — hitting it is the lesson).
- **codex cross-reference**: `run_turn` in `core/src/session/turn.rs` + `core/src/tools/{registry,router}.rs` + `parallel.rs`.
- **Acceptance**: one line of CLI input → agent calls the bash tool → watch the full "sample → execute → feed back → sample again → stop on plain text" cycle; agent events print on stderr. **At this point, you own a real coding agent.**

## M4 · The Full Built-in Tool Family + Truncation (1 week)

**Goal**: read / edit / write / grep / find / ls + the truncation layer.

- **Port `tools/truncate.ts` first**: dual line/byte limits with first-hit-wins truncation, UTF-8 boundary safety, keep-head vs keep-tail, the "Use offset=N to continue" hint. Give it your most exhaustive unit tests (this is the best module for practicing Rust testing).
- For edit, take pi's exact-string-replacement route (the apply_patch language can be a later elective — compare codex's `apply-patch` crate).
- **Rust learning points**: file I/O, `&str`/`String`/bytes and UTF-8 boundary handling (Rust is far stricter than TS here — which is exactly the value), path handling.
- **Acceptance**: complete a real small task with your own agent (e.g. "read this file and fix a bug"); the truncation module's tests cover the edge cases (extremely long lines, multi-byte character cut points, empty files).

## M5 · Session Persistence (1 week)

**Goal**: JSONL tree-structured session storage + resume.

- What to port: read `docs/session-format.md` first, then port `harness/session/jsonl-storage.ts` (the `SessionStorage` trait, append-only, `getPathToRoot`, leaf markers) + `buildSessionContext` (rebuilding context leaf→root).
- **Rust learning points**: a trait as the storage abstraction (a Jsonl impl + an in-memory impl), serde over polymorphic entry types, file append writes.
- **Acceptance**: `--continue` resumes the last session and keeps the conversation going; manually moving the leaf to an earlier node makes new messages correctly form a branch; both storage implementations pass the same trait-level test suite.

## M6 · Compaction (3–7 days)

**Rust warm-up**: `cargo run -p rust-course --bin m6_compaction` (the minimal runnable version of the compaction concept).
**Goal**: automatic context compaction.

- What to port: the spec in `docs/compaction.md` + `harness/compaction/`. Key points: threshold trigger (`contextTokens > window − reserve`), falling back to a turn boundary when picking the cut point, **never split a tool call from its result**, LLM-generated structured summaries, `<read-files>/<modified-files>` accumulating across compactions.
- For token estimation, start with a byte heuristic (the same pragmatic move codex makes — `context_manager/history.rs`).
- **Acceptance**: construct an oversized session that triggers automatic compaction, and the agent can still cite key facts from early on afterwards; cut-point tests prove a tool call and its result always land on the same side.

## M7 · TUI (2–3 weeks, optional deep dive)

**Goal**: an interactive interface. Two routes — pick one:

| Route | What you learn | Recommendation |
|---|---|---|
| A: ratatui + crossterm (the codex route) | A usable UI fast; Rust ecosystem integration | Pick this if you want to reach M8 quickly |
| B: hand-rolled diff renderer (the pi route, porting `tui/src/tui.ts`) | The terminal protocol itself: line-by-line diffing, synchronized output mode, cursor control, raw input parsing | Pick this if you want to truly understand terminals — pi's 1700 lines are the best textbook there is |

- Whichever route you take, the architecture is the same: **the UI only subscribes to M3's agent event stream**, with zero changes to the engine — this is a direct test of the L5 decoupling insight.
- Minimal feature set: streamed rendering of assistant text (markdown can wait), tool-call display, an input box, Esc to interrupt, steering (input queues up while the agent is working — go back and retrofit pi's three-queue model into the Agent wrapper layer from M3).
- **codex cross-reference**: the event loop structure in `tui/src/app.rs`.

## M8 · Elective Pieces (2–5 days each, pick by interest)

- **A second provider (OpenAI)**: the real test of whether M2's trait abstraction holds up — an abstraction is only validated when the second implementation shows up.
- **Skills** (first write a `rust-course` spike that discovers `SKILL.md` files and injects their descriptions): the mechanism is thin (discover SKILL.md → inject name + description into the system prompt → the model `read`s it on its own), the payoff is huge.
- **RPC / print mode**: JSONL over stdio — a test of whether your event protocol is complete.
- **Approval + sandbox** (first write an `Allow/Ask/Reject` decision-function spike in `rust-course`; pi doesn't have this — learn it from codex): the decision function in `safety.rs` + wrapping the bash tool in macOS seatbelt — landing the L4 insight in code.
- **MCP client** (first write a minimal stdio JSON-RPC spike): use the official rust-sdk (rmcp), cross-referencing codex's `rmcp-client`.

## M9 · Harness OS Extensions (Rust-spike track, elective, 3–7 days each)

Once the pi-rs kernel is done, go beyond what pi covers using the same **Rust spike → productize in pi-rs** rhythm. The point of this step: it tests whether your kernel abstractions can actually carry "layering on top" — if adding these features requires changing M3's loop code, your kernel abstraction is broken.

- **Hooks** (Rust spike: an interception trait around tool calls; cross-reference codex's `hooks` crate): interception points around tool calls. You already have `beforeToolCall/afterToolCall` injection points from M3 — this step productizes them into a user-configurable mechanism.
- **Todo / planning tool** (Rust spike: a write-only, executes-nothing tool): pure context engineering — how a write-only tool dramatically improves multi-step task performance.
- **Subagent** (Rust spike: wrap `run_agent_loop` as a tool; cross-reference pi's RPC mode + the orchestrator package): expose your own agent as a tool to your own agent. The ultimate test of event-protocol completeness.
- **File-based memory** (Rust spike: scan a memory directory and inject an index; cross-reference hermes `agent/memory_manager.py`): note that memory in real systems is "files + an index injected into the system prompt," not a vector store.
- **Background tasks / worktree isolation** (Rust spike: task state machine + workspace policy): task lifecycle management.
- If you get all the way here and still want more: multi-agent teams and hermes's gateway are the next horizon — but that's a new project, not part of this learning effort.

---

## Progress Tracking

| Milestone | Status | Notes | Completed |
|---|---|---|---|
| M0 Hello LLM | ⬜ | | |
| M1 Message model + event stream | ⬜ | | |
| M2 Anthropic streaming | ⬜ | | |
| M3 Agent Loop | ⬜ | | |
| M4 Tools + truncation | ⬜ | | |
| M5 Sessions | ⬜ | | |
| M6 Compaction | ⬜ | | |
| M7 TUI | ⬜ | | |
| M8 Electives | ⬜ | | |
| M9 Harness OS extensions | ⬜ | | |
