[English](README.md) | 中文

# s01: Agent Loop — 一个循环就够了

移植自 [learn-claude-code/s01_agent_loop](../../learn-claude-code/s01_agent_loop/)，是 `pi-rs/` 下独立的教学复刻 track，逐阶段推进；它跟 `docs/en/02-rust-rewrite-plan.md` 里 M0-M9 里程碑重写计划的关系见 [pi-rs/README.md](../README.md)。

`s01` → s02 → s03 → ... → s20（阶段随进度陆续添加）

---

## 问题

你提出了一个问题给大模型："帮我读取下我的目录下有哪些文件，并且执行 XXX.py"。

模型能输出一条 bash 命令，但输出完了就停了，它不会自己跑，也不会看到结果后继续推理。把这个手动来回自动化，就是这一阶段要做的事。

## 解决方案

一个循环：模型要用工具就继续，不用就停。

| 信号 | 含义 | 循环动作 |
|---|---|---|
| 这一轮产出了 ≥1 个 function-call | 模型举手说"我要用工具" | 逐个执行 → 结果喂回去 → 继续 |
| 这一轮产出 0 个 function-call | 模型说"我做完了" | 退出循环 |

## 为什么不是 Anthropic Messages API

Python 原版和这个项目最初的 Rust 草稿都是拿 `ANTHROPIC_API_KEY` 调 Anthropic Messages API。这个版本改成复用已有的 **Codex CLI 登录态**（ChatGPT 订阅，不是按 token 计费），调用 Codex CLI 自己用的那个后端上的 **OpenAI Responses API**。这不只是换个 URL 那么简单：

- **认证**：没有 API key。Codex CLI 的 OAuth 登录（`codex login`）已经把 `access_token` 和 `account_id` 写到了 `$CODEX_HOME/auth.json`（默认 `~/.codex/auth.json`）。这一阶段只是读这个文件——**不**实现 OAuth/PKCE 登录流程或 token 刷新。如果调用返回 `401`，解法是重新跑一遍 `codex login`；把主动刷新、JWT 过期检查这些做对，超出了这一阶段的范围。
- **接口形状**：`POST https://chatgpt.com/backend-api/codex/responses`，请求头 `Authorization: Bearer <access_token>`、`ChatGPT-Account-Id: <account_id>`、`originator: codex_cli_rs`。请求体是 OpenAI Responses API 的形状，不是 Anthropic Messages API 的形状：
  - `input` 是**一个扁平的 item 列表**（不是按 role 分组的 `messages`）：`{"type":"message","role":"user","content":[{"type":"input_text",...}]}`、`{"type":"function_call","name","arguments","call_id"}`、`{"type":"function_call_output","call_id","output"}`。
  - 工具 schema 用的字段是 `parameters`（Anthropic 用的是 `input_schema`）。
  - 没有 `stop_reason` 这个东西。"做完了"是隐式信号：这一轮要么产出了 `function_call` item（继续），要么没有（这就是最终答案）。
- **必须走流式**：这个后端只提供 SSE（`stream: true` 不是可选项）。为了让这一阶段的*循环*逻辑保持简单，我们把整个 SSE 响应体缓冲下来一次性解析（`parse_sse_events`），而不是逐 token 渲染——真正的增量流式渲染是 `docs/en/02-rust-rewrite-plan.md` 里 M2 的内容，这里只是被 API 逼着提前借用了一点点。
- **历史会丢信息**：reasoning 之类的其他输出 item 类型没有建模，会从对话历史里被丢掉（`InputItem::Other`）。对于一个只有一个工具的 demo 来说够用；真正的生产 agent 需要把它们保留下来。

这些细节是从 `learn-codex`（Rust 版 `codex` CLI 源码）里逆向出来的——如果想自己顺一遍 wire format，可以看 `codex-rs/model-provider-info/src/lib.rs`、`codex-rs/tools/src/responses_api.rs`、`codex-rs/codex-api/src/sse/responses.rs`。

**模型说明**：默认值（`gpt-5.3-codex-spark`）在这个账号上实测跑通过（`~/.codex/config.toml` 里当时配的 `model = "gpt-5.5"` 也跑通过，两个都行）。如果你的账号/套餐用的是别的模型，查一下自己的 config 或者用 `CODEX_MODEL` 覆盖——传一个账号不支持的模型会返回 `400`，报错信息里会写清楚是哪个模型不行。

## 试一下

**前提**：已经跑过一次 `codex login`（这一阶段不做登录这一步）。

**运行**（在 `pi-rs/` 目录下）：

```sh
cargo run -p s01_agent_loop
```

试试这些 prompt：

1. `Create a file called hello.py that prints "Hello, World!"`
2. `List all files in this directory`
3. `What is the current git branch?`

观察重点：模型什么时候调用工具（循环继续），什么时候不调用（循环结束）？

> **教学 demo 提示**：代码会执行模型生成的 shell 命令，建议在临时测试目录里运行。目前还没有权限系统，那是后面阶段的内容。

## 接下来

现在模型手里只有 bash，读文件要 `cat`，写文件要 `echo ... >`。s02 会给它真正的工具。
