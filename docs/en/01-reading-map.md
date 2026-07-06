English | [中文](../zh/01-reading-map.md)

# 01 — Three-Repo Comparative Reading Map

Organized by concept. For each concept: which files to read, in what order, and what questions to answer while reading.
All paths are relative to each repo's root. **The principle: run the matching learn-claude-code stage first (intuition), then read pi (small and clean), then read codex with questions in hand (the industrial answer). Only skim hermes where explicitly marked.**

---

## Step zero: get the bare loop running (half a day, hands-on, no source reading)

Before reading any real codebase, get the harness's "first principles" running in your own hands:

```bash
cd learn-claude-code && pip install -r requirements.txt
ANTHROPIC_API_KEY=... python s01_agent_loop/code.py   # a complete agent in 137 lines
python s02_tool_use/code.py                            # add tool dispatch
```

Understand these ~300 lines plus the two stage READMEs. Everything you see later in pi/codex is engineering decisions layered on top of this bare loop. **The entire secret of a harness is one line: `while stop_reason == "tool_use"`** — make that line true in your own hands first, then go see how others turned it into a product.

### learn-claude-code stages ↔ this curriculum

| Stage | Topic | Layer / milestone |
|---|---|---|
| s01 agent_loop / s02 tool_use | bare loop + tool dispatch | L1–L2 / warm-up before M3 |
| s03 permission | approval | L4 / M8 |
| s04 hooks / s05 todo_write | hooks, planning tool | L6 / M9 |
| s06 subagent | subagents | L7 / M9 |
| s07 skill_loading | skills | L6 / M8 |
| s08 context_compact | compaction | L3 / M6 |
| s09 memory | file-based memory | L3 / M9 |
| s10 system_prompt | system prompt assembly | L2 / M3 |
| s11 error_recovery | error recovery | L0 / M2 |
| s12–s14 task/background/cron | task system | L7 / M9 (compare against hermes) |
| s15–s17 teams/autonomous | multi-agent | beyond scope; optional after finishing the curriculum |
| s18 worktree_isolation | isolation | L4 / M9 |
| s19 mcp_plugin | MCP | L6 / M8 |
| s20 comprehensive | capstone | full review |

---

## First pass: the trunk line (2–3 days recommended, read-only)

Goal: walk the L0–L2 minimal core once, end to end, and build a complete mental trace of "what actually happens after one user input."

| Step | Read | Answer |
|---|---|---|
| 0 | learn-claude-code `s01` + `s02` (see step zero above) | What does the bare loop look like? What's the stop condition? |
| 1 | pi `README.md` + `AGENTS.md` + `packages/coding-agent/docs/index.md` | What's the author's design philosophy? What does "small core, extended via TypeScript" mean concretely? |
| 2 | pi `packages/ai/src/types.ts` (only `Message`, content blocks, `AssistantMessageEvent`) | What does the internal message model look like? Why does every streaming event carry a complete partial? |
| 3 | pi `packages/agent/src/agent-loop.ts` — **read line by line** | What do the inner and outer loops each own? When does a turn end? What do the prepare/execute/finalize phases of a tool call each do? |
| 4 | pi `packages/agent/src/types.ts` (`AgentMessage`, event types) | Why is `AgentMessage` a superset of the LLM `Message`? Why does the `convertToLlm` injection point exist? |
| 5 | codex `core/src/tasks/regular.rs` + the first ~450 lines of `core/src/session/turn.rs` | The same loop — what does the Rust version look like? How does the `Op`/`EventMsg` submission-event architecture compare to pi's EventStream? |
| 6 | codex `protocol/src/protocol.rs` (browse the `Op` and `EventMsg` enums) | How many message kinds make up the complete engine↔frontend contract? |

When you're done, write your first note: in your own words plus one diagram, describe "the complete data flow after the user presses Enter."

---

## Second pass: by topic (alongside the rewrite milestones — read as you build)

### Topic A: Provider layer and streaming (with M1–M2)

- pi `packages/ai/src/utils/event-stream.ts` — a hand-written async event stream, ~a hundred lines; in the Rust rewrite you'll have to decide whether to reproduce it with channels or a `Stream` trait
- pi `packages/ai/src/api/anthropic-messages.ts` — your first porting target
- pi `packages/ai/src/api/openai-responses.ts` + the `*Compat` interfaces in `types.ts` — get a feel for the complexity of the per-provider compat matrix (don't port it yet)
- codex `process_responses_event` in `codex-api/src/sse/responses.rs` — mapping raw SSE JSON → internal events, and how Rust expresses it with enum + match
- codex `codex-client/src/retry.rs` — a clean retry/backoff implementation
- codex the big comment at the top of `core/src/client.rs` — the seam design between the loop and the network layer

Questions: how do errors flow through this layer? On reconnect/retry, what happens to the half of the stream you've already received?

### Topic B: Tool system (with M3–M4)

- pi `packages/coding-agent/src/core/tools/truncate.ts` — **read this first**: dual-limit truncation + actionable hints
- pi `tools/read.ts` (the definition-pattern template) → `tools/bash.ts` (streaming output, timeouts, spilling overflow to disk) → `tools/edit.ts`
- pi `tools/tool-definition-wrapper.ts` — how ToolDefinition relates to the core AgentTool
- codex `core/src/tools/registry.rs` (trait design) + `router.rs` (dispatch) + `handlers/shell.rs`
- codex `apply-patch/src/parser.rs` — the free-form patch language (compare with pi's exact-replacement edit; weigh the trade-offs)
- hermes, one skim: the module-level self-registration + AST discovery mechanism in `tools/registry.py`

Question: how do you design a tool trait in Rust that supports both schema generation and async execution with progress callbacks?

### Topic C: Sessions and compaction (with M5–M6)

- pi `packages/coding-agent/docs/session-format.md` + `docs/compaction.md` — **docs before code**
- pi `packages/agent/src/harness/session/jsonl-storage.ts` + `buildSessionContext` in `session.ts`
- pi `packages/agent/src/harness/compaction/`
- codex `core/src/context_manager/history.rs` + `core/src/compact.rs` + `session/token_budget.rs`
- hermes, one skim: the opening of `hermes_state.py` (the SQLite+FTS5 alternative route), plus the prompt-cache-sacred principle in AGENTS.md

Questions: how do you rebuild leaf→root from a tree-shaped history? How does the compaction summary accumulate state across compactions?

### Topic D: TUI (with M7)

- pi `packages/coding-agent/docs/tui.md` → `packages/tui/src/tui.ts` (the differential rendering core)
- pi `packages/tui/src/stdin-buffer.ts` + `keys.ts` — raw terminal input parsing
- codex `tui/src/app.rs` (the `tokio::select!` event loop) + `chatwidget.rs` (how streaming enters the scrollback)
- Decision point: does your Rust version use ratatui (the codex route — fast) or a hand-written differential renderer (the pi route — you learn more)? M7 has a recommendation.

### Topic E: Safety (read-only, mostly codex)

- codex `AskForApproval` / `SandboxPolicy` in `protocol/src/protocol.rs` → `core/src/safety.rs` → `sandboxing/src/manager.rs` + any one `.sbpl` file
- pi `packages/coding-agent/docs/containerization.md` — the case for *not* building a permission system, as the counterargument

### Topic F: Extensions and run modes (with M8, optional)

- pi `docs/skills.md` + `packages/coding-agent/src/core/skills.ts` — the skills mechanism is actually quite thin
- pi `docs/extensions.md` + `src/core/extensions/types.ts` — the full shape of an in-process plugin API
- pi `docs/rpc.md` + `modes/rpc/` — the headless JSONL-over-stdio mode
- codex `mcp-server/src/message_processor.rs` — exposing yourself as an MCP server
- hermes, one skim: `gateway/run.py` (the multi-channel resident daemon), `cron/scheduler.py`, `agent/curator.py` (self-improving memory) — a test of your L7 understanding

---

## What to explicitly skip (so you don't drown)

- codex: the entire `app-server*` family, `cloud-*`, `otel`/`analytics`, `realtime-*`/`webrtc`, multi-agent orchestration, giant test files (`session/tests.rs` is 11k lines)
- pi: `models.generated.ts` (generated code), all 39 providers except anthropic/openai, the `orchestrator` package
- hermes: everything except the skim points marked above; do not read `cli.py` (741KB) or `run_agent.py` (269KB) end to end
