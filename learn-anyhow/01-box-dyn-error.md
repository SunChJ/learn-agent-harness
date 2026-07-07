# 从 MiniError 理解 Box<dyn Error + Send + Sync + 'static>

这篇笔记对应下面这个最小版错误类型：

```rust
use std::error::Error as StdError;
use std::fmt;

type Result<T> = std::result::Result<T, MiniError>;

#[derive(Debug)]
struct MiniError {
    inner: Box<dyn StdError + Send + Sync + 'static>,
}

impl MiniError {
    fn new<E>(err: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        MiniError {
            inner: Box::new(err),
        }
    }
}

impl fmt::Display for MiniError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}
```

## 1. `#[derive(Debug)]` 是为了什么

Rust 的标准错误 trait 近似可以理解为：

```rust
pub trait Error: Debug + Display {
    // ...
}
```

也就是说，一个类型想实现 `std::error::Error`，必须同时能被 `Debug` 和 `Display` 打印。

`#[derive(Debug)]` 是让编译器自动为 `MiniError` 生成 `Debug` 实现，这样后面才能写：

```rust
impl std::error::Error for MiniError {}
```

`Debug` 通常是给开发者看的，比如：

```rust
println!("{:?}", err);
```

## 2. 为什么字段叫 `inner`

`inner` 不是语法要求，只是工程习惯。

```rust
struct MiniError {
    inner: Box<dyn StdError + Send + Sync + 'static>,
}
```

它表达的是：`MiniError` 是一个外壳，真正的错误对象被包在里面。

字段名也可以叫 `source`、`error`、`cause`、`boxed`。但 `inner` 很常见，意思是“内部被包装的对象”。

## 3. `Box<dyn StdError + Send + Sync + 'static>` 拆开看

### `Box`

不同错误类型大小不同：

```rust
std::io::Error
serde_json::Error
MyCustomError
```

结构体字段必须有确定大小。`Box<T>` 本身是一个指针大小，所以可以把不同大小的具体错误放到堆上，然后结构体里只保存这个指针。

### `dyn StdError`

`dyn StdError` 是 trait object，表示：

```text
某个实现了 std::error::Error 的具体类型，但现在把具体类型隐藏起来。
```

如果你知道具体类型，不需要 `dyn`：

```rust
struct IoErrorBox {
    inner: Box<std::io::Error>,
}
```

如果你要装“任意标准错误”，就需要：

```rust
Box<dyn StdError>
```

现代 Rust 中 trait object 需要显式写 `dyn`。

### `Send`

`Send` 表示这个值可以被移动到另一个线程。

比如一个错误要通过 channel 发给另一个线程，或者进入 async runtime，通常需要 `Send`。

### `Sync`

`Sync` 表示这个值的共享引用可以跨线程使用。

更直观地说：

```text
T: Sync
```

意味着：

```text
&T: Send
```

也就是 `&T` 可以被发送到另一个线程。

### `'static`

`'static` 在这里是生命周期约束，不是 trait。

它表示这个错误对象不能依赖某个短生命周期引用。

可以：

```rust
struct MyError {
    msg: String,
}
```

因为 `String` 拥有数据。

容易出问题：

```rust
struct MyError<'a> {
    msg: &'a str,
}
```

因为它借用了外部数据，外部数据可能比错误对象先失效。

## 4. `Send + Sync + 'static` 是在哪里实现的

它们不是 `anyhow` 或 `Box` 给你实现的。

这段：

```rust
where
    E: StdError + Send + Sync + 'static,
```

是在要求传进来的具体错误类型 `E` 已经满足这些条件。

`Send` 和 `Sync` 是 auto trait。很多时候编译器会自动判断。

例如：

```rust
#[derive(Debug)]
struct MyError {
    msg: String,
}
```

因为 `String` 是 `Send + Sync`，所以 `MyError` 通常也自动是 `Send + Sync`。

但这个不行：

```rust
use std::rc::Rc;

#[derive(Debug)]
struct MyError {
    data: Rc<String>,
}
```

`Rc<T>` 不是线程安全的，所以这个 `MyError` 也不是 `Send + Sync`。

`'static` 也来自类型本身是否持有短生命周期引用。如果类型拥有自己的数据，通常更容易满足 `'static`。

## 5. 为什么要写 `impl MiniError`

`impl MiniError` 是在给 `MiniError` 定义自己的关联函数或方法。

```rust
impl MiniError {
    fn new<E>(err: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        MiniError {
            inner: Box::new(err),
        }
    }
}
```

这里的 `new` 是构造函数风格。Rust 没有固定构造函数语法，社区习惯写：

```rust
MiniError::new(...)
```

`E` 是泛型，表示“某一种具体错误类型”。

调用：

```rust
MiniError::new(std::io::Error::last_os_error())
```

此时：

```text
E = std::io::Error
```

`Self` 在这个 `impl` 块里就是 `MiniError`。

## 6. `impl fmt::Display for MiniError` 是为了什么

`Display` 决定 `{}` 怎么打印：

```rust
println!("{}", err);
```

这段实现：

```rust
impl fmt::Display for MiniError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}
```

意思是：

```text
MiniError 自己不创造新的错误文案，直接把内部真实错误的 Display 输出转发出去。
```

如果内部是 `std::io::Error`，它打印：

```text
No such file or directory
```

那么 `MiniError` 也打印这个。

`Display` 通常给用户看，`Debug` 通常给开发者看。

## 7. 常见 trait object 组合

简单例子：

```rust
Box<dyn StdError>
```

通用应用错误：

```rust
Box<dyn StdError + Send + Sync + 'static>
```

只要求可跨线程移动：

```rust
Box<dyn StdError + Send>
```

要求跨线程移动和共享：

```rust
Box<dyn StdError + Send + Sync>
```

允许借用外部数据：

```rust
Box<dyn StdError + 'a>
```

但这样外层错误类型通常也要带生命周期参数：

```rust
struct MiniError<'a> {
    inner: Box<dyn StdError + 'a>,
}
```

这会让错误类型更难在应用里到处传递，所以 `anyhow` 选择了更严格、更省心的 `'static`。

## 8. 当前阶段要记住的核心

`MiniError` 做的是：

```text
把各种不同的具体错误类型统一装进一个错误类型里。
```

靠的是：

```text
Box: 统一大小
dyn StdError: 类型擦除
Send + Sync: 线程安全约束
'static: 不依赖短生命周期引用
Display / Debug: 符合标准错误的打印要求
```

这就是理解 `anyhow::Error` 的第一层。
