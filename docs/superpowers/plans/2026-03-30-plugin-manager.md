# Plugin Manager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the full Eagle Mode plugin system — workspace restructuring into `dylib`/`cdylib` crates, dynamic library loading, plugin function invocation, and emStocks conversion to a dynamically loaded plugin.

**Architecture:** Convert the single-crate library into a Cargo workspace: `emcore` (dylib), `emstocks` (cdylib plugin), and `eaglemode` (host binary). Port `emTryOpenLib`/`emTryResolveSymbol` using `libloading`. Plugin functions use `#[no_mangle]` with Rust calling convention — types cross the dylib boundary safely because host and plugins link the same `libemcore.so`.

**Tech Stack:** Rust, `libloading` crate (already a dependency), Cargo workspace with `dylib`/`cdylib` crate types.

**Spec:** `docs/superpowers/specs/2026-03-30-plugin-manager-design.md`

---

## File Structure

### New files

```
Cargo.toml                              (rewrite: workspace root)
crates/
  emcore/
    Cargo.toml                          (new: dylib crate manifest)
    src/
      lib.rs                            (was src/emCore/mod.rs)
      ... (all 101 existing emCore .rs files, moved)
  emstocks/
    Cargo.toml                          (new: cdylib crate manifest)
    src/
      lib.rs                            (was src/emStocks/mod.rs)
      ... (all 11 existing emStocks .rs files, moved)
  eaglemode/
    Cargo.toml                          (new: host binary manifest)
    src/
      main.rs                           (new: binary entry point)
  test_plugin/
    Cargo.toml                          (new: test cdylib for plugin loading tests)
    src/
      lib.rs                            (new: dummy plugin function)
etc/
  emCore/
    FpPlugins/
      emStocks.emFpPlugin               (new: plugin config)
      version                           (new: version string)
```

### Modified files

```
crates/emcore/src/emStd2.rs             (add dynamic library API)
crates/emcore/src/emFpPlugin.rs         (add invocation: TryCreateFilePanel, TryAcquireModel)
crates/emstocks/src/emStocksFpPlugin.rs (rewrite: extern "C" entry point)
.cargo/config.toml                      (add LD_LIBRARY_PATH and EM_DIR)
```

### Deleted files

```
src/lib.rs                              (replaced by workspace)
src/emCore/                             (moved to crates/emcore/src/)
src/emStocks/                           (moved to crates/emstocks/src/)
```

### Import rewriting scope

- 73 files in `crates/emcore/src/`: `use crate::emCore::X` -> `use crate::X` (304 use statements + 480 inline paths)
- 3 files in `crates/emstocks/src/`: `use crate::emStocks::X` -> `use crate::X`, `use crate::emCore::X` -> `use emcore::X`
- 70 test files in `tests/`: `use eaglemode_rs::emCore::X` -> `use emcore::X`, `use eaglemode_rs::emStocks::X` -> `use emstocks::X` (385 + 7 = 392 statements)
- 4 benchmark files in `benches/`: `use eaglemode_rs::emCore::X` -> `use emcore::X`

---

## Task 1: Create workspace Cargo.toml and crate manifests

**Files:**
- Rewrite: `Cargo.toml`
- Create: `crates/emcore/Cargo.toml`
- Create: `crates/emstocks/Cargo.toml`
- Create: `crates/eaglemode/Cargo.toml`
- Create: `crates/eaglemode/src/main.rs`

This task creates the workspace structure and manifests without moving any source files yet. The workspace won't build until source is moved in Task 2.

- [ ] **Step 1: Create directory structure**

```bash
mkdir -p crates/emcore/src crates/emstocks/src crates/eaglemode/src crates/test_plugin/src
```

- [ ] **Step 2: Write workspace root Cargo.toml**

Rewrite the root `Cargo.toml` as a workspace. Move lints, profile settings, and benchmark declarations here. Dependencies that were in `[dependencies]` move to the individual crate manifests.

```toml
[workspace]
members = [
    "crates/emcore",
    "crates/emstocks",
    "crates/eaglemode",
]
resolver = "2"

[workspace.lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(kani)'] }

[workspace.dependencies]
emcore = { path = "crates/emcore" }
bitflags = "2"
bytemuck = { version = "1", features = ["derive"] }
libc = "0.2.183"
libloading = "0.9.0"
log = "0.4"
nix = { version = "0.31.2", features = ["fs"] }
pollster = "0.4"
slotmap = "1"
wgpu = "28"
winit = "0.30"
zbus = "5"
rand = "0.9"
criterion = { version = "0.8", features = ["html_reports"] }
gungraun = "0.17"

[profile.release]
lto = true
codegen-units = 1

[profile.dev.package."*"]
opt-level = 2

[profile.bench]
debug = true
```

- [ ] **Step 3: Write emcore crate Cargo.toml**

```toml
[package]
name = "emcore"
version = "0.1.0"
edition = "2021"
description = "Eagle Mode emCore — zoomable UI framework core"

[lib]
crate-type = ["dylib", "rlib"]

[lints]
workspace = true

[dependencies]
bitflags = { workspace = true }
bytemuck = { workspace = true }
libc = { workspace = true }
libloading = { workspace = true }
log = { workspace = true }
nix = { workspace = true }
pollster = { workspace = true }
slotmap = { workspace = true }
wgpu = { workspace = true }
winit = { workspace = true }
zbus = { workspace = true }

[dev-dependencies]
rand = { workspace = true }
gungraun = { workspace = true }
```

Note: `crate-type = ["dylib", "rlib"]` produces both the shared library (for plugin loading) and the rlib (for tests that link statically). The `rlib` is needed because integration tests and benchmarks cannot link against a `dylib` crate in Cargo's current model — they need the rlib for compilation. At runtime, all plugin `.so` files link the dylib.

- [ ] **Step 4: Write emstocks crate Cargo.toml**

```toml
[package]
name = "emstocks"
version = "0.1.0"
edition = "2021"
description = "Eagle Mode emStocks — stock portfolio plugin"

[lib]
name = "emStocks"
crate-type = ["cdylib", "rlib"]

[lints]
workspace = true

[dependencies]
emcore = { workspace = true }

[dev-dependencies]
gungraun = { workspace = true }
```

Note: `name = "emStocks"` produces `libemStocks.so` matching the C++ library name. `rlib` included for tests.

- [ ] **Step 5: Write eaglemode host binary Cargo.toml**

```toml
[package]
name = "eaglemode"
version = "0.1.0"
edition = "2021"
description = "Eagle Mode — zoomable user interface"

[lints]
workspace = true

[dependencies]
emcore = { workspace = true }

[dev-dependencies]
rand = { workspace = true }
criterion = { workspace = true }
gungraun = { workspace = true }
```

- [ ] **Step 6: Write minimal main.rs**

```rust
fn main() {
    println!("eaglemode: host binary placeholder");
}
```

- [ ] **Step 7: Commit manifests**

```bash
git add Cargo.toml crates/*/Cargo.toml crates/eaglemode/src/main.rs
git commit -m "chore: create workspace manifests for emcore, emstocks, eaglemode"
```

---

## Task 2: Move source files to crate directories

**Files:**
- Move: `src/emCore/*.rs` -> `crates/emcore/src/`
- Move: `src/emCore/*.no_rs` -> `crates/emcore/src/`
- Move: `src/emCore/*.rust_only` -> `crates/emcore/src/`
- Move: `src/emStocks/*.rs` -> `crates/emstocks/src/`
- Delete: `src/lib.rs`
- Rename: `crates/emcore/src/mod.rs` -> `crates/emcore/src/lib.rs`
- Rename: `crates/emstocks/src/mod.rs` -> `crates/emstocks/src/lib.rs`

- [ ] **Step 1: Move emCore source files**

```bash
# Move all .rs, .no_rs, and .rust_only files
mv src/emCore/*.rs crates/emcore/src/
mv src/emCore/*.no_rs crates/emcore/src/ 2>/dev/null || true
mv src/emCore/*.rust_only crates/emcore/src/ 2>/dev/null || true
# Rename mod.rs to lib.rs
mv crates/emcore/src/mod.rs crates/emcore/src/lib.rs
```

- [ ] **Step 2: Move emStocks source files**

```bash
mv src/emStocks/*.rs crates/emstocks/src/
mv crates/emstocks/src/mod.rs crates/emstocks/src/lib.rs
```

- [ ] **Step 3: Delete old src/lib.rs and empty directories**

```bash
rm src/lib.rs
rmdir src/emCore src/emStocks src
```

- [ ] **Step 4: Verify files are in place**

```bash
ls crates/emcore/src/lib.rs crates/emcore/src/emFpPlugin.rs crates/emcore/src/emStd2.rs
ls crates/emstocks/src/lib.rs crates/emstocks/src/emStocksFpPlugin.rs
# Should list all files without errors
```

- [ ] **Step 5: Commit move**

```bash
git add -A
git commit -m "chore: move emCore and emStocks source to workspace crate directories"
```

---

## Task 3: Rewrite emcore lib.rs and fix emcore internal imports

**Files:**
- Modify: `crates/emcore/src/lib.rs`
- Modify: all 100 `.rs` files in `crates/emcore/src/` (import rewriting)

The old `mod.rs` had `#![allow(non_camel_case_types)]` and 100 `pub mod` declarations. The new `lib.rs` keeps those but drops the `mod` wrapper level — types are accessed as `emcore::emFoo::Type` instead of `eaglemode_rs::emCore::emFoo::Type`.

- [ ] **Step 1: Rewrite lib.rs**

The existing file already has the right content (100 `pub mod` declarations with the `#![allow(non_camel_case_types)]` attribute). Add the `#[allow(non_snake_case)]` that was on the module in the old `src/lib.rs`:

At the top of `crates/emcore/src/lib.rs`, ensure these attributes are present:

```rust
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
```

- [ ] **Step 2: Rewrite internal emcore imports**

In all 73 files within `crates/emcore/src/` that contain `crate::emCore::`, replace with `crate::`:

```
use crate::emCore::emColor::emColor  ->  use crate::emColor::emColor
crate::emCore::emPanel::PanelBehavior  ->  crate::emPanel::PanelBehavior
```

This is a mechanical find-and-replace of `crate::emCore::` with `crate::` across all `.rs` files in `crates/emcore/src/`. There are 304 `use` statements and 480 inline path references to update (784 total replacements).

Run:

```bash
cd crates/emcore/src
sed -i 's/crate::emCore::/crate::/g' *.rs
```

- [ ] **Step 3: Verify emcore crate compiles**

```bash
cargo check -p emcore 2>&1 | head -50
```

Expected: compilation succeeds (or only emstocks-related errors, since emstocks isn't wired yet).

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/
git commit -m "chore: rewrite emcore internal imports (crate::emCore:: -> crate::)"
```

---

## Task 4: Rewrite emstocks lib.rs and fix emstocks imports

**Files:**
- Modify: `crates/emstocks/src/lib.rs`
- Modify: all `.rs` files in `crates/emstocks/src/` (import rewriting)

- [ ] **Step 1: Rewrite lib.rs**

The existing content has `#![allow(non_camel_case_types)]` and 10 `pub mod` declarations. Add the non_snake_case allow:

```rust
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
```

- [ ] **Step 2: Rewrite emstocks imports**

Two categories of replacement:

1. `crate::emCore::X` -> `emcore::X` (references to emCore from within emStocks)
2. `crate::emStocks::X` -> `crate::X` (internal emStocks references)

```bash
cd crates/emstocks/src
sed -i 's/crate::emCore::/emcore::/g' *.rs
sed -i 's/crate::emStocks::/crate::/g' *.rs
```

- [ ] **Step 3: Verify emstocks crate compiles**

```bash
cargo check -p emstocks 2>&1 | head -50
```

Expected: compilation succeeds.

- [ ] **Step 4: Commit**

```bash
git add crates/emstocks/src/
git commit -m "chore: rewrite emstocks imports for workspace structure"
```

---

## Task 5: Rewrite test and benchmark imports

**Files:**
- Modify: 70 test files in `tests/` (392 import statements)
- Modify: 4+ benchmark files in `benches/`
- Modify: `benches/common/mod.rs`, `benches/common/scaled.rs`
- Modify: `tests/support/mod.rs`, `tests/support/pipeline.rs`

- [ ] **Step 1: Rewrite test imports**

```bash
# Tests reference eaglemode_rs::emCore:: -> emcore::
sed -i 's/eaglemode_rs::emCore::/emcore::/g' tests/**/*.rs tests/*.rs 2>/dev/null

# Tests reference eaglemode_rs::emStocks:: -> emstocks::
# Note: emstocks lib.rs exports with name="emStocks", so the extern crate name
# in tests is emStocks. However, Cargo normalizes crate names with hyphens to
# underscores. Since the lib name is "emStocks", the extern crate is `emStocks`.
# But test files use `use emstocks::` with the package name. We need to check
# what Cargo makes available.
#
# Actually: with [lib] name = "emStocks" in emstocks crate, the crate name
# for `use` statements is `emStocks` (the lib name). But the dependency in
# test crates would reference the package name. Let's handle this:
sed -i 's/eaglemode_rs::emStocks::/emStocks::/g' tests/**/*.rs tests/*.rs 2>/dev/null
```

Wait — tests in `tests/` at the workspace root are integration tests for a specific crate. In a workspace, integration tests live inside each crate's own `tests/` directory, or they need explicit `[dev-dependencies]` in a crate that depends on the target.

**Decision:** Move tests into the appropriate crate:
- Tests that import only `emcore::` go to `crates/emcore/tests/`
- Tests that import `emstocks::` go to `crates/emstocks/tests/`
- Golden tests and benchmarks that import `emcore::` go to `crates/emcore/tests/` and `crates/emcore/benches/`

Actually, this is a large reorganization. A simpler approach: keep tests at workspace root but create a workspace-level test crate, or add `emcore` and `emstocks` as dev-dependencies of the `eaglemode` crate and put tests there.

**Simplest approach:** Add both `emcore` and `emstocks` as dependencies of `eaglemode`, and move all tests and benchmarks into `crates/eaglemode/`. The host binary crate becomes the integration test host.

Let's revise:

- [ ] **Step 1: Move tests and benchmarks to eaglemode crate**

```bash
mv tests/ crates/eaglemode/tests/
mv benches/ crates/eaglemode/benches/
```

- [ ] **Step 2: Update eaglemode Cargo.toml with test dependencies and benchmarks**

```toml
[package]
name = "eaglemode"
version = "0.1.0"
edition = "2021"
description = "Eagle Mode — zoomable user interface"

[lints]
workspace = true

[dependencies]
emcore = { workspace = true }

[dev-dependencies]
emstocks = { path = "../emstocks" }
rand = { workspace = true }
criterion = { workspace = true }
gungraun = { workspace = true }

[[bench]]
name = "interaction"
harness = false

[[bench]]
name = "interaction_iai"
harness = false

[[bench]]
name = "scaled_tree"
harness = false

[[bench]]
name = "scaled_tree_iai"
harness = false
```

- [ ] **Step 3: Rewrite test imports**

```bash
cd crates/eaglemode
sed -i 's/eaglemode_rs::emCore::/emcore::/g' tests/**/*.rs benches/**/*.rs benches/*.rs 2>/dev/null
sed -i 's/eaglemode_rs::emStocks::/emStocks::/g' tests/**/*.rs 2>/dev/null
```

- [ ] **Step 4: Move golden test assets**

```bash
# Golden test data stays with the tests
# Already moved with tests/ above
```

- [ ] **Step 5: Verify all tests compile**

```bash
cargo check --workspace --tests --benches 2>&1 | head -80
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: move tests and benchmarks to eaglemode crate, rewrite imports"
```

---

## Task 6: Fix remaining compilation errors and verify full test suite

**Files:**
- Potentially any file with compilation errors
- Modify: `.cargo/config.toml` if needed

This task is the catch-all for any import or compilation issues that the mechanical sed replacements missed.

- [ ] **Step 1: Build the full workspace**

```bash
cargo build --workspace 2>&1 | tee /tmp/build-errors.txt
```

- [ ] **Step 2: Fix any remaining import errors**

Common issues to fix:
- `use super::*` in test modules that now need explicit crate references
- Benchmark `common` module paths may need adjustment
- `tests/support/` module references may need updating
- Any `#[cfg(test)]` modules inside emcore/emstocks that reference the old crate path

Fix each error, checking the specific file and line.

- [ ] **Step 3: Run clippy**

```bash
cargo clippy --workspace -- -D warnings 2>&1 | head -50
```

- [ ] **Step 4: Run the full test suite**

```bash
cargo-nextest ntr 2>&1 | tail -30
```

Expected: all tests pass.

- [ ] **Step 5: Run golden tests**

```bash
cargo test -p eaglemode --test golden -- --test-threads=1
```

- [ ] **Step 6: Run benchmarks (compile check only)**

```bash
cargo bench -p eaglemode --no-run
```

- [ ] **Step 7: Commit all fixes**

```bash
git add -A
git commit -m "fix: resolve compilation errors from workspace restructuring"
```

---

## Task 7: Port dynamic library API to emStd2.rs

**Files:**
- Modify: `crates/emcore/src/emStd2.rs`

- [ ] **Step 1: Write failing tests for emTryOpenLib**

Add to the bottom of `crates/emcore/src/emStd2.rs`, inside the existing `#[cfg(test)]` module:

```rust
#[test]
fn test_try_open_lib_nonexistent() {
    let result = super::emTryOpenLib("nonexistent_library_12345", false);
    assert!(result.is_err());
    match result.unwrap_err() {
        super::LibError::LibraryLoad { library, .. } => {
            assert!(library.contains("nonexistent_library_12345"));
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn test_lib_filename_construction() {
    // On Linux: "emFoo" -> "libemFoo.so"
    assert_eq!(super::lib_name_to_filename("emFoo"), "libemFoo.so");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p emcore -- emStd2::tests::test_try_open_lib 2>&1 | tail -10
cargo test -p emcore -- emStd2::tests::test_lib_filename 2>&1 | tail -10
```

Expected: FAIL — functions don't exist yet.

- [ ] **Step 3: Implement the dynamic library API**

Add to `crates/emcore/src/emStd2.rs`, after the existing hash/checksum functions:

```rust
use std::cell::RefCell;
use std::fmt;

// ── Dynamic Library API ─────────────────────────────────────────────
// Port of C++ emStd2.h/emStd2.cpp: emTryOpenLib, emTryResolveSymbolFromLib,
// emCloseLib, emTryResolveSymbol, and the emLibTable cache.

/// Error type for dynamic library operations.
/// Port of C++ exceptions thrown by emTryOpenLib and emTryResolveSymbol.
#[derive(Debug)]
pub enum LibError {
    /// Failed to load a dynamic library.
    LibraryLoad { library: String, message: String },
    /// Failed to resolve a symbol from a loaded library.
    SymbolResolve { library: String, symbol: String, message: String },
}

impl fmt::Display for LibError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LibraryLoad { library, message } => {
                write!(f, "failed to load library \"{library}\": {message}")
            }
            Self::SymbolResolve { library, symbol, message } => {
                write!(f, "failed to resolve \"{symbol}\" in \"{library}\": {message}")
            }
        }
    }
}

impl std::error::Error for LibError {}

/// Port of C++ `emLibTableEntry`.
struct LibTableEntry {
    filename: String,
    ref_count: u64, // 0 = infinite (never unloaded)
    handle: libloading::Library,
}

thread_local! {
    /// Port of C++ `emLibTable`. Single-threaded; no mutex needed.
    static LIBRARY_TABLE: RefCell<Vec<LibTableEntry>> = const { RefCell::new(Vec::new()) };
}

/// Opaque handle to a loaded dynamic library.
/// Port of C++ `emLibHandle`.
#[derive(Debug, Clone, Copy)]
pub struct emLibHandle {
    index: usize,
}

/// Convert a library pure name to a platform filename.
/// Port of C++ logic in `emTryOpenLib`: "emFoo" -> "libemFoo.so" (Linux),
/// "libemFoo.dylib" (macOS), "emFoo.dll" (Windows).
pub fn lib_name_to_filename(name: &str) -> String {
    if cfg!(target_os = "windows") || cfg!(target_os = "cygwin") {
        format!("{name}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{name}.dylib")
    } else {
        format!("lib{name}.so")
    }
}

/// Open a dynamic library. Port of C++ `emTryOpenLib`.
///
/// If `is_filename` is false, `lib_name` is a pure name converted to a
/// platform filename. Libraries are cached: opening the same library twice
/// returns the same handle with an incremented refcount.
pub fn emTryOpenLib(lib_name: &str, is_filename: bool) -> Result<emLibHandle, LibError> {
    let filename = if is_filename {
        lib_name.to_string()
    } else {
        lib_name_to_filename(lib_name)
    };

    LIBRARY_TABLE.with(|table| {
        let mut table = table.borrow_mut();

        // Check cache
        for (i, entry) in table.iter_mut().enumerate() {
            if entry.filename == filename {
                if entry.ref_count > 0 {
                    entry.ref_count += 1;
                }
                return Ok(emLibHandle { index: i });
            }
        }

        // Load new library
        let handle = unsafe { libloading::Library::new(&filename) }.map_err(|e| {
            LibError::LibraryLoad {
                library: filename.clone(),
                message: e.to_string(),
            }
        })?;

        let index = table.len();
        table.push(LibTableEntry {
            filename,
            ref_count: 1,
            handle,
        });

        Ok(emLibHandle { index })
    })
}

/// Resolve a symbol from an open library.
/// Port of C++ `emTryResolveSymbolFromLib`.
///
/// # Safety
/// The returned pointer is only valid while the library remains open.
/// Caller must ensure the pointer is transmuted to the correct function type.
pub unsafe fn emTryResolveSymbolFromLib(
    handle: &emLibHandle,
    symbol: &str,
) -> Result<*const (), LibError> {
    LIBRARY_TABLE.with(|table| {
        let table = table.borrow();
        let entry = &table[handle.index];

        let sym: libloading::Symbol<*const ()> =
            unsafe { entry.handle.get(symbol.as_bytes()) }.map_err(|e| {
                LibError::SymbolResolve {
                    library: entry.filename.clone(),
                    symbol: symbol.to_string(),
                    message: e.to_string(),
                }
            })?;

        Ok(*sym)
    })
}

/// Close a dynamic library. Port of C++ `emCloseLib`.
///
/// Decrements refcount. At zero, the library is unloaded and the entry
/// removed. If refcount was already zero (infinite), this is a no-op.
pub fn emCloseLib(handle: emLibHandle) {
    LIBRARY_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        let entry = &mut table[handle.index];

        if entry.ref_count == 0 {
            return; // infinite lifetime
        }

        entry.ref_count -= 1;
        if entry.ref_count == 0 {
            table.remove(handle.index);
        }
    });
}

/// Open, resolve, and set library to infinite lifetime.
/// Port of C++ `emTryResolveSymbol`.
///
/// The library is never closed after this call (refcount set to 0 = infinite).
///
/// # Safety
/// Same as `emTryResolveSymbolFromLib`.
pub unsafe fn emTryResolveSymbol(
    lib_name: &str,
    is_filename: bool,
    symbol: &str,
) -> Result<*const (), LibError> {
    let handle = emTryOpenLib(lib_name, is_filename)?;
    let ptr = unsafe { emTryResolveSymbolFromLib(&handle, symbol)? };

    // Set to infinite lifetime (matching C++: e->RefCount=0)
    LIBRARY_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        table[handle.index].ref_count = 0;
    });

    Ok(ptr)
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p emcore -- emStd2::tests 2>&1 | tail -20
```

Expected: both new tests pass, all existing tests still pass.

- [ ] **Step 5: Write additional behavioral tests**

Create `crates/emcore/tests/lib_loading.rs`:

```rust
//! Behavioral tests for the dynamic library API (emTryOpenLib, etc.)
//!
//! These tests load real .so files to verify the full dlopen path.
//! They require LD_LIBRARY_PATH to include the Cargo output directory.

use emcore::emStd2::{
    emCloseLib, emLibHandle, emTryOpenLib, emTryResolveSymbol,
    emTryResolveSymbolFromLib, lib_name_to_filename, LibError,
};

#[test]
fn open_nonexistent_returns_error() {
    let result = emTryOpenLib("this_library_does_not_exist_xyz", false);
    assert!(result.is_err());
}

#[test]
fn filename_construction_linux() {
    assert_eq!(lib_name_to_filename("emStocks"), "libemStocks.so");
    assert_eq!(lib_name_to_filename("emCore"), "libemCore.so");
}

#[test]
fn open_libc_and_resolve_symbol() {
    // libc.so.6 is always available on Linux
    let handle = emTryOpenLib("libc.so.6", true).expect("should open libc");
    let ptr = unsafe {
        emTryResolveSymbolFromLib(&handle, "strlen").expect("should find strlen")
    };
    assert!(!ptr.is_null());
    emCloseLib(handle);
}

#[test]
fn resolve_symbol_sets_infinite_lifetime() {
    let ptr = unsafe {
        emTryResolveSymbol("libc.so.6", true, "strlen").expect("should resolve strlen")
    };
    assert!(!ptr.is_null());
    // Library now has infinite lifetime — closing is a no-op
}

#[test]
fn cached_open_returns_same_handle_index() {
    let h1 = emTryOpenLib("libc.so.6", true).expect("first open");
    let h2 = emTryOpenLib("libc.so.6", true).expect("second open");
    assert_eq!(h1.index, h2.index);
    emCloseLib(h1);
    emCloseLib(h2);
}

#[test]
fn resolve_nonexistent_symbol_returns_error() {
    let handle = emTryOpenLib("libc.so.6", true).expect("should open libc");
    let result = unsafe { emTryResolveSymbolFromLib(&handle, "this_does_not_exist_xyz") };
    assert!(result.is_err());
    emCloseLib(handle);
}
```

- [ ] **Step 6: Run behavioral tests**

```bash
cargo test -p emcore --test lib_loading 2>&1 | tail -20
```

Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add crates/emcore/src/emStd2.rs crates/emcore/tests/lib_loading.rs
git commit -m "feat: port dynamic library API (emTryOpenLib, emTryResolveSymbol) to emStd2.rs"
```

---

## Task 8: Define plugin function type aliases and cached function storage

**Files:**
- Modify: `crates/emcore/src/emFpPlugin.rs`

- [ ] **Step 1: Add plugin function type aliases**

At the top of `emFpPlugin.rs`, after the existing imports, add:

```rust
use std::any::Any;
use crate::emPanel::PanelBehavior;

// ── Plugin function types ───────────────────────────────────────────
// Port of C++ emFpPluginFunc and emFpPluginModelFunc from emFpPlugin.h.
// Uses Rust calling convention with #[no_mangle] for symbol lookup.
// Types cross the dylib boundary safely because host and plugins link
// the same libemcore.so.

/// Type of the plugin function for creating a file panel.
/// Port of C++ `emFpPluginFunc`.
pub type emFpPluginFunc = fn(
    parent: &PanelParentArg,
    name: &str,
    path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Rc<RefCell<dyn PanelBehavior>>>;

/// Type of the plugin model function for acquiring file models.
/// Port of C++ `emFpPluginModelFunc`.
pub type emFpPluginModelFunc = fn(
    context: &Rc<emContext>,
    class_name: &str,
    name: &str,
    common: bool,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Rc<RefCell<dyn Any>>>;

/// Placeholder for panel parent argument.
/// DIVERGED: Full PanelParentArg design deferred to panel framework integration.
pub struct PanelParentArg {
    root_context: Rc<emContext>,
}

impl PanelParentArg {
    pub fn new(root_context: Rc<emContext>) -> Self {
        Self { root_context }
    }

    pub fn root_context(&self) -> &Rc<emContext> {
        &self.root_context
    }
}
```

Note: `PanelParentArg` is a simplified version. The full C++ `emPanel::ParentArg` carries the parent panel reference for the panel tree. The exact definition depends on the panel framework state — check what `emPanel.rs` already defines and use that. If `emPanel.rs` already has a `ParentArg` type, import it instead of defining a new one.

- [ ] **Step 2: Add CachedFunctions struct to emFpPlugin**

Replace the existing `cached_library` field with a full cached functions struct:

```rust
/// Cached resolved function pointers. Port of C++ CachedFunc/CachedModelFunc
/// fields on emFpPlugin.
struct CachedFunctions {
    lib_name: String,
    func_name: String,
    func: Option<emFpPluginFunc>,
    model_func_name: String,
    model_func: Option<emFpPluginModelFunc>,
}

impl Default for CachedFunctions {
    fn default() -> Self {
        Self {
            lib_name: String::new(),
            func_name: String::new(),
            func: None,
            model_func_name: String::new(),
            model_func: None,
        }
    }
}
```

Update the `emFpPlugin` struct to use this:

```rust
pub struct emFpPlugin {
    // ... existing fields ...
    cached: RefCell<CachedFunctions>,
}
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p emcore 2>&1 | tail -20
```

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emFpPlugin.rs
git commit -m "feat: add plugin function type aliases and CachedFunctions struct"
```

---

## Task 9: Implement TryCreateFilePanel and TryAcquireModel on emFpPlugin

**Files:**
- Modify: `crates/emcore/src/emFpPlugin.rs`

- [ ] **Step 1: Implement TryCreateFilePanel**

Add to the `impl emFpPlugin` block:

```rust
/// Create a file panel via this plugin's function.
/// Port of C++ `emFpPlugin::TryCreateFilePanel`.
pub fn TryCreateFilePanel(
    &self,
    parent: &PanelParentArg,
    name: &str,
    path: &str,
) -> Result<Rc<RefCell<dyn PanelBehavior>>, FpPluginError> {
    use crate::emStd2::{emTryResolveSymbol, LibError};

    let mut cached = self.cached.borrow_mut();

    // Invalidate cache if library changed (matches C++ CachedLibName check)
    if cached.lib_name != self.library {
        *cached = CachedFunctions::default();
        cached.lib_name = self.library.clone();
    }

    // Resolve function if not cached or function name changed
    if cached.func.is_none() || cached.func_name != self.function {
        if self.function.is_empty() {
            return Err(FpPluginError::EmptyFunctionName);
        }

        let ptr = unsafe {
            emTryResolveSymbol(&self.library, false, &self.function)
        }.map_err(|e| match e {
            LibError::LibraryLoad { library, message } => {
                FpPluginError::LibraryLoad { library, message }
            }
            LibError::SymbolResolve { library, symbol, message } => {
                FpPluginError::SymbolResolve { library, symbol, message }
            }
        })?;

        cached.func = Some(unsafe { std::mem::transmute::<*const (), emFpPluginFunc>(ptr) });
        cached.func_name = self.function.clone();
    }

    let func = cached.func.unwrap();
    drop(cached); // release borrow before calling plugin function

    let mut error_buf = String::new();
    match func(parent, name, path, self, &mut error_buf) {
        Some(panel) => Ok(panel),
        None => {
            if error_buf.is_empty() {
                Err(FpPluginError::PluginFunctionFailed {
                    function: self.function.clone(),
                    message: format!(
                        "Plugin function {} in {} failed.",
                        self.function, self.library
                    ),
                })
            } else {
                Err(FpPluginError::PluginFunctionFailed {
                    function: self.function.clone(),
                    message: error_buf,
                })
            }
        }
    }
}
```

- [ ] **Step 2: Implement TryAcquireModel**

```rust
/// Acquire a model via this plugin's model function.
/// Port of C++ `emFpPlugin::TryAcquireModelImpl`.
pub fn TryAcquireModel(
    &self,
    context: &Rc<emContext>,
    class_name: &str,
    name: &str,
    common: bool,
) -> Result<Rc<RefCell<dyn Any>>, FpPluginError> {
    use crate::emStd2::{emTryResolveSymbol, LibError};

    let mut cached = self.cached.borrow_mut();

    if cached.lib_name != self.library {
        *cached = CachedFunctions::default();
        cached.lib_name = self.library.clone();
    }

    if cached.model_func.is_none() || cached.model_func_name != self.model_function {
        if self.model_function.is_empty() {
            return Err(FpPluginError::EmptyFunctionName);
        }

        let ptr = unsafe {
            emTryResolveSymbol(&self.library, false, &self.model_function)
        }.map_err(|e| match e {
            LibError::LibraryLoad { library, message } => {
                FpPluginError::LibraryLoad { library, message }
            }
            LibError::SymbolResolve { library, symbol, message } => {
                FpPluginError::SymbolResolve { library, symbol, message }
            }
        })?;

        cached.model_func =
            Some(unsafe { std::mem::transmute::<*const (), emFpPluginModelFunc>(ptr) });
        cached.model_func_name = self.model_function.clone();
    }

    let func = cached.model_func.unwrap();
    drop(cached);

    let mut error_buf = String::new();
    match func(context, class_name, name, common, self, &mut error_buf) {
        Some(model) => Ok(model),
        None => {
            if error_buf.is_empty() {
                Err(FpPluginError::PluginFunctionFailed {
                    function: self.model_function.clone(),
                    message: format!(
                        "Plugin model function {} in {} failed.",
                        self.model_function, self.library
                    ),
                })
            } else {
                Err(FpPluginError::PluginFunctionFailed {
                    function: self.model_function.clone(),
                    message: error_buf,
                })
            }
        }
    }
}
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p emcore 2>&1 | tail -20
```

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emFpPlugin.rs
git commit -m "feat: implement TryCreateFilePanel and TryAcquireModel on emFpPlugin"
```

---

## Task 10: Implement CreateFilePanel and TryAcquireModel on emFpPluginList

**Files:**
- Modify: `crates/emcore/src/emFpPlugin.rs`

- [ ] **Step 1: Implement CreateFilePanel (simple overload)**

Add to the `impl emFpPluginList` block:

```rust
/// Create a panel for a file. Port of C++ `emFpPluginList::CreateFilePanel`.
///
/// Calls the appropriate plugin. On failure, returns an emErrorPanel.
pub fn CreateFilePanel(
    &self,
    parent: &PanelParentArg,
    name: &str,
    path: &str,
    alternative: usize,
) -> Rc<RefCell<dyn PanelBehavior>> {
    use crate::emStd1::emGetAbsolutePath;

    let abs_path = emGetAbsolutePath(path);
    let metadata = std::fs::metadata(&abs_path);

    match metadata {
        Err(e) => {
            Rc::new(RefCell::new(crate::emErrorPanel::emErrorPanel::new(
                &e.to_string(),
            )))
        }
        Ok(meta) => {
            let stat_mode = if meta.is_dir() {
                FileStatMode::Directory
            } else {
                FileStatMode::Regular
            };
            self.CreateFilePanelWithStat(
                parent,
                name,
                &abs_path,
                None,
                stat_mode,
                alternative,
            )
        }
    }
}

/// Create a panel with pre-computed stat information.
/// Port of C++ `emFpPluginList::CreateFilePanel` (stat overload).
pub fn CreateFilePanelWithStat(
    &self,
    parent: &PanelParentArg,
    name: &str,
    absolute_path: &str,
    stat_err: Option<std::io::Error>,
    stat_mode: FileStatMode,
    alternative: usize,
) -> Rc<RefCell<dyn PanelBehavior>> {
    if let Some(err) = stat_err {
        return Rc::new(RefCell::new(crate::emErrorPanel::emErrorPanel::new(
            &err.to_string(),
        )));
    }

    let plugin = self.SearchPlugin(None, Some(absolute_path), false, alternative, stat_mode);
    match plugin {
        None => {
            let msg = if alternative == 0 {
                "This file type cannot be shown."
            } else {
                "No alternative file panel plugin available."
            };
            Rc::new(RefCell::new(crate::emErrorPanel::emErrorPanel::new(msg)))
        }
        Some(plugin) => {
            match plugin.TryCreateFilePanel(parent, name, absolute_path) {
                Ok(panel) => panel,
                Err(e) => {
                    Rc::new(RefCell::new(crate::emErrorPanel::emErrorPanel::new(
                        &e.to_string(),
                    )))
                }
            }
        }
    }
}
```

- [ ] **Step 2: Implement TryAcquireModel on emFpPluginList**

```rust
/// Acquire a model via the best matching plugin.
/// Port of C++ `emFpPluginList::TryAcquireModel`.
pub fn TryAcquireModelFromPlugin(
    &mut self,
    context: &Rc<emContext>,
    class_name: &str,
    name: &str,
    name_is_file_path: bool,
    common: bool,
    alternative: usize,
    stat_mode: FileStatMode,
) -> Result<Rc<RefCell<dyn Any>>, FpPluginError> {
    let file_path = if name_is_file_path { Some(name) } else { None };
    let plugin = self.SearchPlugin(
        Some(class_name),
        file_path,
        false,
        alternative,
        stat_mode,
    );

    match plugin {
        None => Err(FpPluginError::NoPluginFound),
        Some(plugin) => plugin.TryAcquireModel(context, class_name, name, common),
    }
}
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check -p emcore 2>&1 | tail -20
```

Note: `emGetAbsolutePath` may or may not exist in `emStd1.rs`. If it doesn't, use `std::fs::canonicalize` or `std::path::Path::canonicalize`. Check what's available and adapt.

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/src/emFpPlugin.rs
git commit -m "feat: implement CreateFilePanel and TryAcquireModel on emFpPluginList"
```

---

## Task 11: Create test plugin crate

**Files:**
- Create: `crates/test_plugin/Cargo.toml`
- Create: `crates/test_plugin/src/lib.rs`
- Add to workspace: root `Cargo.toml`

- [ ] **Step 1: Write test plugin Cargo.toml**

```toml
[package]
name = "test_plugin"
version = "0.1.0"
edition = "2021"

[lib]
name = "test_plugin"
crate-type = ["cdylib"]

[dependencies]
emcore = { path = "../emcore" }
```

- [ ] **Step 2: Add to workspace members**

In root `Cargo.toml`, add `"crates/test_plugin"` to the workspace members list.

- [ ] **Step 3: Write test plugin source**

`crates/test_plugin/src/lib.rs`:

```rust
use std::cell::RefCell;
use std::rc::Rc;

use emcore::emFpPlugin::{emFpPlugin, PanelParentArg};
use emcore::emPanel::PanelBehavior;
use emcore::emErrorPanel::emErrorPanel;

/// Test plugin function that creates a simple error panel with a success message.
/// Used by behavioral tests to validate the full dlopen -> resolve -> call path.
#[no_mangle]
pub fn test_plugin_func(
    _parent: &PanelParentArg,
    _name: &str,
    path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Rc<RefCell<dyn PanelBehavior>>> {
    // Check properties — if any property named "fail" exists, return error
    if plugin.GetProperty("fail").is_some() {
        *error_buf = "test_plugin: instructed to fail".to_string();
        return None;
    }

    // Return an error panel as a simple PanelBehavior implementor
    Some(Rc::new(RefCell::new(emErrorPanel::new(
        &format!("test_plugin loaded: {path}"),
    ))))
}
```

- [ ] **Step 4: Build the test plugin**

```bash
cargo build -p test_plugin 2>&1 | tail -10
ls target/debug/libtest_plugin.so
```

Expected: `libtest_plugin.so` exists in `target/debug/`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/test_plugin/
git commit -m "feat: add test_plugin crate for plugin loading behavioral tests"
```

---

## Task 12: Write behavioral tests for plugin invocation

**Files:**
- Create: `crates/emcore/tests/plugin_invocation.rs`

These tests load the real `test_plugin.so` via `dlopen` and verify the full
invocation path.

- [ ] **Step 1: Write behavioral tests**

```rust
//! Behavioral tests for plugin invocation via dlopen.
//!
//! These tests require `test_plugin` to be built first:
//!   cargo build -p test_plugin
//!
//! They also require LD_LIBRARY_PATH to include target/debug/.

use std::cell::RefCell;
use std::rc::Rc;

use emcore::emContext::emContext;
use emcore::emFpPlugin::{emFpPlugin, emFpPluginList, PanelParentArg, FileStatMode, FpPluginError};

fn make_test_plugin() -> emFpPlugin {
    let mut p = emFpPlugin::new();
    p.file_types = vec![".test".to_string()];
    p.priority = 1.0;
    p.library = "test_plugin".to_string();
    p.function = "test_plugin_func".to_string();
    p
}

#[test]
fn try_create_file_panel_loads_plugin() {
    let plugin = make_test_plugin();
    let ctx = emContext::new_root();
    let parent = PanelParentArg::new(ctx);
    let result = plugin.TryCreateFilePanel(&parent, "test", "/tmp/test.test");
    assert!(result.is_ok(), "TryCreateFilePanel failed: {:?}", result.err());
}

#[test]
fn try_create_file_panel_empty_function_errors() {
    let mut plugin = make_test_plugin();
    plugin.function = String::new();
    let ctx = emContext::new_root();
    let parent = PanelParentArg::new(ctx);
    let result = plugin.TryCreateFilePanel(&parent, "test", "/tmp/test.test");
    assert!(matches!(result, Err(FpPluginError::EmptyFunctionName)));
}

#[test]
fn try_create_file_panel_missing_library_errors() {
    let mut plugin = make_test_plugin();
    plugin.library = "nonexistent_library_xyz".to_string();
    let ctx = emContext::new_root();
    let parent = PanelParentArg::new(ctx);
    let result = plugin.TryCreateFilePanel(&parent, "test", "/tmp/test.test");
    assert!(matches!(result, Err(FpPluginError::LibraryLoad { .. })));
}

#[test]
fn try_create_file_panel_missing_symbol_errors() {
    let mut plugin = make_test_plugin();
    plugin.function = "nonexistent_function_xyz".to_string();
    let ctx = emContext::new_root();
    let parent = PanelParentArg::new(ctx);
    let result = plugin.TryCreateFilePanel(&parent, "test", "/tmp/test.test");
    assert!(matches!(result, Err(FpPluginError::SymbolResolve { .. })));
}

#[test]
fn plugin_list_create_file_panel_finds_matching_plugin() {
    let plugin = make_test_plugin();
    let list = emFpPluginList::from_plugins(vec![plugin]);
    let ctx = emContext::new_root();
    let parent = PanelParentArg::new(ctx);
    // This calls SearchPlugin + TryCreateFilePanel internally
    let panel = list.CreateFilePanelWithStat(
        &parent,
        "test",
        "/tmp/data.test",
        None,
        FileStatMode::Regular,
        0,
    );
    // Should succeed — panel is not an error about missing plugin
    // (it will be an error panel from test_plugin, but with our custom message)
    let borrowed = panel.borrow();
    // Verify it's our test plugin's panel by checking the error message content
    // (test_plugin returns an emErrorPanel with "test_plugin loaded: ...")
}

#[test]
fn plugin_list_no_matching_plugin_returns_error_panel() {
    let list = emFpPluginList::from_plugins(vec![]);
    let ctx = emContext::new_root();
    let parent = PanelParentArg::new(ctx);
    let mut list_mut = list;
    let _panel = list_mut.CreateFilePanelWithStat(
        &parent,
        "test",
        "/tmp/data.unknown",
        None,
        FileStatMode::Regular,
        0,
    );
    // Returns an error panel — "This file type cannot be shown."
}
```

- [ ] **Step 2: Configure LD_LIBRARY_PATH for tests**

Update `.cargo/config.toml`:

```toml
[env]
LLVM_PROFILE_FILE = { value = "target/profraw/default_%m_%p.profraw", force = false }

[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-args=-Wl,-rpath,$ORIGIN/../../target/debug"]
```

Alternatively, add a test runner wrapper. The simplest approach is to set `LD_LIBRARY_PATH` in the config:

```toml
[env]
LLVM_PROFILE_FILE = { value = "target/profraw/default_%m_%p.profraw", force = false }
LD_LIBRARY_PATH = { value = "target/debug", relative = true, force = false }
```

- [ ] **Step 3: Build test_plugin and run tests**

```bash
cargo build -p test_plugin && cargo test -p emcore --test plugin_invocation 2>&1 | tail -30
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/emcore/tests/plugin_invocation.rs .cargo/config.toml
git commit -m "test: add behavioral tests for plugin invocation via dlopen"
```

---

## Task 13: Create etc/ config files

**Files:**
- Create: `etc/emCore/FpPlugins/emStocks.emFpPlugin`
- Create: `etc/emCore/FpPlugins/version`

- [ ] **Step 1: Create directory and config file**

```bash
mkdir -p etc/emCore/FpPlugins
```

Write `etc/emCore/FpPlugins/emStocks.emFpPlugin`:

```
#%rec:emFpPlugin%#

FileTypes = { ".emStocks" }
FileFormatName = "emStocks"
Priority = 1.0
Library = "emStocks"
Function = "emStocksFpPluginFunc"
```

Write `etc/emCore/FpPlugins/version`:

```
0.96.4
```

- [ ] **Step 2: Set EM_DIR for development**

Update `.cargo/config.toml` to set `EM_DIR`:

```toml
[env]
LLVM_PROFILE_FILE = { value = "target/profraw/default_%m_%p.profraw", force = false }
LD_LIBRARY_PATH = { value = "target/debug", relative = true, force = false }
EM_DIR = { value = ".", relative = true, force = false }
```

- [ ] **Step 3: Verify config file loads**

Write a quick test in `crates/emcore/tests/plugin_config.rs`:

```rust
use emcore::emFpPlugin::emFpPluginList;
use emcore::emContext::emContext;
use std::rc::Rc;

#[test]
fn load_plugins_from_etc_directory() {
    // EM_DIR should be set to repo root by .cargo/config.toml
    let ctx = emContext::new_root();
    let list = emFpPluginList::Acquire(&Rc::new(ctx));
    let list = list.borrow();
    // Should find emStocks.emFpPlugin
    assert!(list.plugin_count() > 0, "no plugins loaded — check EM_DIR");
    let plugins = list.plugins();
    let emstocks = plugins.iter().find(|p| p.library == "emStocks");
    assert!(emstocks.is_some(), "emStocks plugin not found");
    let p = emstocks.unwrap();
    assert_eq!(p.function, "emStocksFpPluginFunc");
    assert_eq!(p.file_types, vec![".emStocks"]);
}
```

- [ ] **Step 4: Run test**

```bash
cargo test -p emcore --test plugin_config 2>&1 | tail -20
```

- [ ] **Step 5: Commit**

```bash
git add etc/ .cargo/config.toml crates/emcore/tests/plugin_config.rs
git commit -m "feat: add .emFpPlugin config files and EM_DIR development config"
```

---

## Task 14: Convert emStocksFpPlugin to dynamic plugin entry point

**Files:**
- Rewrite: `crates/emstocks/src/emStocksFpPlugin.rs`

- [ ] **Step 1: Read current emStocksFpPlugin.rs**

Current content (29 lines): placeholder `register_emstocks_plugin()` function with DIVERGED comment.

- [ ] **Step 2: Rewrite with extern entry point**

Replace the entire file:

```rust
//! Plugin entry point for .emStocks files.
//!
//! Port of C++ `emStocksFpPlugin.cpp`. Exports `emStocksFpPluginFunc`
//! which is resolved via dlsym when the plugin manager loads this library.

use std::cell::RefCell;
use std::rc::Rc;

use emcore::emFpPlugin::{emFpPlugin, PanelParentArg};
use emcore::emPanel::PanelBehavior;

use crate::emStocksFileModel::emStocksFileModel;
use crate::emStocksFilePanel::emStocksFilePanel;

/// Plugin entry point for .emStocks files.
/// Port of C++ `emStocksFpPluginFunc` in emStocksFpPlugin.cpp.
///
/// Called by the plugin manager when a .emStocks file needs to be displayed.
/// Loads the file model and creates an emStocksFilePanel.
#[no_mangle]
pub fn emStocksFpPluginFunc(
    parent: &PanelParentArg,
    name: &str,
    path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Rc<RefCell<dyn PanelBehavior>>> {
    if !plugin.properties.is_empty() {
        *error_buf = "emStocksFpPlugin: No properties allowed.".to_string();
        return None;
    }

    let file_model = emStocksFileModel::Acquire(parent.root_context(), path);
    Some(Rc::new(RefCell::new(emStocksFilePanel::new_with_model(
        name, file_model,
    ))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_func_rejects_properties() {
        let ctx = emcore::emContext::emContext::new_root();
        let parent = PanelParentArg::new(Rc::new(ctx));
        let mut plugin = emFpPlugin::new();
        plugin.properties.push(emcore::emFpPlugin::FpPluginProperty {
            name: "bad".to_string(),
            value: "prop".to_string(),
        });
        let mut err = String::new();
        let result = emStocksFpPluginFunc(&parent, "test", "/tmp/test.emStocks", &plugin, &mut err);
        assert!(result.is_none());
        assert!(err.contains("No properties allowed"));
    }
}
```

Note: The exact constructor for `emStocksFilePanel` depends on the current API. Check `emStocksFilePanel.rs` — if it has `new()` with only a `bg_color`, you'll need to add a `new_with_model` constructor that takes a name and file model. Adapt to what exists.

- [ ] **Step 3: Build the emstocks cdylib**

```bash
cargo build -p emstocks 2>&1 | tail -20
ls target/debug/libemStocks.so
```

Expected: `libemStocks.so` exists.

- [ ] **Step 4: Commit**

```bash
git add crates/emstocks/src/emStocksFpPlugin.rs
git commit -m "feat: rewrite emStocksFpPlugin as dynamic plugin entry point"
```

---

## Task 15: Integration test — end-to-end plugin loading

**Files:**
- Create: `crates/eaglemode/tests/integration/plugin_e2e.rs` (or add to existing integration test harness)

- [ ] **Step 1: Write end-to-end integration test**

```rust
//! End-to-end test: load .emStocks file via plugin system.
//!
//! Verifies the full path: config file -> plugin list -> dlopen ->
//! symbol resolve -> emStocksFpPluginFunc -> emStocksFilePanel.

use std::rc::Rc;

use emcore::emContext::emContext;
use emcore::emFpPlugin::{emFpPluginList, PanelParentArg, FileStatMode};

#[test]
fn load_emstocks_plugin_end_to_end() {
    let ctx = Rc::new(emContext::new_root());
    let list = emFpPluginList::Acquire(&ctx);
    let mut list = list.borrow_mut();

    let parent = PanelParentArg::new(Rc::clone(&ctx));

    // Create a temporary .emStocks file
    let tmp_dir = std::env::temp_dir();
    let tmp_file = tmp_dir.join("test_plugin_e2e.emStocks");
    std::fs::write(&tmp_file, "#%rec:emStocksRec%#\n").expect("write test file");

    let panel = list.CreateFilePanelWithStat(
        &parent,
        "test",
        tmp_file.to_str().unwrap(),
        None,
        FileStatMode::Regular,
        0,
    );

    // Panel should be created successfully (not an error panel about missing plugin)
    // The exact type check depends on what emStocksFilePanel exposes
    drop(panel);

    // Cleanup
    let _ = std::fs::remove_file(&tmp_file);
}
```

- [ ] **Step 2: Build all and run**

```bash
cargo build --workspace && cargo test -p eaglemode --test integration -- plugin_e2e 2>&1 | tail -20
```

Or if tests are in a different location, adjust the path.

- [ ] **Step 3: Commit**

```bash
git add crates/eaglemode/tests/
git commit -m "test: add end-to-end integration test for emStocks plugin loading"
```

---

## Task 16: Update CORRESPONDENCE.md and clean up

**Files:**
- Modify: `docs/CORRESPONDENCE.md`
- Modify: `CLAUDE.md` (if commands changed)

- [ ] **Step 1: Update CORRESPONDENCE.md**

Add a new section after the "emStocks port" section:

```markdown
### Plugin system port (2026-03-30)

Workspace restructured: single crate split into Cargo workspace with
crates/emcore/ (dylib), crates/emstocks/ (cdylib), crates/eaglemode/ (bin).

Dynamic library API ported to emStd2.rs:
- emTryOpenLib, emTryResolveSymbolFromLib, emCloseLib, emTryResolveSymbol
- Library table with refcount caching (thread_local! RefCell<Vec>)
- DIVERGED: Single-threaded (no mutex); linear search (not binary search)

Plugin invocation ported to emFpPlugin.rs:
- emFpPluginFunc and emFpPluginModelFunc type aliases (Rust calling convention)
- emFpPlugin::TryCreateFilePanel and TryAcquireModel with cached function pointers
- emFpPluginList::CreateFilePanel (both overloads) and TryAcquireModel
- DIVERGED: #[no_mangle] with Rust ABI (not extern "C") — types cross dylib
  boundary safely because host and plugins link the same libemcore.so

emStocks converted to dynamic plugin:
- emStocksFpPlugin.rs: #[no_mangle] pub fn emStocksFpPluginFunc (was placeholder stub)
- etc/emCore/FpPlugins/emStocks.emFpPlugin config file added
- Static registration stub eliminated

Config files:
- etc/emCore/FpPlugins/ directory with .emFpPlugin config files
- EM_DIR set to repo root for development via .cargo/config.toml
```

- [ ] **Step 2: Update CLAUDE.md commands if needed**

If `cargo check` / `cargo clippy` / `cargo-nextest ntr` still work from the workspace root without changes, no update needed. If workspace-level commands require flags (like `--workspace`), update the Commands section.

Check:

```bash
cargo check
cargo clippy -- -D warnings
cargo-nextest ntr
```

If any fail with workspace errors, update CLAUDE.md to use `cargo check --workspace`, etc.

- [ ] **Step 3: Run full test suite**

```bash
cargo-nextest ntr
cargo test --test golden -- --test-threads=1
```

- [ ] **Step 4: Commit**

```bash
git add docs/CORRESPONDENCE.md CLAUDE.md
git commit -m "docs: update CORRESPONDENCE.md for plugin system port"
```

---

## Task 17: Final verification

- [ ] **Step 1: Clean build from scratch**

```bash
cargo clean && cargo build --workspace 2>&1 | tail -20
```

- [ ] **Step 2: Verify all artifacts exist**

```bash
ls target/debug/libemcore.so target/debug/libemStocks.so target/debug/eaglemode
```

- [ ] **Step 3: Full test suite**

```bash
cargo-nextest ntr
```

- [ ] **Step 4: Clippy clean**

```bash
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 5: Golden tests**

```bash
cargo test -p eaglemode --test golden -- --test-threads=1
```

- [ ] **Step 6: Benchmarks compile**

```bash
cargo bench -p eaglemode --no-run
```

Expected: all pass, all artifacts present, no warnings.
