[English](../en/03-advice-review.md) | 中文

# 03 — 外部建议评审：Mini-Harness OS v0

一份外部 AI 建议（"Mini-Harness OS v0"四阶段路线 + Rust 骨架）曾作为本体系的参考输入。
本文逐条评审：**采纳什么、修正什么、为什么**。这本身是一次认知练习——把建议对照真实代码库做事实核查，比接受任何单一来源的叙事更重要。

## 结论先行

该建议的**方法论层面基本正确**（先统一抽象再对照实现、内核优先、避坑清单），已吸收进本体系；
但它的**事实层面有多处错误**（对三个库的定性张冠李戴），**技术方案层面是 2023 年的设计**（文本动作协议、无流式、玩具记忆），照抄会学到一个过时的 harness。

---

## ✅ 采纳的部分

| 建议 | 处置 |
|---|---|
| "先统一 abstraction，再对照实现，逐个研究源码是低效探索" | **完全正确**，这就是 docs/00 认知框架存在的理由：先有 8 层地图，再进代码丛林。 |
| 内核 = loop + context builder + tool router，先立内核 | 正确且是好的心智锚点，已写入 docs/02 开头（= 我们的 M0–M3）。 |
| 避坑：不要一开始 multi-agent / 工具贪多 / 过早 embedding | 与 pi、codex、hermes 三家的实际演化路径一致，已写入 docs/02"提前避坑"。 |
| 按周的节奏感 | 保留精神（里程碑制），但我们的估时基于真实模块的实际复杂度而非拍脑袋三周。 |

## ❌ 修正的部分

### 1. 对三个代码库的定性是错的

| 该建议的说法 | 事实（探索三个库源码后） |
|---|---|
| "Pi 方向 = memory system / embedding memory / persona" | pi **没有** embedding 记忆和 persona 系统。pi 是极简 coding harness：loop、工具、JSONL 会话树、compaction。"记忆/persona"是 **hermes** 的领地（`agent/memory_manager.py`、curator、Honcho 用户建模）。 |
| "Hermes 方向 = task graph / DAG planner" | hermes 核心**没有** DAG planner。它的独特性是多渠道 gateway（21 个聊天平台）、自我改进技能、cron、ACP。 |
| "Codex 方向 = file graph / patch diff" | codex 没有"file graph"。它的精华是沙箱/审批体系（seatbelt/landlock + `safety.rs`）、submission-event 协议、apply_patch 语言。"patch diff"只对了半个。 |

**教训**：未读源码的定性会自信地错。这正是本体系坚持"探索真实代码库 → 再定路线"的原因。

### 2. 技术方案是过时的（最重要的修正）

- **文本动作协议已死**。建议的核心是"Prompt Protocol 统一动作语言"——用 prompt 强约束让模型输出"要么 tool call 要么 final answer"的文本再解析。这是 2023 年 ReAct 时代的做法。**现代模型都有原生 tool-calling API**（结构化 JSON、由 provider 保证合法性），pi/codex/claude-code 无一例外直接用原生接口。stop 条件不是解析文本，而是读 `stop_reason == "tool_use"`。自己发明动作语言 = 主动放弃模型训练时学到的工具调用能力。
- **`Message { role, content: String }` 装不下现实**。真实消息模型必须容纳：多 content block（text/thinking/image/tool_call）、tool_result 与 tool_call 的 id 配对、stop reason、usage。见 pi `ai/src/types.ts`——这就是我们 M1 的移植对象。
- **没有流式**。建议通篇假设"一次调用返回一个完整回复"。流式（SSE 增量、partial 拼装、流中错误编码、取消）是 harness 的一等难题，也是 M2 被标为"第一个硬仗"的原因。跳过它做出来的是玩具。
- **`MemoryStore { short_term: Vec<String>, long_term: Vec<String> }` 不是任何真实系统的记忆**。真实做法有两种，都不是这个：① compaction——对**真实消息历史**做 LLM 摘要并替换（pi/codex，我们的 M6）；② 文件式记忆——写文件 + 索引注入 system prompt（claude-code `s09`、hermes，我们的 M9）。
- **独立的 Planner 模块是过度抽象**。三个真实 harness 里都没有"planner 模块"——**模型本身就是 planner**，harness 只提供 loop 和工具。最接近"计划"的东西是 todo_write 这种纯上下文工程工具（`s05`）。v0 里给 planner 留模块位，是在为不存在的东西设计接口。
- **`input["path"].as_str().unwrap()` + 无 schema**。工具参数必须由 JSON schema 声明（模型据此生成合法参数），错误必须作为 tool_result 返回给模型（模型能自我纠正），而不是 unwrap panic。这恰好是 Rust 类型系统的主场——我们 M3 用 `schemars` + 分层错误做对这件事。

### 3. 四阶段路线（memory → repo agent → task graph）不采纳

因为它建立在第 1 点的错误定性上。我们的替代路线：M0–M3 内核（= 它的 Phase 1，方向一致）→ M4–M7 完整 harness（截断/会话/压缩/TUI，它完全没提但缺一不可）→ M8–M9 扩展（权限、skills、MCP、hooks、subagent，以真实代码库为规格）。

---

## 元教训

1. **方法论建议和事实陈述要分开评估**。这份建议方法论 7 分、事实 3 分——混着信会出事。
2. **任何"X 库的核心是 Y"的说法，花 10 分钟开源码验证**。本体系的探索结论（docs/00、01）本身也应被你在阅读中持续核查。
3. **警惕带着旧范式设计新系统**。判断一份 agent 方案新旧的试金石：它是让模型用原生 tool-calling，还是发明文本协议再解析？它有没有把流式当一等公民？
