[English](../en/00-harness-mental-model.md) | 中文

# 00 — Harness 认知框架

一个 coding-agent harness，剥掉所有产品功能后，是一台**把"模型采样"和"世界副作用"连接起来的状态机**。
本文把它拆成 8 层，每层给出：它解决什么问题、必须回答的设计问题、三个参考实现各自的答案。

**用法**：先通读一遍建立地图；之后每完成一个重写里程碑，回来重读对应层，检查自己能否独立回答该层的设计问题。

---

## 分层总览

```
L7  运行形态      interactive / exec(print) / RPC / server / gateway
L6  扩展层        skills / extensions / MCP / hooks
L5  前端(TUI)     渲染、键盘、与引擎解耦的事件协议
L4  安全层        审批策略、沙箱、命令安全评估
L3  会话层        历史存储、恢复、分支、压缩(compaction)
L2  回合(turn)    采样 + 工具调用的循环，直到模型不再调工具
L1  工具系统      schema 定义、分发、执行、结果截断
L0  Provider 层   wire 协议、流式事件归一化、重试、per-provider 兼容
```

核心不变量：**L0–L2 是任何 harness 都绕不开的最小核**。pi 的 `agent` 包 + `ai` 包就是这个最小核（~44k 行含生成代码，手写核心不到 1 万行）；codex 的 `core` crate 是同一个核的工业加固版；hermes 在这个核外面包了一整个"个人 AI 操作系统"。

---

## L0 — Provider 层：把 N 种 API 归一成 1 种事件流

**解决什么**：Anthropic Messages、OpenAI Responses、Chat Completions、Gemini… 每家的请求格式、流式分片、thinking 表示、cache 控制都不同。上层 loop 不应该知道这些。

**设计问题**：
1. 归一化的内部事件协议长什么样？（pi: `AssistantMessageEvent` = `start / text_delta / thinking_delta / toolcall_delta / done / error`，每个事件带完整 partial 快照）
2. 流开始后发生错误怎么办？（pi 的答案很漂亮：**流式开始后永不 throw**，错误编码为终端 `error` 事件、`stopReason: "error"|"aborted"`——错误变成数据，上层统一处理）
3. per-provider 的怪癖放哪？（pi: `*Compat` 接口集中声明；hermes: 声明式 `ProviderProfile`；codex: 干脆只支持 Responses API 一种 wire 协议——**砍掉多样性也是一种答案**）
4. 重试边界在哪一层？（codex: `codex-client/src/retry.rs` 在 HTTP 层 + turn 内 sticky routing；pi: SDK 层重试 + `maxRetryDelayMs` 快速失败）

**三库答案**：
- pi: `packages/ai/` — 9 种 API 实现 × 39 个 provider 配置，统一导出 `stream`/`streamSimple`
- codex: `codex-api`（SSE/WS → `ResponseEvent`）+ `codex-client`（HTTP/重试）+ `core/src/client.rs`（loop 与网络层的接缝）
- hermes: `providers/base.py` 声明式 profile + `agent/*_adapter.py` 每 API 一个适配器

---

## L1 — 工具系统：模型的"手"

**解决什么**：把 JSON-schema 声明的能力暴露给模型，把模型的调用意图安全地变成真实副作用，再把结果（截断后）喂回去。

**设计问题**：
1. 工具的最小接口是什么？（pi: `name + description + parameters(schema) + execute(id, params, signal, onUpdate, ctx)`；codex: `ToolExecutor` trait）
2. **截断策略**——工具输出可能是 100MB 的日志，喂给模型多少、怎么截、截了之后如何让模型知道"还有更多"？（pi 的 `truncate.ts` 是教科书：行数/字节双限制先到先截、文件读取保头、bash 输出保尾、附带 "Use offset=N to continue" 的可行动提示）
3. 并行还是串行执行？终止语义？（pi: 每工具可声明 `executionMode`，结果按原顺序发出；`terminate: true` 可终止整批）
4. edit 工具用什么形式？（codex: 自研 apply_patch 自由格式 patch 语言 + 流式解析器；pi: 精确字符串替换 edit + diff 校验——两条路线，各有取舍）

**三库答案**：
- pi: `packages/coding-agent/src/core/tools/`（read/bash/edit/write/grep/find/ls，7 个）
- codex: `core/src/tools/{registry,router}.rs` + `handlers/`（shell、apply_patch、unified_exec…）
- hermes: `tools/registry.py` 自注册 + ~95 个工具 + toolset 分组（**每次 API 调用带全部核心工具 → 新增核心工具的门槛极高**，这是它的"窄腰"哲学）

---

## L2 — 回合循环：整个 harness 的心脏

**解决什么**：一次用户输入之后，"采样 → 执行工具 → 把结果放回上下文 → 再采样"的循环，直到模型给出不含工具调用的最终回答。

**设计问题（最重要的一组）**：
1. **停止条件**：什么时候一个 turn 结束？（三家一致的核心：assistant 消息不含 tool call 即停。codex 额外有 stop-hooks 可要求继续；pi 有 `shouldStopAfterTurn` 注入点）
2. **steering**：用户在 agent 干活时又输入了怎么办？（pi 有三条队列：steering / follow-up / next-turn，配 `"all"|"one-at-a-time"` 两种排空模式——这是 pi 设计里最值得细品的并发模型）
3. token 超限时怎么办？（codex: turn 循环里检测 `token_limit_reached` → 自动 compaction → 继续；见 L3）
4. loop 与外界怎么通信？（三家殊途同归：**事件流**。pi: `agent_start/turn_start/message_update/tool_execution_*/turn_end`；codex: `Op` 提交 / `EventMsg` 事件的 submission-event 架构，`protocol/src/protocol.rs` 里两个大 enum 就是引擎与所有前端的完整契约）

**三库答案**：
- pi: `packages/agent/src/agent-loop.ts` —— **全体系最值得精读的单个文件**，内外双循环 ~500 行
- codex: `core/src/tasks/regular.rs`（外层）+ `core/src/session/turn.rs` 的 `run_turn` / `run_sampling_request`（内层）
- hermes: `agent/conversation_loop.py`（~3900 行，功能最全但也最难读）

---

## L3 — 会话层：记忆与时间旅行

**设计问题**：
1. 历史用什么格式持久化？（pi: 每会话一个 JSONL 文件、首行 header、**append-only**；codex: rollout 文件）
2. 支持分支/回溯吗？（pi: 每条目 `id + parentId` 构成树、`leaf` 条目标记当前位置，**原地分支不复制文件**——非常优雅；codex: fork/resume）
3. **compaction**：上下文快满时怎么压缩？触发阈值？切点怎么选？（pi: `contextTokens > window - reserve` 触发，回退到 turn 边界切——**绝不把 tool call 和它的 result 拆开**，LLM 生成结构化摘要并跨次累积 `<read-files>/<modified-files>`；codex: `compact.rs` 用 SUMMARIZATION_PROMPT 摘要替换旧条目）
4. token 怎么数？（codex: 字节启发式估算，不跑真 tokenizer——工程务实的典型）
5. hermes 的独有答案：**prompt cache 是神圣的**——任何东西不得改写已缓存前缀、不得中途换 toolset/重建 system prompt，唯一豁免是 compaction。这是长会话成本控制的第一原则，值得刻进认知里。

**三库答案**：
- pi: `packages/agent/src/harness/session/` + `coding-agent/docs/session-format.md`（先读 doc）
- codex: `core/src/context_manager/history.rs` + `compact.rs` + `rollout` crate
- hermes: `hermes_state.py`（SQLite + FTS5 全文检索，跨会话搜索）

---

## L4 — 安全层：审批与沙箱

pi 在这层的答案是"**没有**"——它明确说不做权限系统，请用容器（`docs/containerization.md`）。codex 在这层最重，是这层的主要学习来源：

1. 策略词汇：`AskForApproval`（UnlessTrusted / OnRequest / Granular / Never）× `SandboxPolicy`（ReadOnly / WorkspaceWrite / DangerFullAccess）——`protocol/src/protocol.rs`
2. 决策函数：`core/src/safety.rs` 把两个策略结合，输出 `AutoApprove / AskUser / Reject`
3. OS 级执行：macOS seatbelt（`.sbpl` 策略文件）、Linux landlock/bwrap —— `sandboxing/src/manager.rs`

**认知要点**：审批（问不问用户）和沙箱（OS 强制隔离）是两个正交的轴，好的设计让它们独立组合。

---

## L5 — 前端：TUI 与解耦

**设计问题**：
1. 引擎和 UI 怎么解耦？（答案都是 L2 的事件协议：UI 只订阅事件流。codex 的 `protocol` crate 使同一引擎能接 TUI / exec / MCP-server / IDE 多个前端）
2. 渲染架构？两条路线：
   - codex: **ratatui**（immediate-mode 全帧重绘框架）+ crossterm，`tui/src/app.rs` 的 `tokio::select!` 事件循环
   - pi: **手写差分渲染器**（`tui/src/tui.ts`）：组件 `render(width) -> string[]`，逐行 diff 只重写变化行，用同步输出模式 `\x1b[?2026h` 包裹避免闪烁——想真正理解终端渲染，读 pi 这 1700 行比用框架收获大
3. 流式文本怎么渲染？（codex: 增量修改当前 history cell 再重绘；pi: 每个 delta 事件带完整 partial 快照，组件直接重渲染）

---

## L6 — 扩展层

| 机制 | pi | codex | hermes |
|---|---|---|---|
| Skills（SKILL.md，agentskills.io 标准） | ✅ 一等公民 | ✅ | ✅ 且能**自主创建/自我改进**技能 |
| 进程内插件 | ✅ TS extensions（注册工具/命令/provider/UI） | plugins/hooks | ✅ Python plugins |
| MCP | ❌（用 extensions 替代） | ✅ client + server 双向 | ✅ client + server |

**认知要点**：skills 的本质是"把能力描述注入 system prompt，全文按需 `read`"——成本极低、模型可自主发现，这是三家共识。进程内插件 vs MCP 是"信任内嵌代码"vs"进程隔离协议"的取舍。

---

## L7 — 运行形态

- pi: interactive（TUI）/ print / **RPC 模式**（JSONL over stdio，orchestrator 用它编排多 agent）
- codex: TUI / `codex exec` / app-server（IDE 用 JSON-RPC）/ MCP server
- hermes: CLI / TUI / **gateway 常驻守护进程**接 ~21 个聊天平台（telegram/discord/微信…）/ cron 调度 / ACP

**认知要点**：一旦 L2 的事件协议定义干净，运行形态就只是"换一个事件消费者"。hermes 证明了同一个核可以从终端一路伸到 Telegram。

---

## 自检清单

学完全部里程碑后，你应能不看代码回答：

1. 为什么流式错误要编码成事件而不是异常？
2. compaction 的切点为什么必须在 turn 边界？拆开 tool call 和 result 会发生什么？
3. steering 消息注入在 loop 的哪个位置？为什么需要多条队列？
4. 工具输出截断为什么"保头"和"保尾"要分场景？
5. 审批策略和沙箱策略为什么要正交设计？
6. 为什么三家都收敛到"引擎发事件、前端订阅"的架构？
7. prompt cache 约束如何影响你对"中途改 system prompt / 换工具集"这类功能的设计？
8. 如果明天要给你的 harness 加一个 Telegram 入口，需要动 L0–L2 的代码吗？（正确答案应该是：不需要）
