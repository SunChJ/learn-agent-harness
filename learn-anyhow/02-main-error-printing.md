# 为什么 main 返回错误时打印 Debug

运行成功例子：

```bash
cargo run --example mini-anyhow -p learn-anyhow
```

它会打印 `config.toml` 的内容，因为文件存在：

```text
name = "mini-anyhow"
```

运行失败例子：

```bash
cargo run --example 02-main-error-printing -p learn-anyhow
```

这个例子故意读取不存在的文件：

```rust
fn read_missing_config() -> Result<String> {
    let path = format!("{}/missing.toml", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(path)?;
    Ok(text)
}
```

所以 `?` 会把 `std::io::Error` 转成 `MiniError`，然后从 `main` 返回：

```rust
fn main() -> Result<()> {
    read_missing_config()?;
    Ok(())
}
```

## 关键现象

你可能以为它会调用 `Display`：

```rust
impl fmt::Display for MiniError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}
```

也就是只打印：

```text
No such file or directory (os error 2)
```

但实际上，`main() -> Result<(), E>` 返回 `Err(e)` 时，标准库会用 `Debug` 格式打印错误。

所以输出更接近：

```text
Error: MiniError { inner: Os { code: 2, kind: NotFound, message: "No such file or directory" } }
```

这里用的是：

```rust
#[derive(Debug)]
struct MiniError {
    inner: Box<dyn StdError + Send + Sync + 'static>,
}
```

## Display 和 Debug 的区别

`Display` 对应：

```rust
println!("{}", err);
```

它面向用户，通常是一句清楚的人类可读错误。

`Debug` 对应：

```rust
println!("{:?}", err);
```

它面向开发者，通常会暴露结构体字段和内部细节。

`main() -> Result<(), E>` 的默认错误输出走的是 `Debug`，不是 `Display`。

## 这和 anyhow 有什么关系

`anyhow::Error` 自己实现了定制版 `Debug`。

所以 `anyhow` 从 `main` 返回错误时，虽然标准库也是走 `Debug`，但 `anyhow` 的 `Debug` 输出被专门设计成更适合错误报告：

```text
Error: Failed to read config

Caused by:
    No such file or directory (os error 2)
```

我们的 `MiniError` 现在只是 `#[derive(Debug)]`，所以它打印得比较原始。这正好引出下一步学习：

```text
自己为 MiniError 实现 Debug，让它更像 anyhow::Error。
```
