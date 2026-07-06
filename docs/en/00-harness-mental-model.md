English | [中文](../zh/00-harness-mental-model.md)

# 00 — The Harness Mental Model

Strip away all the product features, and a coding-agent harness is **a state machine that connects "model sampling" to "side effects in the world."**
This doc breaks it into 8 layers. For each layer: what problem it solves, the design questions you must be able to answer, and how each of the three reference implementations answers them.

**How to use this**: read it end to end once to build the map; then after each rewrite milestone, come back and reread the corresponding layer, and check whether you can answer that layer's design questions on your own.

---

## Layer overview

```
L7  Run modes       interactive / exec(print) / RPC / server / gateway
L6  Extensions      skills / extensions / MCP / hooks
L5  Frontend (TUI)  rendering, keyboard, event protocol decoupled from the engine
L4  Safety          approval policies, sandbox, command safety assessment
L3  Sessions        history storage, resume, branching, compaction
L2  Turn            the sample + tool-call loop, until the model stops calling tools
L1  Tool system     schema definition, dispatch, execution, result truncation
L0  Provider layer  wire protocols, stream event normalization, retries, per-provider compat
```

The core invariant: **L0–L2 is the minimal core no harness can avoid.** pi's `agent` package + `ai` package *is* that minimal core (~44k lines including generated code, under 10k hand-written); codex's `core` crate is the industrial-strength version of the same core; hermes wraps an entire "personal AI operating system" around it.

---

## L0 — Provider layer: normalize N APIs into 1 event stream

**What it solves**: Anthropic Messages, OpenAI Responses, Chat Completions, Gemini… every vendor differs in request format, stream chunking, thinking representation, and cache control. The loop above should know none of this.

**Design questions**:
1. What does the normalized internal event protocol look like? (pi: `AssistantMessageEvent` = `start / text_delta / thinking_delta / toolcall_delta / done / error`, where every event carries a complete partial snapshot)
2. What happens when an error occurs after the stream has started? (pi's answer is beautiful: **never throw once streaming has begun** — errors are encoded as a terminal `error` event, `stopReason: "error"|"aborted"`. Errors become data, and the layer above handles them uniformly)
3. Where do per-provider quirks live? (pi: declared centrally via `*Compat` interfaces; hermes: a declarative `ProviderProfile`; codex: simply supports only the Responses API as its single wire protocol — **cutting away diversity is also an answer**)
4. At which layer do retries live? (codex: `codex-client/src/retry.rs` at the HTTP layer + sticky routing within a turn; pi: SDK-level retries + `maxRetryDelayMs` for fast failure)

**How the three repos answer**:
- pi: `packages/ai/` — 9 API implementations × 39 provider configs, exporting a unified `stream`/`streamSimple`
- codex: `codex-api` (SSE/WS → `ResponseEvent`) + `codex-client` (HTTP/retries) + `core/src/client.rs` (the seam between the loop and the network layer)
- hermes: `providers/base.py` declarative profiles + `agent/*_adapter.py`, one adapter per API

---

## L1 — Tool system: the model's "hands"

**What it solves**: expose JSON-schema-declared capabilities to the model, safely turn the model's call intent into real side effects, then feed the (truncated) results back.

**Design questions**:
1. What is the minimal tool interface? (pi: `name + description + parameters(schema) + execute(id, params, signal, onUpdate, ctx)`; codex: the `ToolExecutor` trait)
2. **Truncation strategy** — tool output can be a 100MB log. How much do you feed the model, how do you cut it, and once cut, how does the model know "there's more"? (pi's `truncate.ts` is textbook: dual line/byte limits, whichever hits first; file reads keep the head, bash output keeps the tail, with an actionable hint like "Use offset=N to continue")
3. Parallel or serial execution? Termination semantics? (pi: each tool can declare an `executionMode`, results are emitted in original order; `terminate: true` can abort the whole batch)
4. What form does the edit tool take? (codex: a homegrown apply_patch free-form patch language + a streaming parser; pi: exact string-replacement edit + diff validation — two roads, each with trade-offs)

**How the three repos answer**:
- pi: `packages/coding-agent/src/core/tools/` (read/bash/edit/write/grep/find/ls — 7 tools)
- codex: `core/src/tools/{registry,router}.rs` + `handlers/` (shell, apply_patch, unified_exec…)
- hermes: `tools/registry.py` self-registration + ~95 tools + toolset grouping (**every API call carries the full core toolset → adding a core tool has an extremely high bar** — this is its "narrow waist" philosophy)

---

## L2 — The turn loop: the heart of the whole harness

**What it solves**: after one user input, the loop of "sample → execute tools → put results back into context → sample again," until the model produces a final answer with no tool calls.

**Design questions (the most important set)**:
1. **Stop condition**: when does a turn end? (All three agree on the core: stop when the assistant message contains no tool call. codex additionally has stop-hooks that can demand continuation; pi has a `shouldStopAfterTurn` injection point)
2. **Steering**: what happens when the user types while the agent is mid-work? (pi has three queues: steering / follow-up / next-turn, with two drain modes, `"all"|"one-at-a-time"` — this is the concurrency model in pi's design most worth savoring)
3. What happens when tokens run over the limit? (codex: the turn loop detects `token_limit_reached` → automatic compaction → continue; see L3)
4. How does the loop talk to the outside world? (All three converge on the same answer: **an event stream**. pi: `agent_start/turn_start/message_update/tool_execution_*/turn_end`; codex: the submission-event architecture of `Op` submissions / `EventMsg` events — the two big enums in `protocol/src/protocol.rs` *are* the complete contract between the engine and every frontend)

**How the three repos answer**:
- pi: `packages/agent/src/agent-loop.ts` — **the single most rewarding file in the entire corpus to read closely**, an inner/outer double loop in ~500 lines
- codex: `core/src/tasks/regular.rs` (outer) + `run_turn` / `run_sampling_request` in `core/src/session/turn.rs` (inner)
- hermes: `agent/conversation_loop.py` (~3900 lines — the most feature-complete, and the hardest to read)

---

## L3 — Sessions: memory and time travel

**Design questions**:
1. What format persists history? (pi: one JSONL file per session, header on the first line, **append-only**; codex: rollout files)
2. Branching / rewind? (pi: each entry has `id + parentId` forming a tree, a `leaf` entry marks the current position — **in-place branching with no file copies**, very elegant; codex: fork/resume)
3. **Compaction**: how do you compress when the context is nearly full? What's the trigger threshold? How do you pick the cut point? (pi: triggers on `contextTokens > window - reserve`, backs off to a turn boundary — **never separate a tool call from its result**; an LLM generates a structured summary and accumulates `<read-files>/<modified-files>` across compactions. codex: `compact.rs` replaces old entries with a SUMMARIZATION_PROMPT summary)
4. How do you count tokens? (codex: a byte-based heuristic, no real tokenizer — engineering pragmatism at its finest)
5. hermes's unique answer: **the prompt cache is sacred** — nothing may rewrite an already-cached prefix, no mid-flight toolset swaps or system prompt rebuilds; the sole exemption is compaction. This is the first principle of long-session cost control, worth carving into your mental model.

**How the three repos answer**:
- pi: `packages/agent/src/harness/session/` + `coding-agent/docs/session-format.md` (read the doc first)
- codex: `core/src/context_manager/history.rs` + `compact.rs` + the `rollout` crate
- hermes: `hermes_state.py` (SQLite + FTS5 full-text search, cross-session search)

---

## L4 — Safety: approval and sandboxing

pi's answer at this layer is "**none**" — it explicitly declines to build a permission system; use a container instead (`docs/containerization.md`). codex is heaviest here, and is the main thing to learn from at this layer:

1. Policy vocabulary: `AskForApproval` (UnlessTrusted / OnRequest / Granular / Never) × `SandboxPolicy` (ReadOnly / WorkspaceWrite / DangerFullAccess) — `protocol/src/protocol.rs`
2. The decision function: `core/src/safety.rs` combines the two policies and outputs `AutoApprove / AskUser / Reject`
3. OS-level enforcement: macOS seatbelt (`.sbpl` policy files), Linux landlock/bwrap — `sandboxing/src/manager.rs`

**Key insight**: approval (whether to ask the user) and sandboxing (OS-enforced isolation) are two orthogonal axes; good design lets them compose independently.

---

## L5 — Frontend: TUI and decoupling

**Design questions**:
1. How do you decouple engine from UI? (Everyone's answer is L2's event protocol: the UI only subscribes to the event stream. codex's `protocol` crate lets the same engine drive TUI / exec / MCP-server / IDE frontends)
2. Rendering architecture? Two routes:
   - codex: **ratatui** (an immediate-mode full-frame redraw framework) + crossterm, with the `tokio::select!` event loop in `tui/src/app.rs`
   - pi: **a hand-written differential renderer** (`tui/src/tui.ts`): components implement `render(width) -> string[]`, a line-by-line diff rewrites only changed lines, wrapped in synchronized output mode `\x1b[?2026h` to avoid flicker — if you want to truly understand terminal rendering, reading pi's 1700 lines teaches you more than using a framework
3. How do you render streaming text? (codex: incrementally mutate the current history cell then redraw; pi: every delta event carries a complete partial snapshot, so components just re-render)

---

## L6 — Extensions

| Mechanism | pi | codex | hermes |
|---|---|---|---|
| Skills (SKILL.md, the agentskills.io standard) | ✅ first-class | ✅ | ✅ and can **autonomously create / self-improve** skills |
| In-process plugins | ✅ TS extensions (register tools/commands/providers/UI) | plugins/hooks | ✅ Python plugins |
| MCP | ❌ (extensions instead) | ✅ client + server, both directions | ✅ client + server |

**Key insight**: the essence of skills is "inject capability descriptions into the system prompt, `read` the full text on demand" — extremely cheap, discoverable by the model on its own. All three agree on this. In-process plugins vs. MCP is the trade-off between "trusting embedded code" and "process-isolated protocol."

---

## L7 — Run modes

- pi: interactive (TUI) / print / **RPC mode** (JSONL over stdio; the orchestrator uses it to coordinate multiple agents)
- codex: TUI / `codex exec` / app-server (JSON-RPC for IDEs) / MCP server
- hermes: CLI / TUI / **a resident gateway daemon** connected to ~21 chat platforms (telegram/discord/WeChat…) / cron scheduling / ACP

**Key insight**: once L2's event protocol is defined cleanly, a run mode is just "swap in a different event consumer." hermes proves the same core can stretch from the terminal all the way to Telegram.

---

## Self-check

After finishing all milestones, you should be able to answer these without looking at the code:

1. Why encode streaming errors as events rather than exceptions?
2. Why must compaction cut at a turn boundary? What happens if you separate a tool call from its result?
3. Where in the loop are steering messages injected? Why do you need multiple queues?
4. Why does tool output truncation keep the head in some scenarios and the tail in others?
5. Why should approval policy and sandbox policy be designed orthogonally?
6. Why did all three converge on "the engine emits events, the frontend subscribes"?
7. How do prompt-cache constraints shape your design of features like "change the system prompt mid-session / swap the toolset"?
8. If you had to add a Telegram entry point to your harness tomorrow, would you touch any L0–L2 code? (The correct answer should be: no)
