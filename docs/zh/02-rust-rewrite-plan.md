[English](../en/02-rust-rewrite-plan.md) | 中文

# 02 — pi-rs：里程碑式 Rust 重写计划

在 `pi-rs/` 用 Rust 重写 pi。里程碑顺序沿 pi 的包依赖脊柱：**ai → agent → tools → session → tui**。
每个里程碑包含：热身（learn-claude-code 对应阶段）、目标、移植对象（pi 中的文件）、Rust 学习点、codex 对照点、验收标准。

**内核框架**（一个有用的心智锚点）：M0–M3 合起来构成 harness 的"最小内核"——**loop + context 组装 + tool 分发**这三件事。M4 之后的一切（截断、会话、压缩、TUI、权限）都是往内核上叠加的工程决策。走到 M3 验收通过，内核即成立。

**总原则**：
- pi 是规格，不是圣经——遇到 TS 特有的写法（declaration merging、鸭子类型注入），停下来想 Rust 的等价物（trait、泛型、enum），这正是学习发生的地方。
- 每个里程碑开工前，先跑通 learn-claude-code 对应阶段（下文"热身"栏），半小时换一份可运行直觉。
- 每个里程碑结束跑 `cargo clippy && cargo fmt && cargo test`，并写一篇笔记（模板见 `notes/TEMPLATE.md`）。
- 不要提前抽象。pi 自己也是"先能跑，抽象后置"。

**提前避坑**（血泪共识，各家 AGENTS.md 与外部建议均指向同一组）：
- ❌ 一开始就做 multi-agent —— 内核没立住之前必然失控（M9 之前不碰）
- ❌ 工具贪多 —— M3 只要 bash 一个，M4 也只有 7 个；2–3 个就能验证全部抽象
- ❌ 过早上 embedding / RAG / 向量记忆 —— 真实 harness 的记忆是"压缩消息历史 + 文件式记忆"，不是向量库
- ❌ 跳过流式 —— 流式是一等公民难题（M2 是硬仗），跳过它做出来的是玩具

推荐基础依赖：`tokio`（异步运行时）、`reqwest`（HTTP+流式）、`serde`/`serde_json`、`thiserror`+`anyhow`（错误）、`schemars`（JSON schema 生成，对应 typebox）、`clap`（CLI）、`crossterm`（终端）。

---

## M0 · Hello LLM（1–3 天）

**目标**：cargo workspace 骨架 + 一次非流式 Anthropic Messages API 调用，打印回复。

- 结构：workspace 下先建一个 `pi-ai` crate（对应 pi 的 `packages/ai`），后续里程碑逐个加 crate，复现 pi 的包边界。
- **Rust 学习点**：cargo workspace、`Result`/`?`、serde derive、tokio `#[tokio::main]`、reqwest 基本用法。
- **验收**：`cargo run -p pi-ai --example hello` 能拿到 Claude 的回复；API key 从环境变量读。

## M1 · 消息模型 + 事件流（3–7 天）

**目标**：移植 pi-ai 的核心类型系统和 `EventStream`。

- 移植对象：`packages/ai/src/types.ts` 里的 `Message`（User/Assistant/ToolResult）、content blocks（Text/Thinking/Image/ToolCall）、`Usage`、`StopReason`、`AssistantMessageEvent`；`utils/event-stream.ts`。
- 关键设计决策（自己做，笔记里记录理由）：
  - TS 的 tagged union → Rust `enum` + `#[serde(tag = "type")]`
  - `EventStream` → `tokio::sync::mpsc` channel 包一层，还是实现 `futures::Stream`？（建议先 channel，M7 前再评估）
- **Rust 学习点**：enum 建模、serde tagged enum、ownership 在消息传递中的体现、`Arc` 何时需要。
- **codex 对照**：`protocol/src/protocol.rs` 的 `ResponseItem`/`EventMsg`——看工业级 Rust 如何用 enum 表达同类协议。
- **验收**：类型 round-trip 测试（serialize → deserialize == 原值）；一个 mock 事件流的消费测试。

## M2 · Anthropic 流式 Provider（1–2 周，第一个硬仗）

**热身**：learn-claude-code `s11_error_recovery`（错误如何被吞进流里而不是炸出去）。
**目标**：SSE 流式调用 Anthropic，把原始事件翻译成 M1 的 `AssistantMessageEvent`，含工具调用分片的增量拼装。

- 移植对象：`packages/ai/src/api/anthropic-messages.ts`（含 partial JSON 拼装逻辑）。
- 核心纪律：**流开始后永不返回 Err**——错误编码为终端 `error` 事件（pi 的在流错误编码），`stopReason` 进 `Error/Aborted`。
- 定义 provider trait（对应 pi 的 `ProviderStreams`）：`fn stream(&self, ctx: Context) -> AssistantMessageEventStream`。
- **Rust 学习点**：async 深水区（SSE 逐行解析、`tokio::select!`、取消/`AbortSignal` → `CancellationToken`）、`thiserror` 分层错误设计、trait object vs 泛型。
- **codex 对照**：`codex-api/src/sse/responses.rs` 的 `process_responses_event`、`codex-client/src/retry.rs`。
- **验收**：流式打印回复逐字出现；带一个假工具时能完整拼出 `ToolCall`；断网/无效 key 时收到 `error` 事件而非 panic；`Ctrl+C` 中断得到 `aborted`。

## M3 · Agent Loop（1–2 周，全项目认知核心）

**热身**：learn-claude-code `s01_agent_loop` + `s02_tool_use` + `s10_system_prompt`（如果第零步已跑过 s01/s02，重读一遍代码即可）。
**目标**：移植 `agent-loop.ts`，新建 `pi-agent` crate。

- 移植对象：`packages/agent/src/agent-loop.ts`（内外双循环、prepare/execute/finalize 三段工具管线、串行/并行执行、`terminate` 语义）+ `types.ts` 的 `AgentMessage` 与 agent 事件协议。
- 定义工具 trait（对应 `AgentTool`）：schema（用 `schemars` 生成）+ `async fn execute(id, params, cancel, on_update) -> AgentToolResult`。
- 先只实现一个 `bash` 工具（`tokio::process::Command`，流式输出、超时）打通全链路。
- 注入点用泛型/闭包字段复现：`stream_fn`、`convert_to_llm`、`should_stop_after_turn`。
- **Rust 学习点**：trait object（`Box<dyn Tool>`）、async trait、`JoinSet` 并行执行、借用检查器在循环中改历史的经典冲突（你一定会撞上，撞上就是学到）。
- **codex 对照**：`core/src/session/turn.rs` 的 `run_turn` + `core/src/tools/{registry,router}.rs` + `parallel.rs`。
- **验收**：CLI 一行输入 → agent 调 bash 工具 → 观察"采样→执行→回填→再采样→纯文本停止"完整循环；agent 事件流打印在 stderr。**做到这里，你已经拥有一个真正的 coding agent。**

## M4 · 内置工具全家 + 截断（1 周）

**目标**：read / edit / write / grep / find / ls + 截断层。

- **先移植 `tools/truncate.ts`**：行/字节双限制先到先截、UTF-8 边界安全、保头 vs 保尾、"Use offset=N to continue" 提示。给它写最全的单元测试（这是最好练 Rust 测试的模块）。
- edit 用 pi 的精确字符串替换路线（apply_patch 语言可作为后期选修，对照 codex `apply-patch` crate）。
- **Rust 学习点**：文件 I/O、`&str`/`String`/bytes 与 UTF-8 边界处理（Rust 在这里比 TS 严格得多，正是价值所在）、path 处理。
- **验收**：用你自己的 agent 完成一个真实小任务（如"读这个文件并修掉一个 bug"）；截断模块测试覆盖边界情况（超长行、多字节字符切点、空文件）。

## M5 · 会话持久化（1 周）

**目标**：JSONL 树状会话存储 + 恢复。

- 移植对象：先读 `docs/session-format.md`，再移植 `harness/session/jsonl-storage.ts`（`SessionStorage` trait、append-only、`getPathToRoot`、leaf 标记）+ `buildSessionContext`（leaf→root 重建上下文）。
- **Rust 学习点**：trait 作为存储抽象（Jsonl 实现 + 内存实现两个 impl）、serde 处理多态条目类型、文件追加写。
- **验收**：`--continue` 能恢复上次会话继续对话；手动把 leaf 移到早期节点后新消息正确形成分支；两个存储实现通过同一套 trait 测试。

## M6 · Compaction（3–7 天）

**热身**：learn-claude-code `s08_context_compact`（~520 行，压缩概念的最小可运行版）。
**目标**：上下文自动压缩。

- 移植对象：`docs/compaction.md` 的规格 + `harness/compaction/`。要点：阈值触发（`contextTokens > window − reserve`）、回退到 turn 边界选切点、**绝不拆开 tool call 与 result**、LLM 生成结构化摘要、`<read-files>/<modified-files>` 跨次累积。
- token 估算先用字节启发式（codex 同款务实做法，`context_manager/history.rs`）。
- **验收**：构造一个超长会话触发自动 compaction，压缩后 agent 仍能引用早期关键事实；切点测试证明 tool call/result 永远同侧。

## M7 · TUI（2–3 周，可选深潜）

**目标**：交互式界面。两条路线，二选一：

| 路线 | 学到什么 | 建议 |
|---|---|---|
| A：ratatui + crossterm（codex 路线） | 快速得到能用的 UI，学 Rust 生态整合 | 想尽快到 M8 选这个 |
| B：手写差分渲染器（pi 路线，移植 `tui/src/tui.ts`） | 终端协议本身：逐行 diff、同步输出模式、光标控制、raw 输入解析 | 想吃透终端就选这个，pi 那 1700 行是最好的教材 |

- 无论哪条路线，架构不变：**UI 只订阅 M3 的 agent 事件流**，引擎零改动——这是对 L5 解耦认知的直接检验。
- 最小功能集：流式渲染 assistant 文本（markdown 可后置）、工具调用展示、输入框、Esc 中断、steering（干活时输入排队——回头把 pi 的三队列模型补进 M3 的 Agent 包装层）。
- **codex 对照**：`tui/src/app.rs` 的事件循环结构。

## M8 · 选修拼图（各 2–5 天，按兴趣挑）

- **第二个 provider（OpenAI）**：真正检验 M2 的 trait 抽象是否成立——抽象只有在第二个实现出现时才被验证。
- **Skills**（热身：`s07_skill_loading`）：机制很薄（发现 SKILL.md → 名字+描述注入 system prompt → 模型自主 `read`），收益极高。
- **RPC/print 模式**：JSONL over stdio，检验事件协议的完备性。
- **审批+沙箱**（热身：`s03_permission`；pi 没有，从 codex 学）：`safety.rs` 的决策函数 + macOS seatbelt 包裹 bash 工具——把 L4 认知落地。
- **MCP client**（热身：`s19_mcp_plugin`）：用官方 rust-sdk（rmcp），对照 codex 的 `rmcp-client`。

## M9 · Harness OS 扩展（Claude Code 方向，选修，各 3–7 天）

pi-rs 内核完成后，超出 pi 的能力范围，用 **learn-claude-code 的阶段作为规格**继续扩展。这一步的意义：检验你的内核抽象是否真的能承载"叠加"——如果加这些功能需要改 M3 的 loop 代码，说明内核抽象有问题。

- **Hooks**（规格：`s04_hooks`；对照 codex `hooks` crate）：工具调用前后的拦截点。你在 M3 已经有 `beforeToolCall/afterToolCall` 注入点，这一步是把它产品化为用户可配置的机制。
- **Todo/计划工具**（规格：`s05_todo_write`）：纯上下文工程——一个只写不执行的工具如何显著提升多步任务表现。
- **Subagent**（规格：`s06_subagent`；对照 pi 的 RPC 模式 + orchestrator 包）：把你自己的 agent 作为工具暴露给你自己的 agent。事件协议完备性的终极测试。
- **文件式记忆**（规格：`s09_memory`；对照 hermes `agent/memory_manager.py`）：注意真实系统的记忆是"文件 + 索引注入 system prompt"，不是向量库。
- **后台任务 / worktree 隔离**（规格：`s12`–`s13`、`s18`）：任务生命周期管理。
- 走完这里如果还想继续：`s15`–`s17`（multi-agent teams）和 hermes 的 gateway 是下一个地平线，但那已经是新项目而非本学习工程的范围。

---

## 进度追踪

| 里程碑 | 状态 | 笔记 | 完成日期 |
|---|---|---|---|
| M0 Hello LLM | ⬜ | | |
| M1 消息模型+事件流 | ⬜ | | |
| M2 Anthropic 流式 | ⬜ | | |
| M3 Agent Loop | ⬜ | | |
| M4 工具+截断 | ⬜ | | |
| M5 会话 | ⬜ | | |
| M6 Compaction | ⬜ | | |
| M7 TUI | ⬜ | | |
| M8 选修 | ⬜ | | |
| M9 Harness OS 扩展 | ⬜ | | |
