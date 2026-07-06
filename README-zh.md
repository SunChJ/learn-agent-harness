[English](./README.md) | 中文

# Learn Agent Harness — 用 Rust 亲手构建你自己的 Coding Agent

**这是一条学习路径，不是一个框架。** 你将研究四个真实的 agent harness——各司其职——并用 Rust 逐里程碑地重写其中最小、最干净的那个（[pi](https://github.com/badlogic/pi-mono)）。走完之后你同时拥有三样东西：一套关于 coding-agent harness 如何真正工作的认知模型、一个你亲手写出的真实 Rust 代码库、以及用"真正发布过的实现"去评判任何 agent 框架的判断力。

## 为什么做这个

Agent 产品 = **模型 + harness**。模型提供智能；harness 提供循环、工具、上下文管理和安全护栏。模型能力不是你能构建的——但 harness 是，而且 agent 产品里几乎所有工程决策都住在 harness 里。

大多数人学 harness 的方式有两种，都低效：从头到尾啃一个巨型代码库（会淹死——Codex 有约 110 万行 Rust），或者跟玩具教程走（跳过流式、截断、compaction 这些真正难的部分，教你一套 2023 年的过时设计）。本路径走第三条路：

> **先建立统一的认知框架，再带着框架读真实实现，最后动手重写其中一个。**

## 四个参考，四个生态位

四个参考仓库以 git submodule 形式固定——一条命令全部拉齐（见[起步](#起步)）。各占一个生态位：

| 仓库 | 角色 | 它给你什么 |
|---|---|---|
| [learn-claude-code](https://github.com/shareAI-lab/learn-claude-code)（Python，20 个渐进阶段） | **教科书** | 每个概念一个自包含可运行的 `code.py`——从 137 行的裸 loop 到 subagent 和 hooks。每个里程碑开工前先跑对应阶段：半小时换一份可运行的直觉。 |
| [pi](https://github.com/badlogic/pi-mono)（TypeScript，5 个包） | **规格书** | 重写对象。手写核心不到 1 万行，处处是干净的注入接缝，还有 33 篇官方设计文档。小到能装进脑子，真到值得重写。 |
| [Codex CLI](https://github.com/openai/codex)（Rust，约 98 个 crate） | **工业实现** | 同样的问题——loop、工具分发、流式、沙箱——用生产级 Rust 解过一遍。每个里程碑完成后，拿你的代码去和它对照 review。 |
| Nous Research 的 Hermes Agent（Python） | **对照组** | 回答其他几家没问的问题：多渠道 gateway、自我改进记忆/技能、cron、ACP。用来检验你认知框架的完备性——扫读即可。 |

**每个里程碑的循环：**

```
① 热身   —— 跑通 learn-claude-code 对应阶段（可运行直觉，约半小时）
② 读     —— pi 对应模块 + 它的设计文档；用自己的话写下设计决策
③ 写     —— 在 pi-rs/ 里用 Rust 实现；先让它跑起来，再谈漂亮
④ 对照   —— 打开 codex 里解决同一问题的代码，review 自己的抽象
           （它为什么用 trait？为什么用 enum？我哪里不符合 Rust 惯用法？）
⑤ 重构   —— 并在笔记里补一节"对照后学到的"
```

## 课程体系

| 文档 | 内容 |
|---|---|
| [00 — Harness 认知框架](docs/zh/00-harness-mental-model.md) | 先有地图再进丛林：把 harness 拆成 8 层（provider → 工具 → 回合循环 → 会话 → 安全 → TUI → 扩展 → 运行形态），每层必须回答的设计问题，以及四个参考各自的答案。文末附自检题。 |
| [01 — 对照阅读地图](docs/zh/01-reading-map.md) | 读什么、按什么顺序、具体到文件路径——包括刻意**跳过**什么，免得淹死。 |
| [02 — Rust 重写计划（M0–M9）](docs/zh/02-rust-rewrite-plan.md) | 沿 pi 的包依赖脊柱（ai → agent → tools → session → tui）的十个里程碑，每个含热身阶段、移植对象、Rust 学习点、codex 对照点、验收标准。 |
| [03 — 外部建议评审](docs/zh/03-advice-review.md) | 一个实例：如何拿真实代码库对 AI 生成的架构建议做事实核查——采纳什么、拒绝什么，以及如何分辨 2023 年的过时 agent 设计和当下的设计。 |

### 里程碑一览

| | 里程碑 | 你获得什么 |
|---|---|---|
| M0 | Hello LLM | cargo workspace、第一次 API 调用 |
| M1 | 消息模型 + 事件流 | tagged enum、serde、内部事件协议 |
| M2 | Anthropic 流式 provider | SSE、取消、**错误编码进流**——第一个硬仗 |
| M3 | **Agent Loop** | 心脏。工具 trait、prepare/execute/finalize 管线。*M3 之后你就拥有一个真正的 coding agent。* |
| M4 | 内置工具 + 截断 | read/edit/write/grep/find/ls；保头 vs 保尾截断 + 可行动提示 |
| M5 | 会话持久化 | append-only JSONL 会话**树**、恢复、原地分支 |
| M6 | Compaction | 阈值触发的历史摘要，绝不把 tool call 和 result 拆开 |
| M7 | TUI | 两条路线：ratatui（快）或 pi 式手写差分渲染器（深） |
| M8 | 选修 | 第二个 provider、skills、RPC 模式、审批+沙箱、MCP |
| M9 | Harness-OS 扩展 | hooks、subagent、计划工具、文件式记忆——检验你的内核抽象是否真的成立 |

M0–M3 构成**内核**：loop + 上下文组装 + 工具分发。之后的一切都是叠加在内核上的工程决策。

## 起步

```bash
# 四个参考仓库以固定版本的 submodule 一并拉取：
git clone --recurse-submodules https://github.com/SunChJ/learn-agent-harness.git
cd learn-agent-harness

# （已经 clone 但没带 submodule？执行：git submodule update --init）

# 热身阶段需要 Python + Anthropic API key：
cd learn-claude-code && pip install -r requirements.txt
ANTHROPIC_API_KEY=... python s01_agent_loop/code.py
```

你的 Rust 实现放在 `pi-rs/`——M0 时自己 `cargo init`，这本身就是练习的一部分。

## 适合谁

- 会编程，想**真正搞懂** agent harness，而不是收集一堆框架观点。
- 想通过真实项目学 Rust 而不是刷习题。不要求 Rust 基础——里程碑刻意排布了语言难度曲线（M1 所有权、M2 async、M3 trait object、M4 UTF-8 纪律）。
- 愿意花 2–3 个月换持久的判断力，而不是花 2 天走马观花。

## 怎么开始

1. 读[认知框架](docs/zh/00-harness-mental-model.md)（约 1 小时）。先有地图再进丛林。
2. 做[阅读地图](docs/zh/01-reading-map.md)的"第零步"：把裸 loop 跑在手上（约半天）。
3. 开始 [M0](docs/zh/02-rust-rewrite-plan.md)。

一条铁律：每完成一个里程碑，写一篇笔记（[模板](docs/zh/notes/TEMPLATE.md)），**用自己的话**回答该里程碑的设计问题。能回答设计问题才是真正的交付物；能跑的代码只是副产品。

## 致谢

- [pi / pi-mono](https://github.com/badlogic/pi-mono)（Mario Zechner）——重写规格书。
- [Codex CLI](https://github.com/openai/codex)（OpenAI）——工业级 Rust 参考。
- [learn-claude-code](https://github.com/shareAI-lab/learn-claude-code)（shareAI-lab）——渐进式教科书，也是本仓库双语组织方式的灵感来源。
- Hermes Agent（Nous Research）——对照参考。

## 贡献

欢迎：阅读地图的勘误（上游演进后文件路径会漂移）、更好的里程碑验收标准、翻译改进。请开 issue 或 PR。
