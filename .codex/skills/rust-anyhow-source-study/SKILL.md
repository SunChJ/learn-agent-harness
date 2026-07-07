---
name: rust-anyhow-source-study
description: "Use when continuing the learn-anyhow Rust study workflow in the learn-mine-cli repo: explain anyhow source code through small runnable Cargo examples, connect each concept to relevant Rust standard library source when useful, and persist lessons as examples plus notes under learn-anyhow."
---

# Rust Anyhow Source Study

## Overview

Use this skill to continue a source-driven Rust learning path centered on `anyhow`. The goal is not to summarize everything, but to turn each concrete confusion into a small runnable example, a focused note, and, when valuable, a trace into Rust standard library source.

## Repository Conventions

Work inside the current `learn-mine-cli` repository.

Use `learn-anyhow` as the learning package:

```text
learn-anyhow/
├── Cargo.toml
├── examples/
│   ├── mini-anyhow.rs
│   └── 02-main-error-printing.rs
├── 01-box-dyn-error.md
└── 02-main-error-printing.md
```

Add new lessons as paired files:

```text
learn-anyhow/examples/03-topic-name.rs
learn-anyhow/03-topic-name.md
```

Prefer Cargo commands:

```bash
cargo run --example 03-topic-name -p learn-anyhow
cargo check --example 03-topic-name -p learn-anyhow
```

## Learning Workflow

1. Start from the user's concrete confusion.

Example: "`main() -> Result<()>` 为什么用 Debug 打印？"

2. Identify the smallest relevant source path.

For `anyhow`, prefer local crate source:

```text
~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/anyhow-1.0.103/src/
```

For Rust standard library source, derive the sysroot:

```bash
rustc --print sysroot
```

Then inspect paths under:

```text
lib/rustlib/src/rust/library/
```

3. Explain the runtime or type-system path as a chain.

Good shape:

```text
read_to_string
  -> returns io::Result<String>
  -> ? converts io::Error into MiniError
  -> main returns Err(MiniError)
  -> std::process::Termination for Result<T, E>
  -> requires E: Debug
  -> prints Error: {err:?}
```

4. Build or extend one runnable example.

Keep examples small. A good example isolates one mechanism: `From`, `Display` vs `Debug`, `source()`, `Box<dyn Error>`, `downcast_ref`, or `Context`.

5. Persist the lesson in a paired Markdown note.

The note should include:

- the command to run
- the exact concept being demonstrated
- the relevant code excerpt
- the source-code chain
- the takeaway in beginner-friendly language

## Rust Source Study Heuristics

Prefer question-driven source reading over broad source tours.

Use these bridges from `anyhow` into Rust source:

- `?` and conversion: `core::result::Result`, `From`, `Try`, `FromResidual`
- `main() -> Result`: `std::process::Termination`
- file reading: `std::fs::read_to_string`, `std::fs::File`, `std::io::Read`
- error traits: `core::error::Error`, `std::error::Error`
- trait objects: `dyn Trait`, `Box`, pointer metadata, vtables
- downcasting: `Any`, `TypeId`, `Error::downcast_ref`

Do not over-explain compiler internals unless the user's question requires it. First give the practical mental model, then point to the source location that proves it.

## Anyhow Study Sequence

Continue in roughly this order unless the user asks a more specific question:

1. `?` and `From<E> for MiniError`
2. `main() -> Result` and `Debug` printing
3. manual `Debug` for `MiniError`
4. `std::error::Error::source`
5. adding a `ContextError<C, E>` wrapper
6. `chain()` iteration
7. `downcast_ref` and `TypeId`
8. why real `anyhow::Error` uses its own thin pointer and vtable

## Style

Use Chinese explanations by default.

Keep each lesson concrete. Avoid saying "this is magic"; instead say which trait, impl, macro, or runtime hook is responsible.

When adding code, prefer examples that compile with stable Rust and require no external dependency unless the lesson is specifically about `anyhow` itself.

When validating, prefer targeted commands:

```bash
cargo check --example NAME -p learn-anyhow
cargo run --example NAME -p learn-anyhow
```

If the example is supposed to fail at runtime, use `cargo check` for validation and explain that `cargo run` should produce an error.
