[English](README.md) | 中文

# pi-rs

这里有两条独立的线：

1. **里程碑重写**（M0-M9，规划在 `../docs/en/02-rust-rewrite-plan.md`）：真正的、生产形态的 `pi` Rust 重写，沿着 pi 的包依赖脊柱（`ai → agent → tools → session → tui`）推进。还没开始。
2. **`sNN_*` 阶段文件夹**（本 README 的主题）：对 [learn-claude-code](../learn-claude-code/) 20 阶段课程的教学复刻，每个阶段一个 crate，都会真实调用模型（通过已有的 Codex CLI / ChatGPT 订阅登录态，不是按 token 计费的 API key——具体原因见每个阶段自己的 README）。这条线是探索性的，跟 M0-M9 计划各自独立推进——这里验证过的想法之后可能会被产品化进 M0-M9 的 crate，但不是必须的。

每个阶段都是独立的 Cargo package（各自的 `Cargo.toml`、各自的 `README.md`/`README-zh.md`），跟 `learn-claude-code/sNN_*` 每个文件夹自成一体的方式一致。

## 阶段

| 阶段 | 主题 | 状态 |
|---|---|---|
| [s01_agent_loop](s01_agent_loop/) | 核心的 `while tool_use` 循环 | 完成 |

在 `pi-rs/` 下运行任意阶段：

```sh
cargo run -p <stage_name>
```

## `AGETNS.md`

这个目录下的 `AGETNS.md` 是从 `learn-pi` 的 `AGENTS.md` 复制过来的，描述的是那个 TypeScript/npm 项目的规则（packages、changelog、发布流程）——跟这里不适用。先保留原样，之后要么替换成 pi-rs 专属的规则，要么删掉。
