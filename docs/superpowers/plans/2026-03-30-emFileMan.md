# emFileMan Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port C++ emFileMan to Rust as a cdylib+rlib crate providing the file browser that closes the rendering loop: directories display files, files display content via plugins, and the full Eagle Mode browsing experience works end-to-end.

**Architecture:** New `crates/emfileman/` crate (cdylib+rlib) depending on `emcore`. 14 C++ headers map to 15 Rust files (one split) plus 3 FpPlugin entry points. The crate exports 3 `#[no_mangle]` functions discovered at runtime via `.emFpPlugin` config files. All types follow File and Name Correspondence rules.

**Tech Stack:** Rust, libc (stat/lstat/readlink/getpwuid_r/getgrgid_r), emcore (emFileModel, emFilePanel, emConfigModel, emRecFileModel, emFpPlugin, emMiniIpc, Record trait)

---

## File Structure

```
crates/emfileman/
  Cargo.toml
  src/
    lib.rs                     module declarations, re-exports, #[allow(non_snake_case)]
    emDirEntry.rs              filesystem metadata, COW via Rc<SharedData>
    emDirModel.rs              directory loading state machine (FileModelOps)
    emDirPanel.rs              grid of directory entries (emFilePanel)
    emDirEntryPanel.rs         single file/directory display (emPanel)
    emDirEntryAltPanel.rs      alternative content views (emPanel)
    emDirStatPanel.rs          directory statistics (emFilePanel)
    emFileManModel.rs          selections, commands, IPC
    emFileManConfig.rs         global defaults (emConfigModel + Record)
    emFileManTheme.rs          ~100 layout/color params (emConfigModel + Record)
    emFileManThemeNames.rs     theme catalog (SPLIT: from emFileManTheme.h)
    emFileManViewConfig.rs     per-view config bridge, CompareDirEntries
    emFileLinkModel.rs         link file parser (emRecFileModel + Record)
    emFileLinkPanel.rs         link file display (emFilePanel)
    emFileManControlPanel.rs   sort/filter/theme UI (emLinearLayout)
    emFileManSelInfoPanel.rs   selection statistics (emPanel)
    emDirFpPlugin.rs           #[no_mangle] entry point
    emDirStatFpPlugin.rs       #[no_mangle] entry point
    emFileLinkFpPlugin.rs      #[no_mangle] entry point
```

---

## Task 1: Crate Scaffold

**Files:**
- Create: `crates/emfileman/Cargo.toml`
- Create: `crates/emfileman/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "emfileman"
version = "0.1.0"
edition = "2021"

[lib]
name = "emFileMan"
crate-type = ["cdylib", "rlib"]

[dependencies]
emcore = { path = "../emcore" }
libc = { workspace = true }
log = { workspace = true }
```

- [ ] **Step 2: Create lib.rs**

```rust
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

pub mod emDirEntry;
```

Only declare `emDirEntry` for now. Modules will be added as tasks complete.

- [ ] **Step 3: Add to workspace**

In the root `Cargo.toml`, add `"crates/emfileman"` to the `[workspace] members` list.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p emfileman`
Expected: compiles with no errors (emDirEntry module will be empty or have a placeholder).

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/ Cargo.toml Cargo.lock
git commit -m "feat(emfileman): scaffold crate with Cargo.toml and lib.rs"
```

---

## Task 2: emDirEntry — SharedData and Accessors

**Files:**
- Create: `crates/emfileman/src/emDirEntry.rs`
- Test: inline `#[cfg(test)] mod tests`

Port of C++ `emDirEntry` — filesystem entry metadata with COW shared data via `Rc<SharedData>`.

- [ ] **Step 1: Write failing tests for SharedData construction and accessors**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_entry_has_empty_fields() {
        let e = emDirEntry::new();
        assert!(e.GetPath().is_empty());
        assert!(e.GetName().is_empty());
        assert!(e.GetTargetPath().is_empty());
        assert!(e.GetOwner().is_empty());
        assert!(e.GetGroup().is_empty());
        assert!(!e.IsHidden());
        assert!(!e.IsSymbolicLink());
        assert!(!e.IsDirectory());
        assert!(!e.IsRegularFile());
        assert_eq!(e.GetStatErrNo(), 0);
        assert_eq!(e.GetLStatErrNo(), 0);
        assert_eq!(e.GetTargetPathErrNo(), 0);
    }

    #[test]
    fn cow_clone_shares_data() {
        let e1 = emDirEntry::new();
        let e2 = e1.clone();
        // Both point to same shared data (Rc refcount > 1)
        assert_eq!(e1, e2);
    }

    #[test]
    fn load_real_file() {
        let e = emDirEntry::from_path("/dev/null");
        assert_eq!(e.GetPath(), "/dev/null");
        assert_eq!(e.GetName(), "null");
        assert!(e.IsRegularFile() || !e.IsDirectory()); // /dev/null is a char device
        assert_eq!(e.GetStatErrNo(), 0);
        assert!(!e.GetOwner().is_empty());
        assert!(!e.GetGroup().is_empty());
    }

    #[test]
    fn load_parent_and_name() {
        let e = emDirEntry::from_parent_and_name("/dev", "null");
        assert_eq!(e.GetPath(), "/dev/null");
        assert_eq!(e.GetName(), "null");
    }

    #[test]
    fn load_directory() {
        let e = emDirEntry::from_path("/tmp");
        assert!(e.IsDirectory());
        assert!(!e.IsRegularFile());
    }

    #[test]
    fn hidden_file_detection() {
        // Create a temp hidden file
        let dir = std::env::temp_dir();
        let hidden_path = dir.join(".test_hidden_emfileman");
        std::fs::write(&hidden_path, "test").unwrap();
        let e = emDirEntry::from_path(hidden_path.to_str().unwrap());
        assert!(e.IsHidden());
        std::fs::remove_file(&hidden_path).unwrap();
    }

    #[test]
    fn symlink_detection() {
        let dir = std::env::temp_dir();
        let target = dir.join("emfileman_symlink_target");
        let link = dir.join("emfileman_symlink_link");
        let _ = std::fs::remove_file(&link);
        let _ = std::fs::remove_file(&target);
        std::fs::write(&target, "data").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let e = emDirEntry::from_path(link.to_str().unwrap());
        assert!(e.IsSymbolicLink());
        assert!(e.IsRegularFile()); // follows symlink for stat
        assert_eq!(e.GetTargetPathErrNo(), 0);
        assert!(!e.GetTargetPath().is_empty());

        std::fs::remove_file(&link).unwrap();
        std::fs::remove_file(&target).unwrap();
    }

    #[test]
    fn nonexistent_path() {
        let e = emDirEntry::from_path("/nonexistent_emfileman_test_path");
        assert_ne!(e.GetStatErrNo(), 0);
        assert_ne!(e.GetLStatErrNo(), 0);
    }

    #[test]
    fn equality() {
        let e1 = emDirEntry::from_path("/dev/null");
        let e2 = emDirEntry::from_path("/dev/null");
        assert_eq!(e1, e2);

        let e3 = emDirEntry::from_path("/tmp");
        assert_ne!(e1, e3);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo-nextest ntr -p emfileman`
Expected: FAIL — `emDirEntry` type not defined.

- [ ] **Step 3: Implement emDirEntry**

```rust
use std::ffi::CString;
use std::rc::Rc;

#[derive(Clone, Debug)]
struct SharedData {
    path: String,
    name: String,
    target_path: String,
    owner: String,
    group: String,
    hidden: bool,
    stat: libc::stat,
    lstat: Option<libc::stat>,  // Some if symlink
    stat_errno: i32,
    lstat_errno: i32,
    target_path_errno: i32,
}

impl Default for SharedData {
    fn default() -> Self {
        Self {
            path: String::new(),
            name: String::new(),
            target_path: String::new(),
            owner: String::new(),
            group: String::new(),
            hidden: false,
            stat: unsafe { std::mem::zeroed() },
            lstat: None,
            stat_errno: 0,
            lstat_errno: 0,
            target_path_errno: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct emDirEntry {
    data: Rc<SharedData>,
}
```

Implement `new()`, `from_path(path)`, `from_parent_and_name(parent, name)`, and all C++-matching accessors: `GetPath`, `GetName`, `GetTargetPath`, `IsSymbolicLink`, `IsDirectory`, `IsRegularFile`, `IsHidden`, `GetStat`, `GetLStat`, `GetOwner`, `GetGroup`, `GetTargetPathErrNo`, `GetStatErrNo`, `GetLStatErrNo`.

The `Load` logic (called `priv_load` internally):
1. `libc::lstat` the path — if it fails, set `lstat_errno`, try `libc::stat` as fallback
2. If `lstat` succeeds and `S_ISLNK`, store lstat separately, `libc::stat` for Stat, `libc::readlink` for target path
3. `libc::getpwuid_r` for owner name, `libc::getgrgid_r` for group name
4. Hidden = name starts with `.`

Implement `PartialEq` comparing all SharedData fields (path, name, target_path, owner, group, hidden, stat bytes, lstat bytes, errno values).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo-nextest ntr -p emfileman`
Expected: all tests PASS.

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p emfileman -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/emfileman/src/emDirEntry.rs
git commit -m "feat(emDirEntry): COW filesystem metadata with libc stat/lstat/readlink"
```

---

## Task 3: emFileManConfig — Global Defaults

**Files:**
- Create: `crates/emfileman/src/emFileManConfig.rs`
- Modify: `crates/emfileman/src/lib.rs` (add module)

Port of C++ `emFileManConfig` — 6 config fields with Record trait. This is an `emConfigModel<emFileManConfig>`.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use emcore::emRec::{RecStruct, RecValue};
    use emcore::emRecRecord::Record;

    #[test]
    fn default_values() {
        let c = emFileManConfigData::default();
        assert_eq!(c.sort_criterion, SortCriterion::ByName);
        assert_eq!(c.name_sorting_style, NameSortingStyle::PerLocale);
        assert!(!c.sort_directories_first);
        assert!(!c.show_hidden_files);
        assert!(c.theme_name.is_empty());
        assert!(c.autosave);
    }

    #[test]
    fn record_round_trip() {
        let mut c = emFileManConfigData::default();
        c.sort_criterion = SortCriterion::ByDate;
        c.name_sorting_style = NameSortingStyle::CaseInsensitive;
        c.sort_directories_first = true;
        c.show_hidden_files = true;
        c.theme_name = "Glass1".to_string();
        c.autosave = false;

        let rec = c.to_rec();
        let c2 = emFileManConfigData::from_rec(&rec).unwrap();

        assert_eq!(c2.sort_criterion, SortCriterion::ByDate);
        assert_eq!(c2.name_sorting_style, NameSortingStyle::CaseInsensitive);
        assert!(c2.sort_directories_first);
        assert!(c2.show_hidden_files);
        assert_eq!(c2.theme_name, "Glass1");
        assert!(!c2.autosave);
    }

    #[test]
    fn sort_criterion_values_match_cpp() {
        assert_eq!(SortCriterion::ByName as i32, 0);
        assert_eq!(SortCriterion::ByEnding as i32, 1);
        assert_eq!(SortCriterion::ByClass as i32, 2);
        assert_eq!(SortCriterion::ByVersion as i32, 3);
        assert_eq!(SortCriterion::ByDate as i32, 4);
        assert_eq!(SortCriterion::BySize as i32, 5);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo-nextest ntr -p emfileman`
Expected: FAIL — types not defined.

- [ ] **Step 3: Implement emFileManConfig**

```rust
use emcore::emRecRecord::Record;
use emcore::emRec::{RecStruct, RecError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum SortCriterion {
    ByName = 0,
    ByEnding = 1,
    ByClass = 2,
    ByVersion = 3,
    ByDate = 4,
    BySize = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum NameSortingStyle {
    PerLocale = 0,
    CaseSensitive = 1,
    CaseInsensitive = 2,
}

#[derive(Clone, Debug)]
pub struct emFileManConfigData {
    pub sort_criterion: SortCriterion,
    pub name_sorting_style: NameSortingStyle,
    pub sort_directories_first: bool,
    pub show_hidden_files: bool,
    pub theme_name: String,
    pub autosave: bool,
}
```

Implement `Default` (matching C++ defaults: ByName, PerLocale, false, false, "", true).
Implement `Record` trait (`from_rec`, `to_rec`, `SetToDefault`, `IsSetToDefault`).

The Record format name is `"emFileManConfig"`. Field names match C++: `"SortCriterion"` (ident: `"SORT_BY_NAME"` etc.), `"NameSortingStyle"` (ident: `"NSS_PER_LOCALE"` etc.), `"SortDirectoriesFirst"`, `"ShowHiddenFiles"`, `"ThemeName"`, `"Autosave"`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo-nextest ntr -p emfileman`
Expected: PASS.

- [ ] **Step 5: Add module to lib.rs, run clippy**

Add `pub mod emFileManConfig;` to lib.rs.

Run: `cargo clippy -p emfileman -- -D warnings`

- [ ] **Step 6: Commit**

```bash
git add crates/emfileman/src/emFileManConfig.rs crates/emfileman/src/lib.rs
git commit -m "feat(emFileManConfig): global config with 6 fields and Record round-trip"
```

---

## Task 4: emFileManTheme — Theme Configuration

**Files:**
- Create: `crates/emfileman/src/emFileManTheme.rs`
- Modify: `crates/emfileman/src/lib.rs`

Port of C++ `emFileManTheme` — ~100 fields organized into colors, dimensions, alignment, strings, and image file references. All Record fields loaded from `.emFileManTheme` config files.

This is the largest single type. The implementation is mechanical: define the struct with all fields, implement Record.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use emcore::emRecRecord::Record;

    #[test]
    fn default_has_reasonable_height() {
        let t = emFileManThemeData::default();
        assert!(t.Height > 0.0);
    }

    #[test]
    fn record_round_trip_preserves_colors() {
        let mut t = emFileManThemeData::default();
        t.BackgroundColor = 0xAABBCCFF;
        t.SourceSelectionColor = 0x11223344;
        t.Height = 1.5;
        t.DisplayName = "TestTheme".to_string();

        let rec = t.to_rec();
        let t2 = emFileManThemeData::from_rec(&rec).unwrap();

        assert_eq!(t2.BackgroundColor, 0xAABBCCFF);
        assert_eq!(t2.SourceSelectionColor, 0x11223344);
        assert!((t2.Height - 1.5).abs() < f64::EPSILON);
        assert_eq!(t2.DisplayName, "TestTheme");
    }

    #[test]
    fn all_dimension_fields_exist() {
        let t = emFileManThemeData::default();
        // Spot-check field existence — won't compile if missing
        let _ = t.BackgroundX;
        let _ = t.BackgroundY;
        let _ = t.BackgroundW;
        let _ = t.BackgroundH;
        let _ = t.BackgroundRX;
        let _ = t.BackgroundRY;
        let _ = t.NameX;
        let _ = t.NameY;
        let _ = t.NameW;
        let _ = t.NameH;
        let _ = t.MinContentVW;
        let _ = t.MinAltVW;
        let _ = t.DirPaddingL;
        let _ = t.LnkPaddingL;
    }

    #[test]
    fn load_cpp_theme_file() {
        // Load a real theme file from the C++ source tree
        let theme_dir = std::path::Path::new(
            concat!(env!("HOME"), "/git/eaglemode-0.96.4/res/emFileMan/themes")
        );
        if !theme_dir.exists() {
            eprintln!("Skipping: C++ theme dir not found");
            return;
        }
        // Find any .emFileManTheme file
        let entry = std::fs::read_dir(theme_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.path().extension().map_or(false, |ext| {
                e.path().to_str().unwrap_or("").ends_with(".emFileManTheme")
            }));
        if let Some(entry) = entry {
            let content = std::fs::read_to_string(entry.path()).unwrap();
            let rec = emcore::emRec::parse_rec_file(&content).unwrap();
            let theme = emFileManThemeData::from_rec(&rec).unwrap();
            assert!(!theme.DisplayName.is_empty());
            assert!(theme.Height > 0.0);
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo-nextest ntr -p emfileman`
Expected: FAIL.

- [ ] **Step 3: Implement emFileManThemeData**

Define `emFileManThemeData` struct with all fields matching the C++ header exactly:
- 2 string fields: `DisplayName`, `DisplayIcon`
- 17 color fields (u32): `BackgroundColor`, `SourceSelectionColor`, `TargetSelectionColor`, `NormalNameColor`, `ExeNameColor`, `DirNameColor`, `FifoNameColor`, `BlkNameColor`, `ChrNameColor`, `SockNameColor`, `OtherNameColor`, `PathColor`, `SymLinkColor`, `LabelColor`, `InfoColor`, `FileContentColor`, `DirContentColor`
- ~80 f64 dimension fields: `Height`, `BackgroundX/Y/W/H/RX/RY`, `OuterBorderX/Y/W/H/L/T/R/B`, `NameX/Y/W/H`, `PathX/Y/W/H`, `InfoX/Y/W/H`, `FileInnerBorderX/Y/W/H/L/T/R/B`, `FileContentX/Y/W/H`, `DirInnerBorderX/Y/W/H/L/T/R/B`, `DirContentX/Y/W/H`, `AltX/Y/W/H`, `AltLabelX/Y/W/H`, `AltPathX/Y/W/H`, `AltAltX/Y/W/H`, `AltInnerBorderX/Y/W/H/L/T/R/B`, `AltContentX/Y/W/H`, `MinContentVW`, `MinAltVW`, `DirPaddingL/T/R/B`, `LnkPaddingL/T/R/B`
- 5 alignment fields (i32): `NameAlignment`, `PathAlignment`, `InfoAlignment`, `AltLabelAlignment`, `AltPathAlignment`
- 4 image file path strings + 4x4 i32 image border values: `OuterBorderImg`/`ImgL/T/R/B`, `FileInnerBorderImg`/`ImgL/T/R/B`, `DirInnerBorderImg`/`ImgL/T/R/B`, `AltInnerBorderImg`/`ImgL/T/R/B`

Implement `Default` with zeroed dimensions and black colors.
Implement `Record` trait with all field names matching C++ exactly.

The format name is `"emFileManTheme"`.

Constants:
```rust
pub const THEME_FILE_ENDING: &str = ".emFileManTheme";

pub fn GetThemesDirPath() -> Result<std::path::PathBuf, emcore::emInstallInfo::InstallInfoError> {
    emcore::emInstallInfo::emGetInstallPath(
        emcore::emInstallInfo::InstallDirType::Res,
        "emFileMan",
        Some("themes"),
    )
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo-nextest ntr -p emfileman`
Expected: PASS.

- [ ] **Step 5: Add module to lib.rs, run clippy**

Run: `cargo clippy -p emfileman -- -D warnings`

- [ ] **Step 6: Commit**

```bash
git add crates/emfileman/src/emFileManTheme.rs crates/emfileman/src/lib.rs
git commit -m "feat(emFileManTheme): ~100 layout/color params with Record round-trip"
```

---

## Task 5: emFileManThemeNames — Theme Catalog

**Files:**
- Create: `crates/emfileman/src/emFileManThemeNames.rs`
- Modify: `crates/emfileman/src/lib.rs`

Port of C++ `emFileManThemeNames`. SPLIT from `emFileManTheme.h`. Scans the themes directory, groups by display name (style) and aspect ratio.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_to_aspect_ratio_string() {
        assert_eq!(HeightToAspectRatioString(1.0), "1:1");
        assert_eq!(HeightToAspectRatioString(0.5), "2:1");
        assert_eq!(HeightToAspectRatioString(2.0), "1:2");
    }

    #[test]
    fn empty_catalog() {
        let names = emFileManThemeNames::from_themes(&[]);
        assert_eq!(names.GetThemeStyleCount(), 0);
        assert!(!names.IsExistingThemeName("anything"));
        assert!(names.GetDefaultThemeName().is_empty());
    }

    #[test]
    fn single_theme() {
        use crate::emFileManTheme::emFileManThemeData;
        let mut t = emFileManThemeData::default();
        t.DisplayName = "Glass".to_string();
        t.Height = 1.5;

        let names = emFileManThemeNames::from_themes(&[("Glass1", &t)]);
        assert_eq!(names.GetThemeStyleCount(), 1);
        assert!(names.IsExistingThemeName("Glass1"));
        assert_eq!(names.GetThemeStyleIndex("Glass1"), Some(0));
        assert_eq!(names.GetThemeAspectRatioIndex("Glass1"), Some(0));
        assert_eq!(names.GetThemeName(0, 0), Some("Glass1".to_string()));
    }

    #[test]
    fn multiple_aspect_ratios_same_style() {
        use crate::emFileManTheme::emFileManThemeData;
        let mut t1 = emFileManThemeData::default();
        t1.DisplayName = "Glass".to_string();
        t1.Height = 1.0;

        let mut t2 = emFileManThemeData::default();
        t2.DisplayName = "Glass".to_string();
        t2.Height = 2.0;

        let names = emFileManThemeNames::from_themes(&[("Glass1", &t1), ("Glass2", &t2)]);
        assert_eq!(names.GetThemeStyleCount(), 1);
        assert_eq!(names.GetThemeAspectRatioCount(0), 2);
        // Sorted by height: Glass1 (1.0) before Glass2 (2.0)
        assert_eq!(names.GetThemeName(0, 0), Some("Glass1".to_string()));
        assert_eq!(names.GetThemeName(0, 1), Some("Glass2".to_string()));
    }

    #[test]
    fn default_theme_name_prefers_glass1() {
        use crate::emFileManTheme::emFileManThemeData;
        let mut t = emFileManThemeData::default();
        t.DisplayName = "Glass".to_string();
        t.Height = 1.0;

        let names = emFileManThemeNames::from_themes(&[("Glass1", &t), ("Other1", &t)]);
        assert_eq!(names.GetDefaultThemeName(), "Glass1");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo-nextest ntr -p emfileman`

- [ ] **Step 3: Implement emFileManThemeNames**

```rust
// SPLIT: emFileManTheme.h — emFileManThemeNames split into separate file per one-type-per-file rule.

use std::collections::BTreeMap;
use crate::emFileManTheme::emFileManThemeData;

struct ThemeAR {
    name: String,
    aspect_ratio: String,
    height: f64,
}

struct ThemeStyle {
    display_name: String,
    display_icon: String,
    theme_ars: Vec<ThemeAR>,
}

pub struct emFileManThemeNames {
    styles: Vec<ThemeStyle>,
    name_to_packed_index: BTreeMap<String, (usize, usize)>,
}
```

Methods matching C++ names:
- `GetThemeStyleCount() -> usize`
- `GetThemeAspectRatioCount(style_index: usize) -> usize`
- `GetThemeName(style_index: usize, ar_index: usize) -> Option<String>`
- `GetDefaultThemeName() -> String` (prefers "Glass1")
- `GetThemeStyleDisplayName(style_index) -> Option<&str>`
- `GetThemeStyleDisplayIcon(style_index) -> Option<&str>`
- `GetThemeAspectRatio(style_index, ar_index) -> Option<&str>`
- `IsExistingThemeName(name: &str) -> bool`
- `GetThemeStyleIndex(name: &str) -> Option<usize>`
- `GetThemeAspectRatioIndex(name: &str) -> Option<usize>`

Helper: `HeightToAspectRatioString(height: f64) -> String` — port of C++ algorithm: tries denominators 1..10, finds best n:d ratio.

Constructor `from_themes(&[(name, theme_data)])` for unit tests. Full constructor `from_directory(dir: &Path)` that scans `.emFileManTheme` files.

- [ ] **Step 4: Run tests, clippy**

Run: `cargo-nextest ntr -p emfileman && cargo clippy -p emfileman -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileManThemeNames.rs crates/emfileman/src/lib.rs
git commit -m "feat(emFileManThemeNames): theme catalog with style/aspect-ratio grouping"
```

---

## Task 6: emDirModel — Directory Loading State Machine

**Files:**
- Create: `crates/emfileman/src/emDirModel.rs`
- Modify: `crates/emfileman/src/lib.rs`

Port of C++ `emDirModel`. Three-phase incremental loading via `FileModelOps` trait.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let m = emDirModelData::new();
        assert_eq!(m.GetEntryCount(), 0);
    }

    #[test]
    fn load_tmp_directory() {
        let mut m = emDirModelData::new();
        m.try_start_loading_from("/tmp").unwrap();
        // Drive through loading phases
        while !m.try_continue_loading().unwrap() {}
        m.quit_loading();
        assert!(m.GetEntryCount() > 0);
        // Entries are sorted by name
        for i in 1..m.GetEntryCount() {
            assert!(m.GetEntry(i - 1).GetName() <= m.GetEntry(i).GetName());
        }
    }

    #[test]
    fn get_entry_index_binary_search() {
        let mut m = emDirModelData::new();
        m.try_start_loading_from("/tmp").unwrap();
        while !m.try_continue_loading().unwrap() {}
        m.quit_loading();

        if m.GetEntryCount() > 0 {
            let name = m.GetEntry(0).GetName().to_string();
            assert_eq!(m.GetEntryIndex(&name), Some(0));
        }
        assert_eq!(m.GetEntryIndex("__nonexistent_emfileman__"), None);
    }

    #[test]
    fn deduplication() {
        // Can't easily test with real fs, but verify sorted entries have no dupes
        let mut m = emDirModelData::new();
        m.try_start_loading_from("/tmp").unwrap();
        while !m.try_continue_loading().unwrap() {}
        m.quit_loading();

        for i in 1..m.GetEntryCount() {
            assert_ne!(m.GetEntry(i - 1).GetName(), m.GetEntry(i).GetName());
        }
    }

    #[test]
    fn memory_need_scales_with_entries() {
        let mut m = emDirModelData::new();
        m.try_start_loading_from("/tmp").unwrap();
        while !m.try_continue_loading().unwrap() {}
        m.quit_loading();
        // C++ uses entry_count * 8192
        assert_eq!(m.calc_memory_need(), m.name_count as u64 * 8192);
    }

    #[test]
    fn is_out_of_date_always_true() {
        let m = emDirModelData::new();
        assert!(m.IsOutOfDate());
    }

    #[test]
    fn progress_calculation() {
        let mut m = emDirModelData::new();
        // Before loading, progress should handle gracefully
        assert!((m.calc_file_progress() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn reset_data_clears_entries() {
        let mut m = emDirModelData::new();
        m.try_start_loading_from("/tmp").unwrap();
        while !m.try_continue_loading().unwrap() {}
        m.quit_loading();
        let count = m.GetEntryCount();
        assert!(count > 0);
        m.reset_data();
        assert_eq!(m.GetEntryCount(), 0);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo-nextest ntr -p emfileman`

- [ ] **Step 3: Implement emDirModel**

The data-only part of the model (not the emFileModel wrapper — that comes when we integrate with the panel system):

```rust
use crate::emDirEntry::emDirEntry;

pub struct emDirModelData {
    entries: Vec<emDirEntry>,
    // Loading state
    dir_handle: Option<*mut libc::DIR>,
    names: Vec<String>,
    name_count: usize,
    entry_index: usize,
    loading_phase: LoadingPhase,
}

enum LoadingPhase {
    Idle,
    ReadingNames,
    SortedAndAllocated,
    LoadingEntries,
    Done,
}
```

Three-phase loading (matches C++):
1. `ReadingNames`: reads one name per `try_continue_loading()` call via `libc::readdir`
2. `SortedAndAllocated`: sort names, remove duplicates, allocate entries vec
3. `LoadingEntries`: load one `emDirEntry` per call from sorted names

Implement `FileModelOps` trait methods:
- `reset_data()` — clear entries
- `try_start_loading()` — open directory with `libc::opendir`
- `try_continue_loading()` — advance one step, return `Ok(true)` when done
- `quit_loading()` — close dir handle, free temp names
- `calc_memory_need()` — `name_count * 8192`
- `calc_file_progress()` — sqrt-based during phase 1, linear during phases 2-3
- `IsOutOfDate()` — always `true`

Public accessors: `GetEntryCount()`, `GetEntry(index)`, `GetEntryIndex(name)` (binary search on sorted entries).

- [ ] **Step 4: Run tests, clippy**

Run: `cargo-nextest ntr -p emfileman && cargo clippy -p emfileman -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emDirModel.rs crates/emfileman/src/lib.rs
git commit -m "feat(emDirModel): 3-phase incremental directory loading with dedup and binary search"
```

---

## Task 7: emFileManModel — Selection Subsystem

**Files:**
- Create: `crates/emfileman/src/emFileManModel.rs`
- Modify: `crates/emfileman/src/lib.rs`

Port the selection subsystem of C++ `emFileManModel`. The command tree and IPC will be added in Tasks 8 and 9.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_selections() {
        let m = SelectionManager::new();
        assert_eq!(m.GetSourceSelectionCount(), 0);
        assert_eq!(m.GetTargetSelectionCount(), 0);
        assert!(!m.IsSelectedAsSource("/foo"));
        assert!(!m.IsSelectedAsTarget("/foo"));
    }

    #[test]
    fn select_and_deselect_source() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/foo");
        assert!(m.IsSelectedAsSource("/foo"));
        assert_eq!(m.GetSourceSelectionCount(), 1);

        m.DeselectAsSource("/foo");
        assert!(!m.IsSelectedAsSource("/foo"));
        assert_eq!(m.GetSourceSelectionCount(), 0);
    }

    #[test]
    fn select_and_deselect_target() {
        let mut m = SelectionManager::new();
        m.SelectAsTarget("/bar");
        assert!(m.IsSelectedAsTarget("/bar"));
        assert_eq!(m.GetTargetSelectionCount(), 1);

        m.DeselectAsTarget("/bar");
        assert!(!m.IsSelectedAsTarget("/bar"));
    }

    #[test]
    fn duplicate_select_is_idempotent() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/foo");
        m.SelectAsSource("/foo");
        assert_eq!(m.GetSourceSelectionCount(), 1);
    }

    #[test]
    fn swap_selection() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/src1");
        m.SelectAsTarget("/tgt1");
        m.SwapSelection();
        assert!(m.IsSelectedAsTarget("/src1"));
        assert!(m.IsSelectedAsSource("/tgt1"));
    }

    #[test]
    fn clear_selections() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/s1");
        m.SelectAsSource("/s2");
        m.SelectAsTarget("/t1");
        m.ClearSourceSelection();
        assert_eq!(m.GetSourceSelectionCount(), 0);
        assert_eq!(m.GetTargetSelectionCount(), 1);
        m.ClearTargetSelection();
        assert_eq!(m.GetTargetSelectionCount(), 0);
    }

    #[test]
    fn hash_binary_search_ordering() {
        let mut m = SelectionManager::new();
        // Insert in arbitrary order
        m.SelectAsTarget("/z/last");
        m.SelectAsTarget("/a/first");
        m.SelectAsTarget("/m/middle");
        assert_eq!(m.GetTargetSelectionCount(), 3);
        assert!(m.IsSelectedAsTarget("/a/first"));
        assert!(m.IsSelectedAsTarget("/m/middle"));
        assert!(m.IsSelectedAsTarget("/z/last"));
    }

    #[test]
    fn get_selection_by_index() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/b");
        m.SelectAsSource("/a");
        // Entries sorted by (hash, path) — order depends on hash function
        assert_eq!(m.GetSourceSelectionCount(), 2);
        let s0 = m.GetSourceSelection(0);
        let s1 = m.GetSourceSelection(1);
        assert!(s0 == "/a" || s0 == "/b");
        assert!(s1 == "/a" || s1 == "/b");
        assert_ne!(s0, s1);
    }

    #[test]
    fn is_any_selection_in_dir_tree() {
        let mut m = SelectionManager::new();
        m.SelectAsTarget("/home/user/docs/file.txt");
        assert!(m.IsAnySelectionInDirTree("/home/user/docs"));
        assert!(m.IsAnySelectionInDirTree("/home/user"));
        assert!(m.IsAnySelectionInDirTree("/home"));
        assert!(!m.IsAnySelectionInDirTree("/tmp"));
    }

    #[test]
    fn update_selection_removes_nonexistent() {
        let mut m = SelectionManager::new();
        m.SelectAsTarget("/dev/null"); // exists
        m.SelectAsTarget("/nonexistent_emfileman_test"); // doesn't exist
        assert_eq!(m.GetTargetSelectionCount(), 2);
        m.UpdateSelection();
        assert_eq!(m.GetTargetSelectionCount(), 1);
        assert!(m.IsSelectedAsTarget("/dev/null"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo-nextest ntr -p emfileman`

- [ ] **Step 3: Implement SelectionManager**

Port the C++ `SearchSelection` binary search (hash first, then string compare). The selection arrays are sorted by `(hash_code, path)` for O(log n) lookup.

```rust
use emcore::emStd2::emCalcHashCode;

struct SelEntry {
    hash_code: i32,
    path: String,
}

pub struct SelectionManager {
    sel: [Vec<SelEntry>; 2], // 0=source, 1=target
    shift_tgt_sel_path: String,
    sel_cmd_counter: u32,
}
```

Implement all C++-matching methods: `SelectAsSource`, `SelectAsTarget`, `DeselectAsSource`, `DeselectAsTarget`, `IsSelectedAsSource`, `IsSelectedAsTarget`, `ClearSourceSelection`, `ClearTargetSelection`, `SwapSelection`, `UpdateSelection`, `GetSourceSelectionCount`, `GetTargetSelectionCount`, `GetSourceSelection(index)`, `GetTargetSelection(index)`, `IsAnySelectionInDirTree`, `GetShiftTgtSelPath`, `SetShiftTgtSelPath`.

`SearchSelection` returns `Ok(index)` if found, `Err(insert_index)` if not — same as C++ convention where negative return = `~insert_pos`.

- [ ] **Step 4: Run tests, clippy**

Run: `cargo-nextest ntr -p emfileman && cargo clippy -p emfileman -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileManModel.rs crates/emfileman/src/lib.rs
git commit -m "feat(emFileManModel): selection subsystem with hash-based binary search"
```

---

## Task 8: emFileManModel — Command Tree

**Files:**
- Modify: `crates/emfileman/src/emFileManModel.rs`

Add command loading, property parsing, search, and CRC-based hot reload detection.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod command_tests {
    use super::*;

    #[test]
    fn parse_command_properties() {
        let content = r#"#!/bin/bash
# [[BEGIN PROPERTIES]]
# Type = Command
# Order = 1.5
# Interpreter = bash
# Caption = Test Command
# Description = A test command
# DefaultFor = .txt:.rs
# [[END PROPERTIES]]
echo "hello"
"#;
        let cmd = parse_command_properties(content, "/test/cmd.sh").unwrap();
        assert_eq!(cmd.command_type, CommandType::Command);
        assert!((cmd.order - 1.5).abs() < f64::EPSILON);
        assert_eq!(cmd.interpreter, "bash");
        assert_eq!(cmd.caption, "Test Command");
        assert_eq!(cmd.description, "A test command");
        assert_eq!(cmd.default_for, ".txt:.rs");
    }

    #[test]
    fn parse_group_properties() {
        let content = r#"#!/bin/bash
# [[BEGIN PROPERTIES]]
# Type = Group
# Order = 2.0
# Directory = subdir
# Caption = My Group
# [[END PROPERTIES]]
"#;
        let cmd = parse_command_properties(content, "/test/group.sh").unwrap();
        assert_eq!(cmd.command_type, CommandType::Group);
    }

    #[test]
    fn parse_separator() {
        let content = "# [[BEGIN PROPERTIES]]\n# Type = Separator\n# [[END PROPERTIES]]\n";
        let cmd = parse_command_properties(content, "/test/sep.sh").unwrap();
        assert_eq!(cmd.command_type, CommandType::Separator);
    }

    #[test]
    fn check_default_command_for_extension() {
        let cmd = CommandNode {
            default_for: ".txt:.rs".to_string(),
            command_type: CommandType::Command,
            ..CommandNode::default()
        };
        assert_eq!(CheckDefaultCommand(&cmd, "/foo/bar.txt"), 5); // ".txt".len() + 1
        assert_eq!(CheckDefaultCommand(&cmd, "/foo/bar.rs"), 4);  // ".rs".len() + 1
        assert_eq!(CheckDefaultCommand(&cmd, "/foo/bar.py"), 0);
    }

    #[test]
    fn check_default_command_for_file_keyword() {
        let cmd = CommandNode {
            default_for: "file".to_string(),
            command_type: CommandType::Command,
            ..CommandNode::default()
        };
        // Returns 1 for regular files (priority 1 = "file" keyword match)
        assert_eq!(CheckDefaultCommand(&cmd, "/dev/null"), 0); // not regular file
    }

    #[test]
    fn check_command_file_ending() {
        assert!(check_command_file_ending("test.sh"));
        assert!(check_command_file_ending("test.py"));
        assert!(check_command_file_ending("test.pl"));
        assert!(check_command_file_ending("test.js"));
        assert!(check_command_file_ending("test.props"));
        assert!(!check_command_file_ending("test.exe"));
        assert!(!check_command_file_ending("test.txt"));
    }

    #[test]
    fn command_run_id_changes() {
        let mut m = SelectionManager::new();
        let id1 = m.GetCommandRunId();
        m.SelectAsSource("/foo");
        let id2 = m.GetCommandRunId();
        assert_ne!(id1, id2);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo-nextest ntr -p emfileman`

- [ ] **Step 3: Implement command tree**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandType {
    Command,
    Group,
    Separator,
}

#[derive(Clone, Debug)]
pub struct CommandNode {
    pub cmd_path: String,
    pub command_type: CommandType,
    pub order: f64,
    pub interpreter: String,
    pub dir: String,
    pub default_for: String,
    pub caption: String,
    pub description: String,
    pub border_scaling: f64,
    pub pref_child_tallness: f64,
    pub children: Vec<Box<CommandNode>>,
    pub dir_crc: u64,
}
```

Functions:
- `parse_command_properties(content: &str, cmd_path: &str) -> Result<CommandNode, String>` — parse `#[[BEGIN PROPERTIES]]` blocks
- `check_command_file_ending(name: &str) -> bool` — checks `.sh`, `.py`, `.pl`, `.js`, `.props`
- `CheckDefaultCommand(cmd: &CommandNode, file_path: &str) -> i32` — extension matching priority
- `SearchDefaultCommandFor(root: &CommandNode, file_path: &str) -> Option<&CommandNode>` — DFS
- `SearchHotkeyCommand(root: &CommandNode, hotkey: &emInputHotkey) -> Option<&CommandNode>` — DFS
- `LoadCommands(root_dir: &str) -> CommandNode` — recursive filesystem scan
- `CalcDirCRC(dir: &str, names: &[String]) -> u64` — CRC64 of names+mtimes for change detection
- `CheckCRCs(parent: &CommandNode) -> bool` — verify no changes

Add to `SelectionManager`:
- `GetCommandRunId() -> String` — `format!("{}", sel_cmd_counter)`

- [ ] **Step 4: Run tests, clippy**

Run: `cargo-nextest ntr -p emfileman && cargo clippy -p emfileman -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileManModel.rs
git commit -m "feat(emFileManModel): command tree with property parsing, search, and CRC hot reload"
```

---

## Task 9: emFileManModel — IPC Server

**Files:**
- Modify: `crates/emfileman/src/emFileManModel.rs`

Add IPC message handling using `emMiniIpcServer`. Messages: `update`, `select <id> <paths...>`, `selectks <id> <paths...>`, `selectcs <id> <paths...>`.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod ipc_tests {
    use super::*;

    #[test]
    fn ipc_select_message() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/src1");
        let run_id = m.GetCommandRunId();

        m.handle_ipc_message(&["select", &run_id, "/new_target"]);

        // "select" swaps src→tgt, clears tgt, then deselects from src and selects as tgt
        assert!(m.IsSelectedAsTarget("/src1") || m.IsSelectedAsTarget("/new_target"));
    }

    #[test]
    fn ipc_selectks_message() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/src1");
        m.SelectAsTarget("/old_tgt");
        let run_id = m.GetCommandRunId();

        m.handle_ipc_message(&["selectks", &run_id, "/new_target"]);

        // "selectks" keeps source, clears tgt, deselects from src, selects as tgt
        assert!(m.IsSelectedAsTarget("/new_target"));
        assert!(!m.IsSelectedAsTarget("/old_tgt"));
    }

    #[test]
    fn ipc_selectcs_message() {
        let mut m = SelectionManager::new();
        m.SelectAsSource("/src1");
        m.SelectAsTarget("/tgt1");
        let run_id = m.GetCommandRunId();

        m.handle_ipc_message(&["selectcs", &run_id, "/new"]);

        // "selectcs" clears both, selects paths as target
        assert_eq!(m.GetSourceSelectionCount(), 0);
        assert!(m.IsSelectedAsTarget("/new"));
    }

    #[test]
    fn ipc_stale_run_id_ignored() {
        let mut m = SelectionManager::new();
        m.SelectAsTarget("/existing");

        m.handle_ipc_message(&["select", "wrong_id", "/new"]);

        // Stale ID: selection unchanged
        assert!(m.IsSelectedAsTarget("/existing"));
        assert!(!m.IsSelectedAsTarget("/new"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo-nextest ntr -p emfileman`

- [ ] **Step 3: Implement IPC handling**

Add `handle_ipc_message(&mut self, args: &[&str])` to `SelectionManager` matching the C++ `OnIpcReception` logic:

```rust
pub fn handle_ipc_message(&mut self, args: &[&str]) {
    if args.len() == 1 && args[0] == "update" {
        // Signal file update (caller handles this)
        return;
    }
    if args.len() >= 2 && args[0] == "select" {
        if self.GetCommandRunId() == args[1] {
            self.SwapSelection();
            self.ClearTargetSelection();
            for path in &args[2..] {
                self.DeselectAsSource(path);
                self.SelectAsTarget(path);
            }
        }
    } else if args.len() >= 2 && args[0] == "selectks" {
        if self.GetCommandRunId() == args[1] {
            self.ClearTargetSelection();
            for path in &args[2..] {
                self.DeselectAsSource(path);
                self.SelectAsTarget(path);
            }
        }
    } else if args.len() >= 2 && args[0] == "selectcs" {
        if self.GetCommandRunId() == args[1] {
            self.ClearSourceSelection();
            self.ClearTargetSelection();
            for path in &args[2..] {
                self.SelectAsTarget(path);
            }
        }
    }
}
```

- [ ] **Step 4: Run tests, clippy**

Run: `cargo-nextest ntr -p emfileman && cargo clippy -p emfileman -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileManModel.rs
git commit -m "feat(emFileManModel): IPC message handling for select/selectks/selectcs"
```

---

## Task 10: emFileManViewConfig — Config Bridge and Sorting

**Files:**
- Create: `crates/emfileman/src/emFileManViewConfig.rs`
- Modify: `crates/emfileman/src/lib.rs`

Port of C++ `emFileManViewConfig`. The critical piece is `CompareDirEntries` with 6 sort criteria.

- [ ] **Step 1: Write failing tests for CompareDirEntries**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::emDirEntry::emDirEntry;
    use crate::emFileManConfig::{SortCriterion, NameSortingStyle};
    use std::cmp::Ordering;

    fn make_config(sc: SortCriterion, nss: NameSortingStyle, dirs_first: bool) -> SortConfig {
        SortConfig { sort_criterion: sc, name_sorting_style: nss, sort_directories_first: dirs_first }
    }

    #[test]
    fn sort_by_name_basic() {
        let cfg = make_config(SortCriterion::ByName, NameSortingStyle::CaseSensitive, false);
        let e1 = emDirEntry::from_path("/tmp"); // creates entry with name "tmp"
        let e2 = emDirEntry::from_path("/dev"); // creates entry with name "dev"
        // "dev" < "tmp" case-sensitive
        let cmp = CompareDirEntries(&e1, &e2, &cfg);
        assert!(cmp > 0); // tmp > dev
    }

    #[test]
    fn sort_by_name_case_insensitive() {
        let cfg = make_config(SortCriterion::ByName, NameSortingStyle::CaseInsensitive, false);
        let result = CompareNames("ABC", "abc", NameSortingStyle::CaseInsensitive);
        assert_eq!(result, 0);
    }

    #[test]
    fn sort_directories_first() {
        let cfg = make_config(SortCriterion::ByName, NameSortingStyle::CaseSensitive, true);
        let dir = emDirEntry::from_path("/tmp");      // directory
        let file = emDirEntry::from_path("/dev/null"); // not a directory
        let cmp = CompareDirEntries(&dir, &file, &cfg);
        assert!(cmp < 0); // dir comes first
    }

    #[test]
    fn sort_by_version_numeric() {
        let cfg = make_config(SortCriterion::ByVersion, NameSortingStyle::CaseSensitive, false);
        // Test the version comparison logic directly
        let result = compare_version_names("file-2.9", "file-2.10",
            NameSortingStyle::CaseSensitive);
        assert!(result < 0); // 2.9 < 2.10
    }

    #[test]
    fn compare_names_per_locale() {
        let result = CompareNames("hello", "world", NameSortingStyle::PerLocale);
        assert!(result < 0); // h < w in any locale
    }

    #[test]
    fn compare_names_case_sensitive() {
        let result = CompareNames("A", "a", NameSortingStyle::CaseSensitive);
        assert!(result < 0); // 'A' (65) < 'a' (97)
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Implement emFileManViewConfig**

```rust
use crate::emDirEntry::emDirEntry;
use crate::emFileManConfig::{SortCriterion, NameSortingStyle};

pub struct SortConfig {
    pub sort_criterion: SortCriterion,
    pub name_sorting_style: NameSortingStyle,
    pub sort_directories_first: bool,
}
```

Port `CompareDirEntries` exactly from C++ — this is the core sorting logic:
- Directories-first pre-filter
- 6 criteria: ByName (fallthrough), ByEnding (extension compare), ByClass (right-to-left word-class), ByVersion (numeric-aware), ByDate (st_mtime), BySize (st_size)
- Name fallback via `CompareNames`

Port `CompareNames`:
- `PerLocale` → `libc::strcoll` via CString
- `CaseSensitive` → `strcmp` equivalent (byte comparison)
- `CaseInsensitive` → `libc::strcasecmp` via CString

Port `ByClass` comparison exactly from C++ — right-to-left word splitting into alpha/digit/other classes.

Port `ByVersion` comparison exactly from C++ — find divergence point, scan back to digit boundary, compare numeric segments.

Full `emFileManViewConfig` struct (with all 6 config fields, autosave logic, theme reference) will be a wrapper around `SortConfig` + the config bridge logic. The `RevisitEngine` (saves/restores view position on theme change) is deferred until panel integration.

- [ ] **Step 4: Run tests, clippy**

Run: `cargo-nextest ntr -p emfileman && cargo clippy -p emfileman -- -D warnings`

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileManViewConfig.rs crates/emfileman/src/lib.rs
git commit -m "feat(emFileManViewConfig): CompareDirEntries with 6 sort criteria"
```

---

## Task 11: emFileLinkModel — Link File Parser

**Files:**
- Create: `crates/emfileman/src/emFileLinkModel.rs`
- Modify: `crates/emfileman/src/lib.rs`

Port of C++ `emFileLinkModel` — an `emRecFileModel` with Record fields for BasePathType, BasePathProject, Path, HaveDirEntry. `GetFullPath()` resolves base path via `emGetInstallPath`.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use emcore::emRecRecord::Record;

    #[test]
    fn default_values() {
        let m = emFileLinkData::default();
        assert_eq!(m.base_path_type, BasePathType::None);
        assert!(m.base_path_project.is_empty());
        assert!(m.path.is_empty());
        assert!(!m.have_dir_entry);
    }

    #[test]
    fn record_round_trip() {
        let mut m = emFileLinkData::default();
        m.base_path_type = BasePathType::Res;
        m.base_path_project = "emFileMan".to_string();
        m.path = "themes".to_string();
        m.have_dir_entry = true;

        let rec = m.to_rec();
        let m2 = emFileLinkData::from_rec(&rec).unwrap();

        assert_eq!(m2.base_path_type, BasePathType::Res);
        assert_eq!(m2.base_path_project, "emFileMan");
        assert_eq!(m2.path, "themes");
        assert!(m2.have_dir_entry);
    }

    #[test]
    fn base_path_type_values_match_cpp() {
        assert_eq!(BasePathType::None as i32, 0);
        assert_eq!(BasePathType::Bin as i32, 1);
        assert_eq!(BasePathType::Include as i32, 2);
        assert_eq!(BasePathType::Lib as i32, 3);
        assert_eq!(BasePathType::HtmlDoc as i32, 4);
        assert_eq!(BasePathType::PdfDoc as i32, 5);
        assert_eq!(BasePathType::PsDoc as i32, 6);
        assert_eq!(BasePathType::UserConfig as i32, 7);
        assert_eq!(BasePathType::HostConfig as i32, 8);
        assert_eq!(BasePathType::Tmp as i32, 9);
        assert_eq!(BasePathType::Res as i32, 10);
        assert_eq!(BasePathType::Home as i32, 11);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Implement emFileLinkModel**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum BasePathType {
    None = 0,
    Bin = 1,
    Include = 2,
    Lib = 3,
    HtmlDoc = 4,
    PdfDoc = 5,
    PsDoc = 6,
    UserConfig = 7,
    HostConfig = 8,
    Tmp = 9,
    Res = 10,
    Home = 11,
}

#[derive(Clone, Debug)]
pub struct emFileLinkData {
    pub base_path_type: BasePathType,
    pub base_path_project: String,
    pub path: String,
    pub have_dir_entry: bool,
}
```

Implement `Default`, `Record`. Format name: `"emFileLink"`.

Implement `GetFullPath(file_path: &str) -> String`:
- Map `BasePathType` to `InstallDirType` and call `emGetInstallPath`
- For `None`, use parent of file_path as base
- Join with relative `path`
- Make absolute

- [ ] **Step 4: Run tests, clippy**

- [ ] **Step 5: Commit**

```bash
git add crates/emfileman/src/emFileLinkModel.rs crates/emfileman/src/lib.rs
git commit -m "feat(emFileLinkModel): link file parser with BasePathType resolution"
```

---

## Task 12: emDirStatPanel — Directory Statistics

**Files:**
- Create: `crates/emfileman/src/emDirStatPanel.rs`
- Modify: `crates/emfileman/src/lib.rs`

Port of C++ `emDirStatPanel`. Simple: counts entries by type from an `emDirModel` and renders formatted text.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::emDirEntry::emDirEntry;

    #[test]
    fn count_entries() {
        let entries = vec![
            emDirEntry::from_path("/tmp"),     // directory
            emDirEntry::from_path("/dev/null"), // char device
        ];
        let stats = DirStatistics::from_entries(&entries);
        assert_eq!(stats.total_count, 2);
        assert!(stats.sub_dir_count >= 1); // /tmp is a directory
    }

    #[test]
    fn empty_entries() {
        let stats = DirStatistics::from_entries(&[]);
        assert_eq!(stats.total_count, 0);
        assert_eq!(stats.file_count, 0);
        assert_eq!(stats.sub_dir_count, 0);
        assert_eq!(stats.other_type_count, 0);
        assert_eq!(stats.hidden_count, 0);
    }

    #[test]
    fn hidden_count() {
        let dir = std::env::temp_dir();
        let hidden = dir.join(".test_hidden_stat");
        std::fs::write(&hidden, "x").unwrap();
        let entries = vec![emDirEntry::from_path(hidden.to_str().unwrap())];
        let stats = DirStatistics::from_entries(&entries);
        assert_eq!(stats.hidden_count, 1);
        std::fs::remove_file(&hidden).unwrap();
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Implement DirStatistics and emDirStatPanel data**

```rust
pub struct DirStatistics {
    pub total_count: i32,
    pub file_count: i32,
    pub sub_dir_count: i32,
    pub other_type_count: i32,
    pub hidden_count: i32,
}

impl DirStatistics {
    pub fn from_entries(entries: &[emDirEntry]) -> Self { ... }
}
```

The panel rendering (`Paint`) renders the C++ format string:
```
Directory Statistics
~~~~~~~~~~~~~~~~~~~~

Total Entries : %5d
Hidden Entries: %5d
Regular Files : %5d
Subdirectories: %5d
Other Types   : %5d
```

- [ ] **Step 4: Run tests, clippy, commit**

```bash
git add crates/emfileman/src/emDirStatPanel.rs crates/emfileman/src/lib.rs
git commit -m "feat(emDirStatPanel): directory entry counting by type"
```

---

## Task 13: emDirEntryPanel — Data and Selection Logic

**Files:**
- Create: `crates/emfileman/src/emDirEntryPanel.rs`
- Modify: `crates/emfileman/src/lib.rs`

Port the data and selection logic of C++ `emDirEntryPanel`. Paint and layout deferred to Task 16 (panel integration). This task covers: UpdateBgColor, Select, SelectSolely, RunDefaultCommand, GetIconFileName with recursive guard, FormatTime.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_time_inline() {
        let t: libc::time_t = 1609459200; // 2021-01-01 00:00:00 UTC
        let s = FormatTime(t, false);
        assert!(s.contains("2021"));
        assert!(s.contains("01"));
    }

    #[test]
    fn format_time_newline() {
        let t: libc::time_t = 1609459200;
        let s = FormatTime(t, true);
        assert!(s.contains('\n'));
    }

    #[test]
    fn bg_color_no_selection() {
        let bg_color = 0x112233FF_u32; // theme background
        let result = compute_bg_color(false, false, bg_color, 0xAABBCCFF, 0xDDEEFFFF);
        assert_eq!(result, bg_color);
    }

    #[test]
    fn bg_color_source_selection() {
        let result = compute_bg_color(true, false, 0x112233FF, 0xAABBCCFF, 0xDDEEFFFF);
        assert_eq!(result, 0xAABBCCFF); // source selection color
    }

    #[test]
    fn bg_color_target_selection() {
        let result = compute_bg_color(false, true, 0x112233FF, 0xAABBCCFF, 0xDDEEFFFF);
        assert_eq!(result, 0xDDEEFFFF); // target selection color
    }

    #[test]
    fn bg_color_both_selections_blended() {
        let result = compute_bg_color(true, true, 0x112233FF, 0xAABBCCFF, 0xDDEEFFFF);
        // 50% blend of target + source
        assert_ne!(result, 0xAABBCCFF);
        assert_ne!(result, 0xDDEEFFFF);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Implement helper functions**

```rust
pub fn FormatTime(t: libc::time_t, nl: bool) -> String {
    // Port of C++ FormatTime using libc::localtime_r
}

pub fn compute_bg_color(
    sel_src: bool,
    sel_tgt: bool,
    bg_color: u32,
    source_sel_color: u32,
    target_sel_color: u32,
) -> u32 {
    // Port of C++ UpdateBgColor logic
}

pub const CONTENT_NAME: &str = "";
pub const ALT_NAME: &str = "a";
```

- [ ] **Step 4: Run tests, clippy, commit**

```bash
git add crates/emfileman/src/emDirEntryPanel.rs crates/emfileman/src/lib.rs
git commit -m "feat(emDirEntryPanel): selection bg color, FormatTime, content name constants"
```

---

## Task 14: emDirEntryAltPanel — Alternative Content Stub

**Files:**
- Create: `crates/emfileman/src/emDirEntryAltPanel.rs`
- Modify: `crates/emfileman/src/lib.rs`

Port of C++ `emDirEntryAltPanel`. Creates content via `CreateFilePanel(..., alternative)` with incrementing alternative index. Recursive nesting.

- [ ] **Step 1: Write a minimal test and implementation**

This is primarily panel rendering code. Create the module with the data structures and constants.

```rust
pub const CONTENT_NAME: &str = "";
pub const ALT_NAME: &str = "a";

/// Data for an alternative content view panel.
pub struct emDirEntryAltPanelData {
    pub dir_entry: crate::emDirEntry::emDirEntry,
    pub alternative: i32,
}
```

- [ ] **Step 2: Run cargo check, clippy, commit**

```bash
git add crates/emfileman/src/emDirEntryAltPanel.rs crates/emfileman/src/lib.rs
git commit -m "feat(emDirEntryAltPanel): alternative content data stub"
```

---

## Task 15: emDirPanel — Data and Grid Layout Logic

**Files:**
- Create: `crates/emfileman/src/emDirPanel.rs`
- Modify: `crates/emfileman/src/lib.rs`

Port the data structures and grid layout algorithm from C++ `emDirPanel`.

- [ ] **Step 1: Write failing tests for grid layout**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_layout_single_entry() {
        let rects = compute_grid_layout(1, 1.5, 1.0, 0.02, 0.02, 0.02, 0.02);
        assert_eq!(rects.len(), 1);
        assert!(rects[0].x >= 0.0);
        assert!(rects[0].y >= 0.0);
        assert!(rects[0].w > 0.0);
        assert!(rects[0].h > 0.0);
    }

    #[test]
    fn grid_layout_many_entries() {
        let rects = compute_grid_layout(20, 1.5, 1.0, 0.02, 0.02, 0.02, 0.02);
        assert_eq!(rects.len(), 20);
        // All within bounds [0,1] x [0,height]
        for r in &rects {
            assert!(r.x >= 0.0);
            assert!(r.x + r.w <= 1.0 + 1e-9);
        }
    }

    #[test]
    fn grid_layout_column_major() {
        // With height=1.5, theme_height=1.5, 4 entries: should be 2 rows x 2 cols
        let rects = compute_grid_layout(4, 1.5, 1.5, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(rects.len(), 4);
        // Column-major: entries[0] and [1] share same column (x)
        assert!((rects[0].x - rects[1].x).abs() < 1e-9);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Implement grid layout**

```rust
pub struct LayoutRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Port of C++ emDirPanel::LayoutChildren grid algorithm.
/// panel_height is GetHeight(), theme_height is theme.Height.
/// Padding values from theme: DirPaddingL/T/R/B.
pub fn compute_grid_layout(
    count: usize,
    theme_height: f64,
    panel_height: f64,
    pad_l: f64,
    pad_t: f64,
    pad_r: f64,
    pad_b: f64,
) -> Vec<LayoutRect> {
    // Port of C++ algorithm:
    // 1. Find rows such that rows*cols >= count
    // 2. cols = (count + rows - 1) / rows
    // 3. Compute cell width/height with padding
    // 4. Column-major layout
}
```

- [ ] **Step 4: Run tests, clippy, commit**

```bash
git add crates/emfileman/src/emDirPanel.rs crates/emfileman/src/lib.rs
git commit -m "feat(emDirPanel): grid layout algorithm for directory entries"
```

---

## Task 16: emFileLinkPanel — Link Display Data

**Files:**
- Create: `crates/emfileman/src/emFileLinkPanel.rs`
- Modify: `crates/emfileman/src/lib.rs`

Port the data structures and content coordinate calculation from C++ `emFileLinkPanel`.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_coords_no_border_no_dir_entry() {
        let (x, y, w, h) = CalcContentCoords(1.0, false, false, 1.5, 0.0, 0.0, 0.0, 0.0);
        assert!((x - 0.0).abs() < 1e-9);
        assert!((y - 0.0).abs() < 1e-9);
        assert!((w - 1.0).abs() < 1e-9);
    }

    #[test]
    fn content_coords_with_border() {
        let (x, y, w, h) = CalcContentCoords(1.0, true, false, 1.5, 0.0, 0.0, 0.0, 0.0);
        assert!(x > 0.0);
        assert!(y > 0.0);
        assert!(w < 1.0);
    }

    #[test]
    fn border_colors() {
        assert_eq!(BORDER_BG_COLOR, 0xBBBBBBFF_u32);
        assert_eq!(BORDER_FG_COLOR, 0x444444FF_u32);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Implement CalcContentCoords and constants**

Port of C++ `CalcContentCoords` logic. Constants:
```rust
pub const BORDER_BG_COLOR: u32 = 0xBBBBBBFF;
pub const BORDER_FG_COLOR: u32 = 0x444444FF;
```

- [ ] **Step 4: Run tests, clippy, commit**

```bash
git add crates/emfileman/src/emFileLinkPanel.rs crates/emfileman/src/lib.rs
git commit -m "feat(emFileLinkPanel): content coordinate calculation and border constants"
```

---

## Task 17: emFileManSelInfoPanel — Selection Statistics State Machine

**Files:**
- Create: `crates/emfileman/src/emFileManSelInfoPanel.rs`
- Modify: `crates/emfileman/src/lib.rs`

Port the state machine logic from C++ `emFileManSelInfoPanel`. State machine: COSTLY -> WAIT -> SCANNING -> SUCCESS/ERROR. Two parallel stat computations: direct and recursive.

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_costly() {
        let info = SelInfoState::new();
        assert_eq!(info.direct.state, ScanState::Costly);
        assert_eq!(info.recursive.state, ScanState::Costly);
    }

    #[test]
    fn work_on_detail_entry_counts_file() {
        let mut details = ScanDetails::new();
        let e = crate::emDirEntry::emDirEntry::from_path("/dev/null");
        work_on_detail_entry(&mut details, &e);
        assert_eq!(details.entries, 1);
    }

    #[test]
    fn work_on_detail_entry_counts_directory() {
        let mut details = ScanDetails::new();
        let e = crate::emDirEntry::emDirEntry::from_path("/tmp");
        let mut dir_stack = Vec::new();
        work_on_detail_entry_with_stack(&mut details, &e, &mut dir_stack);
        assert_eq!(details.subdirectories, 1);
        assert_eq!(dir_stack.len(), 1); // /tmp pushed for recursive scan
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

- [ ] **Step 3: Implement state machine**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanState {
    Costly,
    Wait,
    Scanning,
    Error,
    Success,
}

#[derive(Clone, Debug)]
pub struct ScanDetails {
    pub state: ScanState,
    pub error_message: String,
    pub entries: i32,
    pub hidden_entries: i32,
    pub symbolic_links: i32,
    pub regular_files: i32,
    pub subdirectories: i32,
    pub other_types: i32,
    pub size: u64,
    pub disk_usage: u64,
    pub disk_usage_unknown: bool,
}
```

- [ ] **Step 4: Run tests, clippy, commit**

```bash
git add crates/emfileman/src/emFileManSelInfoPanel.rs crates/emfileman/src/lib.rs
git commit -m "feat(emFileManSelInfoPanel): selection stats state machine with direct/recursive scanning"
```

---

## Task 18: emFileManControlPanel — Stub

**Files:**
- Create: `crates/emfileman/src/emFileManControlPanel.rs`
- Modify: `crates/emfileman/src/lib.rs`

The control panel is a UI widget (emLinearLayout with radio buttons, checkboxes, command groups). It requires the full widget toolkit. Create a stub module that declares the type.

- [ ] **Step 1: Create stub**

```rust
//! Sort/filter/theme UI control panel.
//!
//! Port of C++ `emFileManControlPanel`. Extends `emLinearLayout`.
//! Contains sort criterion radio buttons, name sorting style radio buttons,
//! directories-first and show-hidden checkboxes, theme selectors,
//! autosave checkbox, and command group buttons.

// Full implementation requires panel tree integration.
// Data structures and logic are in emFileManViewConfig (Task 10) and
// emFileManModel (Tasks 7-9).
```

- [ ] **Step 2: Cargo check, commit**

```bash
git add crates/emfileman/src/emFileManControlPanel.rs crates/emfileman/src/lib.rs
git commit -m "feat(emFileManControlPanel): stub module for sort/filter/theme UI"
```

---

## Task 19: FpPlugin Entry Points

**Files:**
- Create: `crates/emfileman/src/emDirFpPlugin.rs`
- Create: `crates/emfileman/src/emDirStatFpPlugin.rs`
- Create: `crates/emfileman/src/emFileLinkFpPlugin.rs`
- Modify: `crates/emfileman/src/lib.rs`

Three `#[no_mangle]` entry points matching `emFpPluginFunc` signature.

- [ ] **Step 1: Create emDirFpPlugin.rs**

```rust
use emcore::emFpPlugin::{emFpPlugin, PanelParentArg};
use emcore::emPanel::PanelBehavior;
use std::cell::RefCell;
use std::rc::Rc;

/// Entry point for the directory panel plugin.
/// Loaded via `emDir.emFpPlugin` config file.
#[no_mangle]
pub extern "C" fn emDirFpPluginFunc(
    parent: &PanelParentArg,
    name: &str,
    path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Rc<RefCell<dyn PanelBehavior>>> {
    if !plugin.properties.is_empty() {
        *error_buf = "emDirFpPlugin: No properties allowed.".to_string();
        return None;
    }
    // TODO: return new emDirPanel when panel integration is complete
    *error_buf = "emDirFpPlugin: not yet implemented".to_string();
    None
}
```

- [ ] **Step 2: Create emDirStatFpPlugin.rs**

```rust
#[no_mangle]
pub extern "C" fn emDirStatFpPluginFunc(
    parent: &PanelParentArg,
    name: &str,
    path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Rc<RefCell<dyn PanelBehavior>>> {
    if !plugin.properties.is_empty() {
        *error_buf = "emDirStatFpPlugin: No properties allowed.".to_string();
        return None;
    }
    // TODO: create emDirStatPanel with emDirModel, updateFileModel=false
    *error_buf = "emDirStatFpPlugin: not yet implemented".to_string();
    None
}
```

- [ ] **Step 3: Create emFileLinkFpPlugin.rs**

```rust
#[no_mangle]
pub extern "C" fn emFileLinkFpPluginFunc(
    parent: &PanelParentArg,
    name: &str,
    path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Rc<RefCell<dyn PanelBehavior>>> {
    if !plugin.properties.is_empty() {
        *error_buf = "emFileLinkFpPlugin: No properties allowed.".to_string();
        return None;
    }
    // TODO: create emFileLinkPanel with emFileLinkModel
    *error_buf = "emFileLinkFpPlugin: not yet implemented".to_string();
    None
}
```

- [ ] **Step 4: Add modules to lib.rs**

```rust
pub mod emDirFpPlugin;
pub mod emDirStatFpPlugin;
pub mod emFileLinkFpPlugin;
```

- [ ] **Step 5: Verify cdylib builds**

Run: `cargo build -p emfileman`
Expected: produces `target/debug/libemFileMan.so` (or `.dylib`).

Run: `nm -D target/debug/libemFileMan.so | grep FpPlugin`
Expected: shows `emDirFpPluginFunc`, `emDirStatFpPluginFunc`, `emFileLinkFpPluginFunc`.

- [ ] **Step 6: Commit**

```bash
git add crates/emfileman/src/emDirFpPlugin.rs crates/emfileman/src/emDirStatFpPlugin.rs crates/emfileman/src/emFileLinkFpPlugin.rs crates/emfileman/src/lib.rs
git commit -m "feat(emFileMan): FpPlugin entry points for directory, dirstat, and filelink panels"
```

---

## Task 20: .emFpPlugin Config Files

**Files:**
- Create: `etc/emCore/FpPlugins/emDir.emFpPlugin`
- Create: `etc/emCore/FpPlugins/emDirStat.emFpPlugin`
- Create: `etc/emCore/FpPlugins/emFileLink.emFpPlugin`

- [ ] **Step 1: Check if etc/emCore/FpPlugins/ exists**

Run: `ls etc/emCore/FpPlugins/`

- [ ] **Step 2: Create config files**

`emDir.emFpPlugin`:
```
#%rec:emFpPlugin%#
FileTypes = { "directory" }
Library = "emFileMan"
Function = "emDirFpPluginFunc"
Priority = 1.0
```

`emDirStat.emFpPlugin`:
```
#%rec:emFpPlugin%#
FileTypes = { "directory" }
Library = "emFileMan"
Function = "emDirStatFpPluginFunc"
Priority = 0.1
```

`emFileLink.emFpPlugin`:
```
#%rec:emFpPlugin%#
FileTypes = { ".emFileLink" }
Library = "emFileMan"
Function = "emFileLinkFpPluginFunc"
Priority = 1.0
```

- [ ] **Step 3: Commit**

```bash
git add etc/emCore/FpPlugins/emDir.emFpPlugin etc/emCore/FpPlugins/emDirStat.emFpPlugin etc/emCore/FpPlugins/emFileLink.emFpPlugin
git commit -m "feat(emFileMan): .emFpPlugin config files for directory/dirstat/filelink plugins"
```

---

## Task 21: Full Build Verification

- [ ] **Step 1: Run full workspace build**

Run: `cargo build --workspace`
Expected: all crates compile, `libemFileMan.so` produced.

- [ ] **Step 2: Run all tests**

Run: `cargo-nextest ntr`
Expected: all tests pass across all crates.

- [ ] **Step 3: Run clippy on emfileman**

Run: `cargo clippy -p emfileman -- -D warnings`
Expected: no warnings.

- [ ] **Step 4: Verify exported symbols**

Run: `nm -D target/debug/libemFileMan.so | grep -E '(emDirFpPlugin|emDirStatFpPlugin|emFileLinkFpPlugin)Func'`
Expected: 3 symbols found.

---

## Notes for Implementer

### Dependencies not yet in emcore
Some C++ features used by emFileMan may not yet exist in emcore. If you encounter missing APIs:
- `emTryOpenDir` / `emTryReadDir` / `emCloseDir` → use `libc::opendir`/`readdir`/`closedir` directly
- `emGetExtensionInPath` → use `Path::extension()` or manual string split
- `emGetNameInPath` → use `Path::file_name()`
- `emGetChildPath` → use `Path::join()`
- `emGetParentPath` → use `Path::parent()`
- `emGetAbsolutePath` → use `std::fs::canonicalize` or manual resolution
- `emIsExistingPath` → use `Path::exists()`
- `emIsRegularFile` → use `Path::is_file()`
- `emIsDirectory` → use `Path::is_dir()`
- `emCalcHashCode` for strings → call `emCalcHashCode(s.as_bytes(), 0)` from emcore

### Panel integration
Tasks 12-18 focus on data structures and algorithms. Full panel rendering (Paint methods, LayoutChildren, Notice/Cycle integration) requires wiring into the emcore panel tree, which depends on the panel subsystem's current state. The FpPlugin entry points (Task 19) are stubs that return `None` until panels are connected. A follow-up plan should integrate these data types into the panel tree.

### Ordering
Tasks 1-11 are the critical path with full test coverage. Tasks 12-18 can be parallelized (they depend only on Tasks 1-2 for emDirEntry). Tasks 19-20 are thin wrappers. Task 21 is the gate.
