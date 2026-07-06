English | [中文](../zh/03-advice-review.md)

# 03 — Reviewing Outside Advice: Mini-Harness OS v0

A piece of external AI-generated advice (the "Mini-Harness OS v0" four-phase roadmap + Rust skeleton) was once a reference input to this curriculum.
This document reviews it point by point: **what we adopted, what we corrected, and why**. The review is itself a cognitive exercise — fact-checking advice against real codebases matters more than accepting any single source's narrative.

## Verdict First

The advice's **methodology is largely sound** (unify abstractions before comparing implementations, kernel first, a pitfall checklist), and has been absorbed into this curriculum.
But its **facts are wrong in several places** (its characterizations of the three codebases are misattributed), and its **technical design is 2023-era** (text-based action protocol, no streaming, toy memory). Copy it verbatim and you'll learn an obsolete harness.

---

## ✅ What We Adopted

| Claim | Disposition |
|---|---|
| "Unify the abstraction first, then compare implementations — studying source repos one by one is inefficient exploration" | **Completely correct.** This is the entire reason docs/00's cognitive framework exists: build the 8-layer map first, then enter the code jungle. |
| Kernel = loop + context builder + tool router; establish the kernel first | Correct, and a good mental anchor — written into the opening of docs/02 (= our M0–M3). |
| Pitfalls: no multi-agent from day one / no tool hoarding / no premature embeddings | Matches the actual evolution of pi, codex, and hermes; written into docs/02's "pitfalls to dodge up front." |
| A week-by-week cadence | Kept in spirit (the milestone system), but our estimates come from the real complexity of real modules, not a made-up three weeks. |

## ❌ What We Corrected

### 1. Its characterizations of the three codebases are wrong

| The advice's claim | The facts (after exploring all three codebases) |
|---|---|
| "The Pi direction = memory system / embedding memory / persona" | pi has **no** embedding memory and no persona system. pi is a minimalist coding harness: loop, tools, JSONL session tree, compaction. "Memory/persona" is **hermes** territory (`agent/memory_manager.py`, the curator, Honcho user modeling). |
| "The Hermes direction = task graph / DAG planner" | hermes's core has **no** DAG planner. What makes it distinctive is the multi-channel gateway (21 chat platforms), self-improving skills, cron, ACP. |
| "The Codex direction = file graph / patch diff" | codex has no "file graph." Its crown jewels are the sandbox/approval system (seatbelt/landlock + `safety.rs`), the submission-event protocol, and the apply_patch language. "Patch diff" is only half right. |

**The lesson**: characterizations made without reading the source will be confidently wrong. This is exactly why this curriculum insists on "explore the real codebases first → then set the route."

### 2. The technical design is obsolete (the most important correction)

- **Text-based action protocols are dead.** The core of the advice is a "Prompt Protocol unified action language" — hard-constraining the model via prompt to emit "either a tool call or a final answer" as text, then parsing it. That's the 2023 ReAct-era approach. **Modern models all have native tool-calling APIs** (structured JSON, validity guaranteed by the provider), and pi/codex/claude-code use the native interface without exception. The stop condition isn't parsing text — it's reading `stop_reason == "tool_use"`. Inventing your own action language = deliberately throwing away the tool-calling ability the model learned during training.
- **`Message { role, content: String }` cannot hold reality.** A real message model must accommodate: multiple content blocks (text/thinking/image/tool_call), id pairing between tool_result and tool_call, stop reason, usage. See pi's `ai/src/types.ts` — that's exactly what our M1 ports.
- **No streaming.** The advice assumes throughout that "one call returns one complete reply." Streaming (SSE deltas, partial assembly, in-stream error encoding, cancellation) is a first-class hard problem for a harness — it's why M2 is labeled "the first hard fight." Skip it and what you build is a toy.
- **`MemoryStore { short_term: Vec<String>, long_term: Vec<String> }` is not how any real system does memory.** Real practice comes in two flavors, and neither is this: (1) compaction — LLM-summarize the **actual message history** and replace it (pi/codex, our M6); (2) file-based memory — write files + inject an index into the system prompt (claude-code `s09`, hermes, our M9).
- **A standalone Planner module is over-abstraction.** None of the three real harnesses has a "planner module" — **the model itself is the planner**; the harness only provides the loop and the tools. The closest thing to "planning" is a pure context-engineering tool like todo_write (`s05`). Reserving a module slot for a planner in v0 is designing an interface for something that doesn't exist.
- **`input["path"].as_str().unwrap()` + no schema.** Tool parameters must be declared by a JSON schema (which is how the model generates valid arguments), and errors must be returned to the model as a tool_result (so it can self-correct) — not unwrap-and-panic. This happens to be home turf for Rust's type system — our M3 gets it right with `schemars` + layered errors.

### 3. The four-phase roadmap (memory → repo agent → task graph) — not adopted

Because it's built on the mischaracterizations in point 1. Our replacement route: M0–M3 kernel (= its Phase 1, same direction) → M4–M7 the complete harness (truncation/sessions/compaction/TUI — which it never mentions but which are all non-negotiable) → M8–M9 extensions (permissions, skills, MCP, hooks, subagent, with the real codebases as the spec).

---

## Meta-Lessons

1. **Evaluate methodological advice and factual claims separately.** This advice scores 7/10 on methodology and 3/10 on facts — trust them as one blob and you'll get burned.
2. **For any claim of the form "library X's core is Y," spend 10 minutes opening the source to verify.** This curriculum's own exploration conclusions (docs/00, 01) should also be continuously fact-checked as you read.
3. **Beware designing new systems with old paradigms.** The litmus test for whether an agent design is current or dated: does it use the model's native tool-calling, or invent a text protocol and parse it? Does it treat streaming as a first-class citizen?
