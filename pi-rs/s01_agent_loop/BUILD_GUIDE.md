# s01_agent_loop 从零实现指南

`src/main.rs` 现在是跑通的版本（对照答案）。这份指南把它拆成 8 步，每步单独能编译、能验证，从零开始重新搭一遍。目标不是抄代码，是自己写、卡住了再看对照答案里对应的部分。

建议流程：新建 `src/main.rs`（或者先把现在的重命名备份成 `main.rs.ref`），每一步：
1. 读这一步的"要做什么"和"为什么"
2. 自己写，不看答案
3. 跑通验证方式里写的检查
4. 卡住 > 5-10 分钟再去看对照答案对应的行号，理解完继续往下写，不要复制粘贴

---

## Step 0 — 项目骨架

**要做什么**：`pi-rs/` 下已经有 workspace（`Cargo.toml` 里 `members = ["s01_agent_loop"]`）。在 `s01_agent_loop/` 建一个最小的二进制 crate：`Cargo.toml` + `src/main.rs`，先只 `println!` 一行字，能跑通。

**依赖先只加这些**（后面步骤再逐个加，不要一次性抄全）：
```toml
[dependencies]
anyhow = "1"
```

**验证**：`cargo run -p s01_agent_loop` 打印出一行字。

**为什么先这步**：确认 workspace/package 结构没问题，后面每步都在这个骨架上加。

---

## Step 1 — 读 Codex 登录态

**要做什么**：写一个函数，读 `$CODEX_HOME/auth.json`（没设就是 `~/.codex/auth.json`），解析出 `access_token` 和 `account_id` 两个字段，`main` 里打印 `account_id`（**不要打印 access_token 本身**，打印它的长度就行，避免意外把 token 输出到终端记录里）。

**新加依赖**：`serde`（`features = ["derive"]`）、`serde_json`。

**概念**：
- `#[derive(Deserialize)]` + 嵌套 struct：`auth.json` 长这样 `{"tokens": {"access_token": "...", "account_id": "...", ...}}`，你只需要关心的字段，其余字段 serde 默认会忽略，不用全部声明。
- `anyhow::Context` 给错误加提示信息（比如"文件读不到，去跑 codex login"）。

**验证**：`cargo run` 打印出你自己账号真实的 `account_id`，和 token 长度（应该是几百到上千字符）。

**对照答案**：`main.rs` 里的 `AuthDotJson` / `TokenData` / `load_codex_tokens`。

---

## Step 2 — 打一发写死的请求

**要做什么**：先不管 loop、不管工具，就发一个写死 prompt（比如 "Say exactly: pong"）的请求到 Responses API，把原始响应文本整个打印出来看看长什么样。

**新加依赖**：`tokio`（`features = ["rt-multi-thread", "macros"]`）、`reqwest`（`features = ["json", "rustls-tls"]`，`default-features = false`）。

**关键事实**（不是要你猜，直接告诉你，省得瞎试）：
- URL: `https://chatgpt.com/backend-api/codex/responses`
- Headers: `Authorization: Bearer <access_token>`、`ChatGPT-Account-Id: <account_id>`、`originator: codex_cli_rs`、`Accept: text/event-stream`
- Body 最小字段：`model`、`instructions`（系统提示词）、`input`（这次先写死一条 `{"type":"message","role":"user","content":[{"type":"input_text","text":"..."}]}`）、`tools: []`、`stream: true`（这个后端不支持非流式，必须给 true）
- 你账号能用的 `model` 是 `gpt-5.5`（写在 `~/.codex/config.toml` 里的 `model = "..."`，跟你自己账号核对）

**概念**：`#[tokio::main] async fn main()`、`serde_json::json!` 宏拼 body（不用先定义 struct，用 `Value` 最快）、`reqwest::Client::post().header().json().send().await`。

**验证**：打印出来的是一大段 `data: {...}\n\n` 格式的文本（SSE 格式），里面能肉眼看到你的 prompt 得到的回复文本。

**对照答案**：`agent_loop` 函数里发请求那一段（`cfg.client.post(...)...`），但注意答案版是循环里的，你这步先不用循环。

---

## Step 3 — 把 SSE 文本解析成结构化事件

**要做什么**：Step 2 拿到的是一坨文本，现在把它解析成 Rust 的类型。写 `SseEvent` enum（内部打标签 `type` 字段），至少认出 `response.output_item.done` 和 `response.completed` 两种；`item` 字段再建一个 `InputItem` enum，先只认 `message`（带 `content: Vec<ContentPart>`，`ContentPart` 先只认 `output_text`）。其他不认识的类型都要能优雅跳过，不能一遇到没见过的 `type` 就 panic。

**概念**：
- serde 内部标签枚举：`#[serde(tag = "type", rename_all = "snake_case")]`
- `#[serde(other)]`：给枚举一个"其他情况都归到这里"的兜底分支，这是让解析不因为遇到没建模的类型（比如 reasoning）就报错的关键
- 字符串按行处理：SSE 每条消息是 `data: {json}\n\n`，用 `.lines().filter_map(|l| l.strip_prefix("data: "))` 之类的方式取出 json 部分

**验证**：把 Step 2 打印出来的原始文本喂给你的解析函数，能打印出一个 `Message { content: [OutputText { text: "pong" }] }` 这样的结构（用 `{:?}` 打印验证）。

**对照答案**：`ContentPart` / `InputItem` / `SseEvent` / `parse_sse_events`。

---

## Step 4 — 从事件里提取"模型说了什么"

**要做什么**：写一个函数（或者直接在 `main` 里），遍历 Step 3 解析出的事件列表，把所有 `output_text` 的内容拼起来，作为"模型最终回答"打印出来。

**概念**：嵌套 `match`（先匹配 `SseEvent::OutputItemDone`，再匹配里面的 `InputItem::Message`，再遍历 `content`）。

**验证**：`cargo run` 完整跑一遍（Step 2 发请求 → Step 3 解析 → Step 4 提取），终端打印出模型的回答文本（这次应该是 "pong"）。

到这里，你已经有了一个能问一句、答一句的最小程序，但还不会用工具，也不会循环。

---

## Step 5 — 加一个工具，识别"模型要调用工具"

**要做什么**：
1. `InputItem` 加一个 `FunctionCall { name, arguments, call_id }` 分支
2. 请求 body 的 `tools` 字段填一个 `bash` 工具的 schema（注意字段名是 `parameters`，不是 Anthropic 用的 `input_schema`）：
   ```json
   {"type":"function","name":"bash","description":"Run a shell command.","strict":false,
    "parameters":{"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}}
   ```
3. 换一个必须用工具才能回答的 prompt（比如 "What is the current git branch?"），跑一遍，这次事件里应该会出现 `FunctionCall` 而不是 `Message`

**验证**：打印出 `FunctionCall { name: "bash", arguments: "{\"command\":\"git branch --show-current\"}", call_id: "..." }` 这样的结构。**先不用真的执行它**，这一步只是"看见"模型要什么。

**对照答案**：`InputItem::FunctionCall` 分支、`bash_tool_schema()`。

---

## Step 6 — 真的执行工具

**要做什么**：写 `run_bash(command: &str) -> String`：
- 危险命令拦截（`rm -rf /`、`sudo` 这种，命中就直接返回错误字符串，不执行）
- 用 `tokio::process::Command` 起一个 `bash -c "<command>"` 子进程，拿 stdout+stderr
- 加超时（120 秒），超时返回 "Error: Timeout"
- 输出太长要截断（比如 5 万字符），避免刷屏或者把下一次请求 body 撑爆

**概念**：`tokio::process::Command::spawn()` + `tokio::time::timeout(...).await`，`Stdio::piped()`。

**验证**：把 Step 5 拿到的 `arguments`（是个 JSON 字符串，要 `serde_json::from_str` 再取出 `command` 字段）传给 `run_bash`，打印执行结果，应该看到真实的 git branch 名字。

**对照答案**：`run_bash`。

---

## Step 7 — 拼成真正的 loop

**要做什么**：这是整个 s01 的核心。把 Step 2-6 串起来：
1. `InputItem` 再加 `FunctionCallOutput { call_id, output }` 分支（工具执行结果喂回去的格式）
2. 写 `agent_loop(input: &mut Vec<InputItem>)`：
   ```
   loop {
       发请求(带上完整的 input 历史)
       解析事件
       把 Message/FunctionCall 都 push 进 input（保持顺序，这样下次请求带着完整历史）
       如果这一轮一个 FunctionCall 都没有 → 打印最终文本，return
       否则 → 依次执行每个 FunctionCall，把结果包成 FunctionCallOutput push 进 input，继续 loop
   }
   ```

**关键坑**：Responses API 的 `input` 是**扁平列表**，不是按 role 分组的 messages —— 一次工具调用产生的 `FunctionCall` 和你补上的 `FunctionCallOutput` 都是列表里独立的元素，顺序要保持跟对话发生的顺序一致，不能乱序。

**验证**：问一个需要"调用工具 → 看结果 → 再回答"的问题（比如 "创建一个 hello.txt 文件，然后读出它的内容告诉我"），应该看到:执行 mkdir/echo → 执行 cat → 模型总结回答，一次问答里循环跑了两轮以上。

**对照答案**：`agent_loop` 整个函数。

---

## Step 8 — 套上 REPL

**要做什么**：`main()` 里包一层 `loop`，读 stdin 一行，空行/`q`/`exit` 退出；每轮把用户输入 push 成 `Message{role:"user",...}` 加进一个跨轮次持续存在的 `input: Vec<InputItem>`，调用 `agent_loop`。

**概念**：`io::stdin().read_line()` 返回 0 代表 EOF（Ctrl-D）。

**验证**：能连续问好几个问题，模型记得住前面对话的上下文（因为 `input` 是跨轮次累积的，没有清空）。

到这一步，就跟现在 `main.rs` 的完整实现等价了。

---

## 之后可以深挖的坑（s01 范围之外，故意没做）

- **Token 刷新**：现在 401 了只能手动重新 `codex login`，真正的 proactive refresh 逻辑在 `learn-codex` 的 `codex-rs/login/src/auth/manager.rs`（`should_refresh_proactively` 检查 JWT `exp`）。
- **真流式渲染**：现在是等整个 SSE 响应完了才一次性解析，真正的逐 token 渲染是 `docs/en/02-rust-rewrite-plan.md` 里 M2 的内容。
- **reasoning 等未建模的 item 类型**：现在直接丢弃（`#[serde(other)]` 兜底），生产级实现需要保留它们维持推理连续性。
