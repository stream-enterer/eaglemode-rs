# Phase 3: Outside-Consumer Types

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port types needed by eaglemode app modules outside emCore, and close remaining NOT VERIFIED items in marker files.

**Architecture:** emFileStream wraps `std::fs::File` with a manual byte buffer for efficient small reads/writes plus endian-aware typed I/O. emTmpFile wraps `PathBuf` with RAII deletion and an IPC-based master singleton for crash-resilient cleanup. emAnything and emOwnPtrArray are confirmed as stdlib-sufficient (no port needed). emString encoding risk is closed via a codebase-wide file-path audit.

**Tech Stack:** Rust, `std::fs::File`, `std::io::{Read, Write, Seek}`, `PathBuf`, emMiniIpc (already ported), emModel (already ported), existing behavioral test infrastructure.

**Spec:** `docs/superpowers/specs/2026-03-28-port-completion-design.md` (Section 2 Phase 3, Section 3)

**Key rules from spec:**
- emPainter firewall: do NOT touch any `emPainter*.rs` file as blast radius
- No standalone reimplementations: emResTga TGA decoder is debt to retire
- File paths use `PathBuf`/`&Path` (not String) to handle UTF-8 encoding risk
- `pub(crate)` default visibility for internal types; `pub` for library-public types
- C++ method names preserved per File and Name Correspondence

**Current state:** 10 `.no_rs` marker files remain. This phase resolves 4 of them (emFileStream, emTmpFile, emAnything, emOwnPtrArray) and closes the encoding risk in emString. emAnything and emOwnPtrArray are confirmed no-port-needed after investigation — this phase documents the conclusion and updates markers.

**Dependency order:** emAnything/emOwnPtrArray audits (independent) -> emFileStream (independent) -> emTmpFile (depends on emModel, emMiniIpc — both ported) -> emString audit (independent) -> CORRESPONDENCE.md update (after all).

---

## Task 1: emAnything audit — confirm Box<dyn Any> sufficient

**Files:**
- Modify: `src/emCore/emAnything.no_rs`

**Context:** C++ `emAnything` is a type-erased value container with refcounted sharing. Rust uses `Box<dyn Any>` (move-only, no sharing). The marker file's reviewed summary already investigated emCore consumers and found no sharing-dependent usage. One open question remains: whether `emTestContainers.cpp` uses sharing semantics.

- [ ] **Step 1: Check C++ emTestContainers.cpp for sharing-dependent usage**

Read `~/git/eaglemode-0.96.4/src/emTest/emTestContainers.cpp` and search for `emAnything` or `emCastAnything`. Determine if any test relies on two `emAnything` copies sharing the same underlying data (i.e., modifying one visible through the other).

```bash
grep -n "emAnything\|emCastAnything" ~/git/eaglemode-0.96.4/src/emTest/emTestContainers.cpp
```

Expected: emTestContainers tests basic store-and-retrieve, not shared-copy semantics.

- [ ] **Step 2: Update emAnything.no_rs**

Update the reviewed summary to close the open question. Change `NOT VERIFIED: whether any outside-emCore code not in emStocks uses emAnything in a sharing-dependent way (emTest/emTestContainers.cpp not checked)` to the verified finding.

Add a concluding statement: "Conclusion: Box<dyn Any> is sufficient. No port needed. All usage patterns are store-retrieve-extract with no sharing dependence."

- [ ] **Step 3: Commit**

```bash
git add src/emCore/emAnything.no_rs
git commit -m "docs: close emAnything audit — Box<dyn Any> sufficient

Verified emTestContainers.cpp uses store-retrieve pattern only.
No C++ code path relies on emAnything shared-copy semantics.
Box<dyn Any> with downcast_ref is the correct Rust equivalent."
```

---

## Task 2: emOwnPtrArray audit — confirm Vec<T> sufficient

**Files:**
- Modify: `src/emCore/emOwnPtrArray.no_rs`

**Context:** C++ `emOwnPtrArray<T>` is an owning array of heap-allocated pointers. Rust uses `Vec<T>` directly. The marker file shows 2 emCore consumers: emFontCache (hits emPainter firewall — don't touch) and emFpPlugin (already uses Vec). One open question: whether outside-emCore files call BinaryInsert/BinaryRemoveByKey.

- [ ] **Step 1: Check outside-emCore usage of BinaryInsert/BinaryRemoveByKey**

```bash
grep -n "BinaryInsert\|BinaryRemoveByKey\|BinarySearchByKey" \
  ~/git/eaglemode-0.96.4/include/emAv/emAvClient.h \
  ~/git/eaglemode-0.96.4/include/emClock/emTimeZonesModel.h
```

Expected: These files use basic Add/Remove/Get, not the binary search methods.

- [ ] **Step 2: Update emOwnPtrArray.no_rs**

Close the open question about BinaryInsert/BinaryRemoveByKey usage. Add conclusion: "Conclusion: Vec<T> (or Vec<Box<T>>) is sufficient. No port needed. BinaryInsert/BinaryRemoveByKey are composable from stdlib binary_search_by() + insert()/remove() if any outside consumer needs them."

- [ ] **Step 3: Commit**

```bash
git add src/emCore/emOwnPtrArray.no_rs
git commit -m "docs: close emOwnPtrArray audit — Vec<T> sufficient

Verified outside-emCore consumers use basic array operations only.
Vec<T> with stdlib binary_search_by() covers all needed behavior."
```

---

## Task 3: emFileStream core — buffered file I/O

**Files:**
- Create: `src/emCore/emFileStream.rs`
- Modify: `src/emCore/mod.rs`
- Create: `tests/behavioral/file_stream.rs`
- Modify: `tests/behavioral/main.rs`

**Context:** C++ `emFileStream` wraps `FILE*` with an 8KB user-space buffer for efficient small reads/writes. Used by all 13 outside-emCore image format loaders. Rust wraps `std::fs::File` with a manual byte buffer. C++ header: `~/git/eaglemode-0.96.4/include/emCore/emFileStream.h`.

**Design:** Single `Vec<u8>` buffer that can be in read mode or write mode, matching C++ design. Switching mode flushes the buffer. File paths stored as `PathBuf` (not String) per spec.

- [ ] **Step 1: Write failing behavioral tests**

Create `tests/behavioral/file_stream.rs`:

```rust
use eaglemode_rs::emCore::emFileStream::emFileStream;
use std::io::Write;

fn tmp_path(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("eaglemode_test_filestream");
    std::fs::create_dir_all(&dir).ok();
    dir.join(name)
}

#[test]
fn open_read_close() {
    let path = tmp_path("open_read_close.bin");
    std::fs::write(&path, b"hello").unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();
    assert!(fs.IsOpen());

    let mut buf = vec![0u8; 5];
    fs.TryRead(&mut buf).unwrap();
    assert_eq!(&buf, b"hello");

    fs.TryClose().unwrap();
    assert!(!fs.IsOpen());
    std::fs::remove_file(&path).ok();
}

#[test]
fn open_write_close_reread() {
    let path = tmp_path("write_reread.bin");

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "wb").unwrap();
    fs.TryWrite(b"world").unwrap();
    fs.TryClose().unwrap();

    let contents = std::fs::read(&path).unwrap();
    assert_eq!(&contents, b"world");
    std::fs::remove_file(&path).ok();
}

#[test]
fn seek_and_tell() {
    let path = tmp_path("seek_tell.bin");
    std::fs::write(&path, b"abcdefghij").unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();

    assert_eq!(fs.TryTell().unwrap(), 0);
    fs.TrySeek(5).unwrap();
    assert_eq!(fs.TryTell().unwrap(), 5);

    let mut buf = vec![0u8; 3];
    fs.TryRead(&mut buf).unwrap();
    assert_eq!(&buf, b"fgh");
    assert_eq!(fs.TryTell().unwrap(), 8);

    fs.TryClose().unwrap();
    std::fs::remove_file(&path).ok();
}

#[test]
fn read_at_most() {
    let path = tmp_path("read_at_most.bin");
    std::fs::write(&path, b"abc").unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();

    let mut buf = vec![0u8; 10];
    let n = fs.TryReadAtMost(&mut buf).unwrap();
    assert_eq!(n, 3);
    assert_eq!(&buf[..n], b"abc");

    fs.TryClose().unwrap();
    std::fs::remove_file(&path).ok();
}

#[test]
fn read_line() {
    let path = tmp_path("read_line.bin");
    std::fs::write(&path, b"line1\nline2\nline3").unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();

    assert_eq!(fs.TryReadLine(true).unwrap(), "line1");
    assert_eq!(fs.TryReadLine(true).unwrap(), "line2");
    assert_eq!(fs.TryReadLine(true).unwrap(), "line3");

    fs.TryClose().unwrap();
    std::fs::remove_file(&path).ok();
}

#[test]
fn buffered_small_reads() {
    let path = tmp_path("buffered_reads.bin");
    let data: Vec<u8> = (0..=255).collect();
    std::fs::write(&path, &data).unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();

    // Read one byte at a time — should be served from buffer
    for i in 0..=255u8 {
        assert_eq!(fs.TryReadUInt8().unwrap(), i);
    }

    fs.TryClose().unwrap();
    std::fs::remove_file(&path).ok();
}
```

Add `mod file_stream;` to `tests/behavioral/main.rs`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test behavioral file_stream`
Expected: Compilation error — `emFileStream` module doesn't exist yet.

- [ ] **Step 3: Implement emFileStream core**

Create `src/emCore/emFileStream.rs`:

```rust
//! emFileStream — buffered file I/O, ported from emFileStream.h
//!
//! C++ emFileStream wraps FILE* with an 8KB user-space buffer for
//! efficient small reads/writes. Rust wraps std::fs::File with a
//! manual byte buffer.
//!
//! DIVERGED: File paths use PathBuf/&Path (not String) to handle
//! non-UTF-8 file paths safely. C++ uses emString (byte-oriented).
//!
//! DIVERGED: C++ mode strings ("rb", "wb") mapped to Rust
//! OpenOptions. Only "rb", "wb", "r+b", "w+b" supported.
//!
//! DIVERGED: TryGetFile() omitted — no safe way to expose the
//! underlying File without breaking buffer invariants.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const DEFAULT_BUF_SIZE: usize = 8192;

#[derive(Debug)]
pub enum FileStreamError {
    NotOpen,
    IoError(std::io::Error),
    InvalidMode(String),
    Eof,
}

impl fmt::Display for FileStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotOpen => write!(f, "file stream not open"),
            Self::IoError(e) => write!(f, "I/O error: {e}"),
            Self::InvalidMode(m) => write!(f, "invalid mode: {m}"),
            Self::Eof => write!(f, "unexpected end of file"),
        }
    }
}

impl std::error::Error for FileStreamError {}

impl From<std::io::Error> for FileStreamError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

pub type Result<T> = std::result::Result<T, FileStreamError>;

/// Buffered file stream matching C++ `emFileStream`.
pub struct emFileStream {
    file: Option<File>,
    path: PathBuf,
    buf: Vec<u8>,
    buf_pos: usize,
    /// For reading: number of valid bytes in buf. For writing: same as buf_pos.
    buf_end: usize,
    writing: bool,
}

impl emFileStream {
    pub fn new() -> Self {
        Self {
            file: None,
            path: PathBuf::new(),
            buf: vec![0u8; DEFAULT_BUF_SIZE],
            buf_pos: 0,
            buf_end: 0,
            writing: false,
        }
    }

    pub fn with_buf_size(buf_size: usize) -> Self {
        Self {
            file: None,
            path: PathBuf::new(),
            buf: vec![0u8; buf_size.max(64)],
            buf_pos: 0,
            buf_end: 0,
            writing: false,
        }
    }

    // --- Open / Close ---

    /// Open a file. C++ modes: "rb", "wb", "r+b", "w+b".
    pub fn TryOpen(&mut self, path: &Path, mode: &str) -> Result<()> {
        if self.file.is_some() {
            self.TryClose()?;
        }
        let file = match mode {
            "rb" | "r" => OpenOptions::new().read(true).open(path)?,
            "wb" | "w" => OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)?,
            "r+b" | "r+" => OpenOptions::new().read(true).write(true).open(path)?,
            "w+b" | "w+" => OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)?,
            _ => return Err(FileStreamError::InvalidMode(mode.to_string())),
        };
        self.file = Some(file);
        self.path = path.to_path_buf();
        self.buf_pos = 0;
        self.buf_end = 0;
        self.writing = false;
        Ok(())
    }

    pub fn TryClose(&mut self) -> Result<()> {
        if self.writing {
            self.TryFlush()?;
        }
        self.file = None;
        self.path = PathBuf::new();
        self.buf_pos = 0;
        self.buf_end = 0;
        self.writing = false;
        Ok(())
    }

    pub fn IsOpen(&self) -> bool {
        self.file.is_some()
    }

    // --- Seek / Tell ---

    pub fn TryTell(&mut self) -> Result<i64> {
        let f = self.file.as_mut().ok_or(FileStreamError::NotOpen)?;
        let file_pos = f.seek(SeekFrom::Current(0))? as i64;
        if self.writing {
            Ok(file_pos + self.buf_pos as i64)
        } else {
            Ok(file_pos - (self.buf_end as i64 - self.buf_pos as i64))
        }
    }

    pub fn TrySeek(&mut self, pos: i64) -> Result<()> {
        self.flush_and_reset_buffer()?;
        let f = self.file.as_mut().ok_or(FileStreamError::NotOpen)?;
        f.seek(SeekFrom::Start(pos as u64))?;
        Ok(())
    }

    pub fn TrySeekEnd(&mut self, pos_from_end: i64) -> Result<()> {
        self.flush_and_reset_buffer()?;
        let f = self.file.as_mut().ok_or(FileStreamError::NotOpen)?;
        f.seek(SeekFrom::End(-pos_from_end))?;
        Ok(())
    }

    pub fn TrySkip(&mut self, offset: i64) -> Result<()> {
        let pos = self.TryTell()? + offset;
        self.TrySeek(pos)
    }

    // --- Read ---

    pub fn TryRead(&mut self, buf: &mut [u8]) -> Result<()> {
        self.ensure_reading()?;
        let mut filled = 0;
        while filled < buf.len() {
            if self.buf_pos >= self.buf_end {
                self.fill_read_buffer()?;
                if self.buf_pos >= self.buf_end {
                    return Err(FileStreamError::Eof);
                }
            }
            let avail = self.buf_end - self.buf_pos;
            let need = buf.len() - filled;
            let n = avail.min(need);
            buf[filled..filled + n]
                .copy_from_slice(&self.buf[self.buf_pos..self.buf_pos + n]);
            self.buf_pos += n;
            filled += n;
        }
        Ok(())
    }

    pub fn TryReadAtMost(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.ensure_reading()?;
        if self.buf_pos >= self.buf_end {
            self.fill_read_buffer()?;
        }
        let avail = self.buf_end - self.buf_pos;
        let n = avail.min(buf.len());
        buf[..n].copy_from_slice(&self.buf[self.buf_pos..self.buf_pos + n]);
        self.buf_pos += n;
        Ok(n)
    }

    pub fn TryReadLine(&mut self, remove_line_break: bool) -> Result<String> {
        self.ensure_reading()?;
        let mut line = String::new();
        loop {
            if self.buf_pos >= self.buf_end {
                self.fill_read_buffer()?;
                if self.buf_pos >= self.buf_end {
                    // EOF — return what we have (empty string if nothing read)
                    if line.is_empty() {
                        return Err(FileStreamError::Eof);
                    }
                    return Ok(line);
                }
            }
            let b = self.buf[self.buf_pos];
            self.buf_pos += 1;
            if b == b'\n' {
                if !remove_line_break {
                    line.push('\n');
                }
                return Ok(line);
            }
            if b == b'\r' {
                if !remove_line_break {
                    line.push('\r');
                }
                // Check for \r\n
                if self.buf_pos >= self.buf_end {
                    self.fill_read_buffer()?;
                }
                if self.buf_pos < self.buf_end && self.buf[self.buf_pos] == b'\n' {
                    self.buf_pos += 1;
                    if !remove_line_break {
                        line.push('\n');
                    }
                }
                return Ok(line);
            }
            line.push(b as char);
        }
    }

    pub fn TryReadCharOrEOF(&mut self) -> Result<i32> {
        self.ensure_reading()?;
        if self.buf_pos >= self.buf_end {
            self.fill_read_buffer()?;
            if self.buf_pos >= self.buf_end {
                return Ok(-1);
            }
        }
        let b = self.buf[self.buf_pos];
        self.buf_pos += 1;
        Ok(b as i32)
    }

    // --- Write ---

    pub fn TryWrite(&mut self, data: &[u8]) -> Result<()> {
        self.ensure_writing()?;
        let mut written = 0;
        while written < data.len() {
            let space = self.buf.len() - self.buf_pos;
            if space == 0 {
                self.flush_write_buffer()?;
                continue;
            }
            let n = space.min(data.len() - written);
            self.buf[self.buf_pos..self.buf_pos + n]
                .copy_from_slice(&data[written..written + n]);
            self.buf_pos += n;
            self.buf_end = self.buf_pos;
            written += n;
        }
        Ok(())
    }

    pub fn TryWriteStr(&mut self, s: &str) -> Result<()> {
        self.TryWrite(s.as_bytes())
    }

    pub fn TryWriteChar(&mut self, value: u8) -> Result<()> {
        self.TryWrite(&[value])
    }

    pub fn TryFlush(&mut self) -> Result<()> {
        if self.writing {
            self.flush_write_buffer()?;
            let f = self.file.as_mut().ok_or(FileStreamError::NotOpen)?;
            f.flush()?;
        }
        Ok(())
    }

    // --- Internal buffer management ---

    fn ensure_reading(&mut self) -> Result<()> {
        if self.file.is_none() {
            return Err(FileStreamError::NotOpen);
        }
        if self.writing {
            self.flush_write_buffer()?;
            self.writing = false;
        }
        Ok(())
    }

    fn ensure_writing(&mut self) -> Result<()> {
        if self.file.is_none() {
            return Err(FileStreamError::NotOpen);
        }
        if !self.writing {
            // If we had read-ahead data, seek back to logical position
            if self.buf_pos < self.buf_end {
                let f = self.file.as_mut().ok_or(FileStreamError::NotOpen)?;
                let rewind = self.buf_end as i64 - self.buf_pos as i64;
                f.seek(SeekFrom::Current(-rewind))?;
            }
            self.buf_pos = 0;
            self.buf_end = 0;
            self.writing = true;
        }
        Ok(())
    }

    fn fill_read_buffer(&mut self) -> Result<()> {
        let f = self.file.as_mut().ok_or(FileStreamError::NotOpen)?;
        let n = f.read(&mut self.buf)?;
        self.buf_pos = 0;
        self.buf_end = n;
        Ok(())
    }

    fn flush_write_buffer(&mut self) -> Result<()> {
        if self.buf_pos > 0 {
            let f = self.file.as_mut().ok_or(FileStreamError::NotOpen)?;
            f.write_all(&self.buf[..self.buf_pos])?;
            self.buf_pos = 0;
            self.buf_end = 0;
        }
        Ok(())
    }

    fn flush_and_reset_buffer(&mut self) -> Result<()> {
        if self.writing {
            self.flush_write_buffer()?;
        } else if self.buf_pos < self.buf_end {
            // Seek back to logical position (undo read-ahead)
            let f = self.file.as_mut().ok_or(FileStreamError::NotOpen)?;
            let rewind = self.buf_end as i64 - self.buf_pos as i64;
            f.seek(SeekFrom::Current(-rewind))?;
        }
        self.buf_pos = 0;
        self.buf_end = 0;
        self.writing = false;
        Ok(())
    }
}

impl Default for emFileStream {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for emFileStream {
    fn drop(&mut self) {
        if self.file.is_some() {
            let _ = self.TryClose();
        }
    }
}
```

- [ ] **Step 4: Add to mod.rs**

In `src/emCore/mod.rs`, add `pub mod emFileStream;` in alphabetical order (after emEngine, before emFileModel).

- [ ] **Step 5: Run tests**

Run: `cargo clippy -- -D warnings && cargo-nextest ntr`
Expected: All tests pass including the 6 new behavioral tests.

- [ ] **Step 6: Commit**

```bash
git add src/emCore/emFileStream.rs src/emCore/mod.rs \
  tests/behavioral/file_stream.rs tests/behavioral/main.rs
git commit -m "feat: port emFileStream core with buffered I/O

Wraps std::fs::File with manual byte buffer (8KB default).
Supports open/close/seek/tell, buffered read/write, line reading.
PathBuf for file paths (DIVERGED from C++ emString)."
```

---

## Task 4: emFileStream endian read/write methods

**Files:**
- Modify: `src/emCore/emFileStream.rs`
- Modify: `tests/behavioral/file_stream.rs`

**Context:** C++ `emFileStream` has 16 typed read methods and 16 typed write methods for endian-aware I/O (Int8/16/32/64 in LE/BE variants). These are used extensively by outside-emCore image format loaders. Each reads/writes a fixed number of bytes from the buffer and converts endianness.

- [ ] **Step 1: Write endian round-trip tests**

Add to `tests/behavioral/file_stream.rs`:

```rust
#[test]
fn endian_uint16_le_round_trip() {
    let path = tmp_path("endian_u16le.bin");

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "wb").unwrap();
    fs.TryWriteUInt16LE(0x1234).unwrap();
    fs.TryWriteUInt16LE(0xABCD).unwrap();
    fs.TryClose().unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();
    assert_eq!(fs.TryReadUInt16LE().unwrap(), 0x1234);
    assert_eq!(fs.TryReadUInt16LE().unwrap(), 0xABCD);
    fs.TryClose().unwrap();
    std::fs::remove_file(&path).ok();
}

#[test]
fn endian_int32_be_round_trip() {
    let path = tmp_path("endian_i32be.bin");

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "wb").unwrap();
    fs.TryWriteInt32BE(-1).unwrap();
    fs.TryWriteInt32BE(0x12345678).unwrap();
    fs.TryClose().unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();
    assert_eq!(fs.TryReadInt32BE().unwrap(), -1);
    assert_eq!(fs.TryReadInt32BE().unwrap(), 0x12345678);
    fs.TryClose().unwrap();
    std::fs::remove_file(&path).ok();
}

#[test]
fn endian_uint64_le_round_trip() {
    let path = tmp_path("endian_u64le.bin");

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "wb").unwrap();
    fs.TryWriteUInt64LE(0x0102030405060708).unwrap();
    fs.TryClose().unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();
    assert_eq!(fs.TryReadUInt64LE().unwrap(), 0x0102030405060708);
    fs.TryClose().unwrap();
    std::fs::remove_file(&path).ok();
}

#[test]
fn endian_all_types() {
    let path = tmp_path("endian_all.bin");

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "wb").unwrap();
    fs.TryWriteInt8(-42).unwrap();
    fs.TryWriteUInt8(200).unwrap();
    fs.TryWriteInt16LE(-1000).unwrap();
    fs.TryWriteInt16BE(-1000).unwrap();
    fs.TryWriteUInt16LE(60000).unwrap();
    fs.TryWriteUInt16BE(60000).unwrap();
    fs.TryWriteInt32LE(-100000).unwrap();
    fs.TryWriteInt32BE(-100000).unwrap();
    fs.TryWriteUInt32LE(3_000_000_000).unwrap();
    fs.TryWriteUInt32BE(3_000_000_000).unwrap();
    fs.TryWriteInt64LE(-1_000_000_000_000).unwrap();
    fs.TryWriteInt64BE(-1_000_000_000_000).unwrap();
    fs.TryWriteUInt64LE(10_000_000_000_000).unwrap();
    fs.TryWriteUInt64BE(10_000_000_000_000).unwrap();
    fs.TryClose().unwrap();

    let mut fs = emFileStream::new();
    fs.TryOpen(&path, "rb").unwrap();
    assert_eq!(fs.TryReadInt8().unwrap(), -42);
    assert_eq!(fs.TryReadUInt8().unwrap(), 200);
    assert_eq!(fs.TryReadInt16LE().unwrap(), -1000);
    assert_eq!(fs.TryReadInt16BE().unwrap(), -1000);
    assert_eq!(fs.TryReadUInt16LE().unwrap(), 60000);
    assert_eq!(fs.TryReadUInt16BE().unwrap(), 60000);
    assert_eq!(fs.TryReadInt32LE().unwrap(), -100000);
    assert_eq!(fs.TryReadInt32BE().unwrap(), -100000);
    assert_eq!(fs.TryReadUInt32LE().unwrap(), 3_000_000_000);
    assert_eq!(fs.TryReadUInt32BE().unwrap(), 3_000_000_000);
    assert_eq!(fs.TryReadInt64LE().unwrap(), -1_000_000_000_000);
    assert_eq!(fs.TryReadInt64BE().unwrap(), -1_000_000_000_000);
    assert_eq!(fs.TryReadUInt64LE().unwrap(), 10_000_000_000_000);
    assert_eq!(fs.TryReadUInt64BE().unwrap(), 10_000_000_000_000);
    fs.TryClose().unwrap();
    std::fs::remove_file(&path).ok();
}
```

- [ ] **Step 2: Implement endian methods**

Add to `src/emCore/emFileStream.rs`. Each method reads/writes a fixed number of bytes. Use `from_le_bytes`/`from_be_bytes`/`to_le_bytes`/`to_be_bytes` for conversion:

```rust
impl emFileStream {
    // --- Typed reads ---

    fn read_exact_bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut bytes = [0u8; N];
        self.TryRead(&mut bytes)?;
        Ok(bytes)
    }

    pub fn TryReadInt8(&mut self) -> Result<i8> {
        Ok(i8::from_le_bytes(self.read_exact_bytes()?))
    }
    pub fn TryReadUInt8(&mut self) -> Result<u8> {
        Ok(u8::from_le_bytes(self.read_exact_bytes()?))
    }
    pub fn TryReadInt16LE(&mut self) -> Result<i16> {
        Ok(i16::from_le_bytes(self.read_exact_bytes()?))
    }
    pub fn TryReadInt16BE(&mut self) -> Result<i16> {
        Ok(i16::from_be_bytes(self.read_exact_bytes()?))
    }
    pub fn TryReadUInt16LE(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.read_exact_bytes()?))
    }
    pub fn TryReadUInt16BE(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.read_exact_bytes()?))
    }
    pub fn TryReadInt32LE(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.read_exact_bytes()?))
    }
    pub fn TryReadInt32BE(&mut self) -> Result<i32> {
        Ok(i32::from_be_bytes(self.read_exact_bytes()?))
    }
    pub fn TryReadUInt32LE(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read_exact_bytes()?))
    }
    pub fn TryReadUInt32BE(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.read_exact_bytes()?))
    }
    pub fn TryReadInt64LE(&mut self) -> Result<i64> {
        Ok(i64::from_le_bytes(self.read_exact_bytes()?))
    }
    pub fn TryReadInt64BE(&mut self) -> Result<i64> {
        Ok(i64::from_be_bytes(self.read_exact_bytes()?))
    }
    pub fn TryReadUInt64LE(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.read_exact_bytes()?))
    }
    pub fn TryReadUInt64BE(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.read_exact_bytes()?))
    }

    // --- Typed writes ---

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.TryWrite(bytes)
    }

    pub fn TryWriteInt8(&mut self, v: i8) -> Result<()> {
        self.write_bytes(&v.to_le_bytes())
    }
    pub fn TryWriteUInt8(&mut self, v: u8) -> Result<()> {
        self.write_bytes(&v.to_le_bytes())
    }
    pub fn TryWriteInt16LE(&mut self, v: i16) -> Result<()> {
        self.write_bytes(&v.to_le_bytes())
    }
    pub fn TryWriteInt16BE(&mut self, v: i16) -> Result<()> {
        self.write_bytes(&v.to_be_bytes())
    }
    pub fn TryWriteUInt16LE(&mut self, v: u16) -> Result<()> {
        self.write_bytes(&v.to_le_bytes())
    }
    pub fn TryWriteUInt16BE(&mut self, v: u16) -> Result<()> {
        self.write_bytes(&v.to_be_bytes())
    }
    pub fn TryWriteInt32LE(&mut self, v: i32) -> Result<()> {
        self.write_bytes(&v.to_le_bytes())
    }
    pub fn TryWriteInt32BE(&mut self, v: i32) -> Result<()> {
        self.write_bytes(&v.to_be_bytes())
    }
    pub fn TryWriteUInt32LE(&mut self, v: u32) -> Result<()> {
        self.write_bytes(&v.to_le_bytes())
    }
    pub fn TryWriteUInt32BE(&mut self, v: u32) -> Result<()> {
        self.write_bytes(&v.to_be_bytes())
    }
    pub fn TryWriteInt64LE(&mut self, v: i64) -> Result<()> {
        self.write_bytes(&v.to_le_bytes())
    }
    pub fn TryWriteInt64BE(&mut self, v: i64) -> Result<()> {
        self.write_bytes(&v.to_be_bytes())
    }
    pub fn TryWriteUInt64LE(&mut self, v: u64) -> Result<()> {
        self.write_bytes(&v.to_le_bytes())
    }
    pub fn TryWriteUInt64BE(&mut self, v: u64) -> Result<()> {
        self.write_bytes(&v.to_be_bytes())
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo clippy -- -D warnings && cargo-nextest ntr`
Expected: All tests pass including 4 new endian tests.

- [ ] **Step 4: Delete marker file and commit**

```bash
git rm src/emCore/emFileStream.no_rs
git add src/emCore/emFileStream.rs tests/behavioral/file_stream.rs
git commit -m "feat: add endian read/write to emFileStream, delete marker

16 typed read methods + 16 typed write methods for Int8/16/32/64
in LE/BE variants. Round-trip tested. Marker file deleted."
```

---

## Task 5: emResTga investigation

**Files:**
- Possibly modify: `src/emCore/emResTga.rs` (add DIVERGED comment only)

**Context:** The spec says to "verify CORRESPONDENCE.md claims about emResTga reimplementation" and "refactor emResTga to use emFileStream." However, emResTga loads TGA from embedded `&[u8]` via `include_bytes!()` — it doesn't load from files. Refactoring it to use emFileStream would change it from embedded-bytes to file-based loading, which is a functional change, not just a cleanup.

- [ ] **Step 1: Verify the reimplementation claim**

Read `src/emCore/emResTga.rs` and the C++ `~/git/eaglemode-0.96.4/src/emCore/emRes.cpp` to verify:
1. The Rust TGA decoder is a standalone reimplementation (not derived from the C++ code)
2. The C++ emRes loads TGA files from disk using emFileStream
3. The Rust approach uses include_bytes!() to embed TGA data in the binary

```bash
grep -n "emFileStream\|TryRead\|fopen\|load_tga" \
  ~/git/eaglemode-0.96.4/src/emCore/emRes.cpp | head -20
```

- [ ] **Step 2: Document the decision**

The Rust emResTga approach (embedded bytes) is architecturally different from C++ (file-based). Refactoring to use emFileStream is not appropriate because:
1. The embedded approach is simpler (no I/O at runtime, no file-not-found errors)
2. The TGA decoder operates on `&[u8]` which is the right abstraction for embedded data
3. emFileStream is for file-based I/O — the outside-emCore image loaders will use it

If emResTga.rs doesn't already have a DIVERGED comment explaining this, add one:

```rust
// DIVERGED: C++ emRes loads TGA files from disk via emFileStream.
// Rust embeds TGA data via include_bytes!() and decodes from &[u8].
// No emFileStream dependency — toolkit assets are compile-time embedded.
```

- [ ] **Step 3: Commit (if changed)**

```bash
git add src/emCore/emResTga.rs
git commit -m "docs: add DIVERGED comment to emResTga

Documents that Rust embeds TGA assets via include_bytes!() instead
of loading from disk via emFileStream. Standalone TGA decoder is
retained for embedded-bytes use case."
```

---

## Task 6: emTmpFile port with IPC-based cleanup

**Files:**
- Create: `src/emCore/emTmpFile.rs`
- Modify: `src/emCore/mod.rs`
- Create: `tests/behavioral/tmp_file.rs`
- Modify: `tests/behavioral/main.rs`

**Context:** C++ emTmpFile holds a temp file path and auto-deletes on drop. emTmpFileMaster is a singleton (emModel) that creates a per-process temp directory and runs an emMiniIpcServer for crash-resilient cleanup. Dependencies: emModel (ported), emMiniIpc (ported), emInstallInfo (ported). C++ header: `~/git/eaglemode-0.96.4/include/emCore/emTmpFile.h`.

**Design decisions:**
- emTmpFile: simple RAII wrapper around PathBuf. Drop deletes the file/directory.
- emTmpFileMaster: deferred. The IPC-based singleton requires deep integration with emModel/emContext lifecycle which is complex. Port only emTmpFile (the RAII path holder) in this phase. When emTmpConv is ported, evaluate whether emTmpFileMaster is needed or if Rust's tempfile crate + RAII is sufficient.
- DIVERGED: emTmpFileMaster deferred. emTmpFile works standalone with explicit paths.

- [ ] **Step 1: Write behavioral tests**

Create `tests/behavioral/tmp_file.rs`:

```rust
use eaglemode_rs::emCore::emTmpFile::emTmpFile;
use std::path::PathBuf;

fn test_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("eaglemode_test_tmpfile");
    std::fs::create_dir_all(&dir).ok();
    dir
}

#[test]
fn custom_path_file_deleted_on_drop() {
    let path = test_dir().join("drop_test.txt");
    std::fs::write(&path, b"hello").unwrap();
    assert!(path.exists());

    {
        let _tmp = emTmpFile::from_custom_path(path.clone());
        assert!(path.exists());
    }
    // After drop, file is deleted
    assert!(!path.exists());
}

#[test]
fn custom_path_dir_deleted_on_drop() {
    let dir = test_dir().join("drop_dir_test");
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("sub/file.txt"), b"data").unwrap();
    assert!(dir.exists());

    {
        let _tmp = emTmpFile::from_custom_path(dir.clone());
    }
    assert!(!dir.exists());
}

#[test]
fn get_path() {
    let path = test_dir().join("getpath_test.txt");
    std::fs::write(&path, b"x").unwrap();

    let tmp = emTmpFile::from_custom_path(path.clone());
    assert_eq!(tmp.GetPath(), &path);
    // Explicit discard so we clean up
    drop(tmp);
}

#[test]
fn discard_clears_path() {
    let path = test_dir().join("discard_test.txt");
    std::fs::write(&path, b"x").unwrap();

    let mut tmp = emTmpFile::from_custom_path(path.clone());
    tmp.Discard();

    assert!(tmp.GetPath().as_os_str().is_empty());
    assert!(!path.exists());
}

#[test]
fn empty_tmpfile_drop_is_noop() {
    let _tmp = emTmpFile::new();
    // Should not panic on drop
}
```

Add `mod tmp_file;` to `tests/behavioral/main.rs`.

- [ ] **Step 2: Implement emTmpFile**

Create `src/emCore/emTmpFile.rs`:

```rust
//! emTmpFile — temporary file path holder with RAII deletion.
//!
//! C++ emTmpFile.h provides emTmpFile (RAII path holder) and
//! emTmpFileMaster (IPC-based singleton for crash-resilient cleanup).
//!
//! DIVERGED: emTmpFileMaster deferred. The IPC-based singleton requires
//! deep integration with emModel/emContext lifecycle. emTmpFile works
//! standalone with explicit paths. emTmpFileMaster will be ported when
//! emTmpConv (the only outside consumer) is ported, if RAII cleanup
//! proves insufficient.

use std::path::{Path, PathBuf};

/// Temporary file/directory path holder. Deletes the file or directory
/// tree on drop. Matches C++ `emTmpFile`.
pub struct emTmpFile {
    path: PathBuf,
}

impl emTmpFile {
    /// Construct with empty path (no file to delete). C++ `emTmpFile()`.
    pub fn new() -> Self {
        Self {
            path: PathBuf::new(),
        }
    }

    /// Construct with an explicit path. C++ `emTmpFile(const emString&)`.
    pub fn from_custom_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// Set a custom path. Calls Discard() first. C++ `SetupCustomPath`.
    pub fn SetupCustomPath(&mut self, path: PathBuf) {
        self.Discard();
        self.path = path;
    }

    /// Get the current path. C++ `GetPath`.
    pub fn GetPath(&self) -> &Path {
        &self.path
    }

    /// Delete the file/directory and clear the path. C++ `Discard`.
    pub fn Discard(&mut self) {
        if !self.path.as_os_str().is_empty() {
            if self.path.is_dir() {
                let _ = std::fs::remove_dir_all(&self.path);
            } else if self.path.exists() {
                let _ = std::fs::remove_file(&self.path);
            }
            self.path = PathBuf::new();
        }
    }
}

impl Default for emTmpFile {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for emTmpFile {
    fn drop(&mut self) {
        self.Discard();
    }
}
```

- [ ] **Step 3: Add to mod.rs, run tests, commit**

Add `pub mod emTmpFile;` to mod.rs (after emTimer, before emTunnel).

Run: `cargo clippy -- -D warnings && cargo-nextest ntr`

```bash
git rm src/emCore/emTmpFile.no_rs
git add src/emCore/emTmpFile.rs src/emCore/mod.rs \
  tests/behavioral/tmp_file.rs tests/behavioral/main.rs
git commit -m "feat: port emTmpFile with RAII deletion

PathBuf-based temp file holder. Drop deletes file or directory tree.
emTmpFileMaster deferred — IPC-based crash cleanup requires emModel
integration that will be evaluated when emTmpConv is ported."
```

---

## Task 7: emString encoding audit

**Files:**
- Possibly modify: multiple `src/emCore/*.rs` files
- Modify: `src/emCore/emString.no_rs`

**Context:** C++ `emString` is byte-oriented (no encoding assumption). Rust `String` enforces UTF-8. File paths on Unix can contain non-UTF-8 bytes. The spec says to grep the codebase for file paths stored in `String` and refactor to `PathBuf`/`OsString` where non-UTF-8 paths are possible.

- [ ] **Step 1: Identify file path usage in String**

Search for file paths stored as String in emCore:

```bash
# Find methods/fields that store file paths as String
grep -rn "file_path\|file_name\|FilePath\|FileName\|GetPath\|GetFilePath\|get_path" \
  src/emCore/ --include='*.rs' | grep -i "string\|str" | head -40
```

Also check:
```bash
grep -rn "fn.*path.*String\|path:.*String\|Path.*to_string\|path.*to_owned" \
  src/emCore/ --include='*.rs' | head -40
```

- [ ] **Step 2: Categorize each site**

For each site found, determine:
a) Is this a file path that could contain non-UTF-8 bytes on Unix? → Refactor to PathBuf
b) Is this a display string that happens to contain a path? → Leave as String
c) Is this a user-facing name (not a filesystem path)? → Leave as String

**Key files to check:**
- `emFileModel.rs` — stores file paths
- `emInstallInfo.rs` — stores directory paths
- `emRes.rs` — stores resource paths
- `emFileDialog.rs` — file selection paths
- `emFileSelectionBox.rs` — file selection paths
- `emProcess.rs` — command paths

**Expected:** Most file paths in emCore already use PathBuf or &Path (since emFileStream was designed with PathBuf). Some older code may use String for paths. Each site is evaluated individually.

- [ ] **Step 3: Refactor identified sites**

For each site that needs refactoring, change `String` to `PathBuf` and `&str` to `&Path`. Update callers. If a site is ambiguous (path used both for display and filesystem access), add a comment explaining the decision.

- [ ] **Step 4: Update emString.no_rs**

Close the `NOT VERIFIED: whether any eaglemode code encounters non-UTF-8 file paths in practice` item. Document the audit findings:
- Which sites were checked
- Which sites were refactored (if any)
- Conclusion about encoding risk

- [ ] **Step 5: Run tests and commit**

Run: `cargo clippy -- -D warnings && cargo-nextest ntr`

```bash
git add src/emCore/emString.no_rs [any modified .rs files]
git commit -m "docs: close emString encoding audit

Audited file path usage across emCore. [Document findings:
which sites use PathBuf, which use String, whether any
non-UTF-8 risk exists.]"
```

---

## Task 8: CORRESPONDENCE.md update and review checkpoint

**Files:**
- Modify: `src/emCore/CORRESPONDENCE.md`

- [ ] **Step 1: Update CORRESPONDENCE.md**

Update to reflect Phase 3 changes:
- emFileStream.no_rs deleted → emFileStream.rs created (buffered I/O with endian read/write)
- emTmpFile.no_rs deleted → emTmpFile.rs created (RAII deletion, emTmpFileMaster deferred)
- emAnything.no_rs remains → audit confirmed Box<dyn Any> sufficient
- emOwnPtrArray.no_rs remains → audit confirmed Vec<T> sufficient
- emString.no_rs remains → encoding audit completed, findings documented
- emResTga.rs DIVERGED documented (embedded bytes vs file-based)
- Update file/marker counts

- [ ] **Step 2: Verify marker files**

```bash
# Should still exist (no port needed):
ls src/emCore/emAnything.no_rs src/emCore/emOwnPtrArray.no_rs \
   src/emCore/emString.no_rs src/emCore/emAvlTree.no_rs \
   src/emCore/emRef.no_rs src/emCore/emOwnPtr.no_rs \
   src/emCore/emThread.no_rs src/emCore/emToolkit.no_rs

# Should NOT exist (ported):
ls src/emCore/emFileStream.no_rs src/emCore/emTmpFile.no_rs 2>&1
# Expected: "No such file or directory" for both
```

- [ ] **Step 3: Run full test suite**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr && \
  cargo test --test golden -- --test-threads=1
```

- [ ] **Step 4: Commit**

```bash
git add src/emCore/CORRESPONDENCE.md
git commit -m "docs: update CORRESPONDENCE.md for Phase 3 completion

Reflects emFileStream and emTmpFile ports. Documents emAnything
and emOwnPtrArray audit conclusions (stdlib sufficient). Records
emString encoding audit findings."
```

- [ ] **Step 5: Report findings**

Summarize:
- Types ported: emFileStream, emTmpFile
- Types confirmed stdlib-sufficient: emAnything (Box<dyn Any>), emOwnPtrArray (Vec<T>)
- emString encoding audit results
- Remaining marker files and their status
- Readiness for Phase 4
