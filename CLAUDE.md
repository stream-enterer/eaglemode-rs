# EGOPOL

Cargo workspace with two crates:
- `zuicchini/` — UI framework library (reimplementation of Eagle Mode's emCore in Rust)
- `egopol/` — game binary, depends on zuicchini via path
- `~/.local/git/eaglemode-0.96.4` — Eagle Mode C++ source code

## Commands

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo run -p egopol
```

## Pre-commit hook

Runs `cargo fmt` (auto-applied) then `clippy -D warnings` then `cargo test`.
Do not skip with `--no-verify`. If a commit fails, fix the cause and retry.

## Code Rules

- **Types & coordinates**: `f64` logical, `i32` pixel, `u32` image dims, `u8` color channels.
- **Color**: `Color` (packed u32 RGBA) for storage. Intermediate blend math in `i32` or wider.
- **Ownership**: `Rc`/`RefCell` shared state, `Weak` parent refs.
- **Strings**: `String` owned, `&str` params. Convert with `.to_string()`.
- **Errors**: Per-module `Result` with custom error enums (`Display` + `Error`). `assert!` only for logic-error invariants.
- **Imports**: std → external → `crate::`. Explicit names. `use super::*` only in `#[cfg(test)]`.
- **Construction**: `new()` primary, builder `with_*(self) -> Self` for optional config.
- **Modules**: One primary type per file. Private `mod` + public `use` re-exports in `mod.rs`.
- **Visibility**: `pub(crate)` default. `pub` only for library API consumed by `egopol`.
- **Unwrap**: `expect("reason")` unless invariant is obvious from context. Bare `unwrap()` fine in tests and same-line proofs.
- **Warnings**: Fix the cause (remove dead code, prefix `_`, apply clippy fix). Suppress only genuine false positives with a comment.

## Compile-first for type questions

When exploring unfamiliar external library APIs (not project code), prefer writing
a best-guess usage and running `cargo check` over researching documentation or
reading source files to answer type-level questions (method names, return types,
import paths, trait bounds, struct fields).

This applies because:
- `cargo check` is sub-second, reversible, and version-correct for this project
- Training data and web docs may describe wrong versions of skrifa, wgpu, etc.
- The pre-commit hook makes compilation unavoidable; front-load it

This does NOT replace reading when you need to understand:
- Semantic behavior, preconditions, or side effects of an API
- Performance characteristics (allocation, complexity, caching)
- Architectural patterns in zuicchini's own codebase (panel trees, VIFs, behaviors)
- Correct values for parameters where the type is right but the semantics matter
  (e.g., rustybuzz Direction/Script, wgpu buffer formats, RefCell borrow lifetimes)

The only qualifying check is `cargo check`. Do not extend this to `cargo test`,
`cargo run`, or any operation that is slow, may have side effects, or allocates
GPU resources.

## Do NOT

- `#[allow(...)]` / `#[expect(...)]` — fix the warning instead
- `Arc` / `Mutex` — single-threaded UI tree
- `Cow` — use `String` / `&str`
- Glob imports (`use foo::*`) — except `use super::*` in tests
- Truncate color math to `u8` mid-calculation
- `assert!` for recoverable errors
- `--no-verify` on commits
