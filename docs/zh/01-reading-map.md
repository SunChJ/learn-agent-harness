[English](../en/01-reading-map.md) | 中文

# 01 — 三库对照阅读地图

按概念组织，每个概念给出：读哪些文件、按什么顺序、读的时候回答什么问题。
路径均相对于各仓库根目录。**原则：先跑 learn-claude-code 对应阶段（直觉），再读 pi（小而干净），带着问题读 codex（工业答案），hermes 只在标注处扫一眼。**

---

## 第零步：跑通裸 loop（半天，动手不读源码）

在读任何真实代码库之前，先把 harness 的"第一性原理"跑在手上：

```bash
cd learn-claude-code && pip install -r requirements.txt
ANTHROPIC_API_KEY=... python s01_agent_loop/code.py   # 137 行的完整 agent
python s02_tool_use/code.py                            # 加上工具分发
```

读懂这 ~300 行 + 两篇 stage README。此后你在 pi/codex 里看到的一切，都是往这个裸 loop 上叠加的工程决策。**整个 harness 的秘密就是 `while stop_reason == "tool_use"` 这一行**——先让这句话在你手里成立，再去看别人怎么把它做成产品。

### learn-claude-code 阶段 ↔ 本体系映射表

| 阶段 | 主题 | 对应认知层 / 里程碑 |
|---|---|---|
| s01 agent_loop / s02 tool_use | 裸 loop + 工具分发 | L1–L2 / M3 前热身 |
| s03 permission | 审批 | L4 / M8 |
| s04 hooks / s05 todo_write | 钩子、计划工具 | L6 / M9 |
| s06 subagent | 子 agent | L7 / M9 |
| s07 skill_loading | skills | L6 / M8 |
| s08 context_compact | 压缩 | L3 / M6 |
| s09 memory | 文件式记忆 | L3 / M9 |
| s10 system_prompt | system prompt 组装 | L2 / M3 |
| s11 error_recovery | 错误恢复 | L0 / M2 |
| s12–s14 task/background/cron | 任务系统 | L7 / M9（对照 hermes） |
| s15–s17 teams/autonomous | 多 agent | 超纲，体系完成后选读 |
| s18 worktree_isolation | 隔离 | L4 / M9 |
| s19 mcp_plugin | MCP | L6 / M8 |
| s20 comprehensive | 综合 | 总复习 |

---

## 第一遍：主干线（建议 2–3 天，只读不写）

目标：把 L0–L2 的最小核走通一遍，形成"一次用户输入到底发生了什么"的完整心智轨迹。

| 步骤 | 读什么 | 回答什么 |
|---|---|---|
| 0 | learn-claude-code `s01` + `s02`（见上文第零步） | 裸 loop 长什么样？停止条件是什么？ |
| 1 | pi `README.md` + `AGENTS.md` + `packages/coding-agent/docs/index.md` | 作者的设计哲学是什么？"small core, extended via TypeScript" 具体指什么？ |
| 2 | pi `packages/ai/src/types.ts`（只看 `Message`、content blocks、`AssistantMessageEvent`） | 内部消息模型长什么样？流式事件为什么每个都带完整 partial？ |
| 3 | pi `packages/agent/src/agent-loop.ts` **逐行精读** | 内外双循环各管什么？turn 何时结束？工具调用的 prepare/execute/finalize 三段各做什么？ |
| 4 | pi `packages/agent/src/types.ts`（`AgentMessage`、事件类型） | `AgentMessage` 为什么是 LLM `Message` 的超集？`convertToLlm` 这个注入点存在的意义？ |
| 5 | codex `core/src/tasks/regular.rs` + `core/src/session/turn.rs` 前 ~450 行 | 同一个 loop，Rust 版长什么样？`Op`/`EventMsg` 的 submission-event 架构和 pi 的 EventStream 有何异同？ |
| 6 | codex `protocol/src/protocol.rs`（`Op`、`EventMsg` 两个 enum 浏览一遍） | 引擎与前端的完整契约有多少种消息？ |

读完写第一篇笔记：用自己的话 + 一张图，描述"用户按下回车之后的完整数据流"。

---

## 第二遍：分专题（配合重写里程碑，边写边读）

### 专题 A：Provider 层与流式（配合 M1–M2）

- pi `packages/ai/src/utils/event-stream.ts` —— 手写异步事件流，~百行，Rust 重写时你要决定用 channel 还是 `Stream` trait 复现它
- pi `packages/ai/src/api/anthropic-messages.ts` —— 你的第一个移植对象
- pi `packages/ai/src/api/openai-responses.ts` + `types.ts` 里的 `*Compat` 接口 —— 感受 per-provider 兼容矩阵的复杂度（先不移植）
- codex `codex-api/src/sse/responses.rs` 的 `process_responses_event` —— SSE 原始 JSON → 内部事件的映射，Rust 里如何用 enum + match 表达
- codex `codex-client/src/retry.rs` —— 重试/退避的干净实现
- codex `core/src/client.rs` 顶部大注释 —— loop 与网络层的接缝设计

问题：错误如何流经这一层？重连/重试时已收到的一半流怎么办？

### 专题 B：工具系统（配合 M3–M4）

- pi `packages/coding-agent/src/core/tools/truncate.ts` —— **先读这个**，双限制截断 + 可行动提示
- pi `tools/read.ts`（定义模式模板）→ `tools/bash.ts`（流式输出、超时、溢出落盘）→ `tools/edit.ts`
- pi `tools/tool-definition-wrapper.ts` —— ToolDefinition 与核心 AgentTool 的关系
- codex `core/src/tools/registry.rs`（trait 设计）+ `router.rs`（分发）+ `handlers/shell.rs`
- codex `apply-patch/src/parser.rs` —— 自由格式 patch 语言（对照 pi 的精确替换 edit，思考取舍）
- hermes 扫一眼：`tools/registry.py` 的模块级自注册 + AST 发现机制

问题：工具 trait 在 Rust 里怎么设计才能既支持 schema 生成又支持异步执行 + 进度回调？

### 专题 C：会话与 compaction（配合 M5–M6）

- pi `packages/coding-agent/docs/session-format.md` + `docs/compaction.md` —— **先读 doc 再读码**
- pi `packages/agent/src/harness/session/jsonl-storage.ts` + `session.ts` 的 `buildSessionContext`
- pi `packages/agent/src/harness/compaction/`
- codex `core/src/context_manager/history.rs` + `core/src/compact.rs` + `session/token_budget.rs`
- hermes 扫一眼：`hermes_state.py` 开头（SQLite+FTS5 的另一条路）、AGENTS.md 里 prompt-cache-sacred 原则

问题：树状历史的 leaf→root 重建怎么做？compaction 摘要如何跨次累积状态？

### 专题 D：TUI（配合 M7）

- pi `packages/coding-agent/docs/tui.md` → `packages/tui/src/tui.ts`（差分渲染核心）
- pi `packages/tui/src/stdin-buffer.ts` + `keys.ts` —— 原始终端输入解析
- codex `tui/src/app.rs`（`tokio::select!` 事件循环）+ `chatwidget.rs`（流式如何进 scrollback）
- 决策点：你的 Rust 版用 ratatui（codex 路线，快）还是手写差分渲染（pi 路线，学得深）？M7 里有建议。

### 专题 E：安全层（纯读，主要看 codex）

- codex `protocol/src/protocol.rs` 的 `AskForApproval` / `SandboxPolicy` → `core/src/safety.rs` → `sandboxing/src/manager.rs` + 任一 `.sbpl` 文件
- pi `packages/coding-agent/docs/containerization.md` —— "不做权限系统"的理由，作为反方观点

### 专题 F：扩展与运行形态（配合 M8，选读）

- pi `docs/skills.md` + `packages/coding-agent/src/core/skills.ts` —— skills 机制其实很薄
- pi `docs/extensions.md` + `src/core/extensions/types.ts` —— 进程内插件 API 的完整形态
- pi `docs/rpc.md` + `modes/rpc/` —— JSONL over stdio 的无头模式
- codex `mcp-server/src/message_processor.rs` —— 把自己暴露成 MCP server
- hermes 扫一眼：`gateway/run.py`（多渠道常驻）、`cron/scheduler.py`、`agent/curator.py`（自我改进记忆）—— 检验你的 L7 认知

---

## 明确跳过的部分（避免淹死）

- codex：`app-server*` 全家、`cloud-*`、`otel`/`analytics`、`realtime-*`/`webrtc`、multi-agent 编排、巨型测试文件（`session/tests.rs` 有 1.1 万行）
- pi：`models.generated.ts`（生成代码）、39 个 provider 里除 anthropic/openai 之外的全部、`orchestrator` 包
- hermes：除上文标注的扫读点外全部；`cli.py`（741KB）和 `run_agent.py`（269KB）不要通读
