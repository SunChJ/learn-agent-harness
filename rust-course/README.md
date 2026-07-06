# Rust Course Labs

These labs replace the old Python-first warm-up path. They are intentionally small and deterministic: no API key, no network, just the harness mechanics in Rust.

Run from the repository root:

```bash
cargo run -p rust-course --bin m0_hello_rust
cargo run -p rust-course --bin m2_stream_errors
cargo run -p rust-course --bin m3_agent_loop
cargo run -p rust-course --bin m6_compaction
cargo test
```

Use these files as scratch space before moving ideas into `pi-rs/`. If a later milestone has no lab yet, add a small bin here first, keep it focused, and only then productize the design in the real rewrite.

