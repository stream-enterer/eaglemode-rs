# Phase 1: emCore Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close all API surface and behavioral gaps in emCore foundation types — collections, emColor, emATMatrix, emCursor, emButton, emTmpFile.

**Architecture:** Bottom-up: fix emColor scale factors first (breaking change touching ~40 callers), then add missing methods to collections, then small API additions to individual types. TDD throughout — tests before implementation.

**Tech Stack:** Rust, cargo-nextest, emcore crate

---

## Task 1: emColor GetBlended Scale Change [0,1] → [0,100]

**Files:**
- Modify: `crates/emcore/src/emColor.rs:253-270`
- Modify: `crates/emcore/src/emBorder.rs` (4 call sites)
- Modify: `crates/emcore/src/emFileSelectionBox.rs` (3 call sites)
- Modify: `crates/emcore/src/emListBox.rs` (6 call sites)
- Modify: `crates/emcore/src/emLook.rs` (1 call site)
- Modify: `crates/emcore/src/emScalarField.rs` (4 call sites)
- Modify: `crates/emcore/src/emTextField.rs` (8 call sites)
- Modify: `crates/emcore/src/emPainter.rs` (4 call sites)
- Modify: `crates/emcore/src/emPainterInterpolation.rs` (1 call site)
- Modify: `crates/emfileman/src/emFileManSelInfoPanel.rs` (1 call site)
- Test: `crates/eaglemode/tests/behavioral/color.rs` (existing) or inline tests in emColor.rs

- [ ] **Step 1: Update GetBlended to accept [0, 100] range**

In `crates/emcore/src/emColor.rs`, replace the GetBlended method:

```rust
    /// Linearly interpolate between `self` and `other` by weight `weight` (0.0–100.0).
    ///
    /// Matches C++ `emColor::GetBlended(color, weight)` with 16-bit precision:
    /// `w2 = (int)(weight * 655.36 + 0.5)`, `result = (a*w1 + b*w2 + 32768) >> 16`.
    pub fn GetBlended(self, other: emColor, weight: f64) -> emColor {
        if weight <= 0.0 {
            return self;
        }
        if weight >= 100.0 {
            return other;
        }
        let w2 = (weight * 655.36 + 0.5) as i32;
        let w1 = 65536 - w2;
        let mix = |a: i32, b: i32| -> u8 { ((a * w1 + b * w2 + 32768) >> 16) as u8 };
        emColor::rgba(
            mix(self.GetRed() as i32, other.GetRed() as i32),
            mix(self.GetGreen() as i32, other.GetGreen() as i32),
            mix(self.GetBlue() as i32, other.GetBlue() as i32),
            mix(self.GetAlpha() as i32, other.GetAlpha() as i32),
        )
    }
```

Remove the DIVERGED comment on line 258.

- [ ] **Step 2: Update inline test to use [0, 100] scale**

In `crates/emcore/src/emColor.rs`, update `test_lerp_interpolates_alpha`:

```rust
    #[test]
    fn test_lerp_interpolates_alpha() {
        let a = emColor::rgba(0, 0, 0, 0);
        let b = emColor::rgba(255, 255, 255, 255);
        let result = a.GetBlended(b, 50.0);

        // C++ formula: w2 = (50.0 * 655.36 + 0.5) as i32 = 32768
        // mix(0, 255) = (0 * (65536-32768) + 255 * 32768 + 32768) >> 16
        //             = (8355840 + 32768) >> 16 = 8388608 >> 16 = 128
        assert_eq!(result.GetAlpha(), 128, "lerp alpha at weight=50: got {} expected 128", result.GetAlpha());
        assert_eq!(result.GetRed(), result.GetAlpha());
        assert_eq!(result.GetGreen(), result.GetAlpha());
        assert_eq!(result.GetBlue(), result.GetAlpha());

        // Verify endpoints
        let at_zero = a.GetBlended(b, 0.0);
        assert_eq!(at_zero.GetAlpha(), 0, "lerp alpha at weight=0 should be 0");
        let at_hundred = a.GetBlended(b, 100.0);
        assert_eq!(at_hundred.GetAlpha(), 255, "lerp alpha at weight=100 should be 255");
    }
```

- [ ] **Step 3: Update all callers — multiply by 100**

Apply these mechanical replacements across all files. The pattern is: every literal `f64` argument to `GetBlended` that was in [0,1] becomes [0,100].

**crates/emcore/src/emBorder.rs** — 4 sites, all `0.80` → `80.0`:
```
.GetBlended(look.bg_color, 0.80)  →  .GetBlended(look.bg_color, 80.0)
```

**crates/emcore/src/emFileSelectionBox.rs** — 3 sites, all `0.80` → `80.0`:
```
bg.GetBlended(base, 0.80)  →  bg.GetBlended(base, 80.0)
fg.GetBlended(base, 0.80)  →  fg.GetBlended(base, 80.0)
hl.GetBlended(base, 0.80)  →  hl.GetBlended(base, 80.0)
```

**crates/emcore/src/emListBox.rs** — 6 sites, all `0.80` → `80.0`:
```
.GetBlended(base, 0.80)  →  .GetBlended(base, 80.0)
```

**crates/emcore/src/emLook.rs** — 1 site, `0.5` → `50.0`:
```
self.fg_color.GetBlended(self.bg_color, 0.5)  →  self.fg_color.GetBlended(self.bg_color, 50.0)
```

**crates/emcore/src/emScalarField.rs** — 4 sites:
```
.GetBlended(self.look.bg_color, 0.80)  →  .GetBlended(self.look.bg_color, 80.0)
.GetBlended(fg_col, 0.25)              →  .GetBlended(fg_col, 25.0)
.GetBlended(fg_col, 0.66)              →  .GetBlended(fg_col, 66.0)
```

**crates/emcore/src/emTextField.rs** — 8 sites:
```
.GetBlended(base, 0.8)   →  .GetBlended(base, 80.0)
.GetBlended(fg, 0.4)     →  .GetBlended(fg, 40.0)
```

**crates/emcore/src/emPainter.rs** — 4 sites (dynamic values need formula change):
```
color1.GetBlended(color2, t)                              →  color1.GetBlended(color2, t * 100.0)
color_a.GetBlended(*color_b, t.clamp(0.0, 1.0))          →  color_a.GetBlended(*color_b, (t * 100.0).clamp(0.0, 100.0))
color_outer.GetBlended(*color_inner, 0.0)                 →  color_outer.GetBlended(*color_inner, 0.0)  // (no change — 0 is same in both scales)
color_inner.GetBlended(*color_outer, factor as f64 / 255.0)  →  color_inner.GetBlended(*color_outer, factor as f64 * 100.0 / 255.0)
```

**crates/emcore/src/emPainterInterpolation.rs** — 1 site:
```
c0.GetBlended(c1, t.clamp(0.0, 1.0))  →  c0.GetBlended(c1, (t * 100.0).clamp(0.0, 100.0))
```

**crates/emfileman/src/emFileManSelInfoPanel.rs** — 1 site:
```
color.GetBlended(blend_color, 0.5)  →  color.GetBlended(blend_color, 50.0)
```

- [ ] **Step 4: Run tests to verify no regressions**

Run: `cargo-nextest ntr`
Expected: All tests PASS. Golden tests will catch any rendering regressions.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "fix(emColor): restore GetBlended to C++ [0,100] scale

Change GetBlended parameter from normalized [0,1] to C++ percent
[0,100] range. Update all ~40 callers across emBorder, emListBox,
emScalarField, emTextField, emPainter, emFileSelectionBox, emLook,
emPainterInterpolation, and emFileManSelInfoPanel."
```

---

## Task 2: emColor GetLighted Restoration

**Files:**
- Modify: `crates/emcore/src/emColor.rs:220-231`
- Modify: `crates/emcore/src/emLook.rs` (3 callers of lighten/darken)
- Modify: `crates/eaglemode/tests/kani/proofs_generated.rs` (2 callers)

- [ ] **Step 1: Write test for GetLighted**

Add to the test module in `crates/emcore/src/emColor.rs`:

```rust
    #[test]
    fn test_get_lighted() {
        let c = emColor::rgb(100, 100, 100);

        // Positive = lighten (blend toward white)
        let lighter = c.GetLighted(50.0);
        // GetLighted(50) = GetBlended(WHITE, 50.0)
        let expected = c.GetBlended(emColor::WHITE.SetAlpha(c.GetAlpha()), 50.0);
        assert_eq!(lighter.GetPacked(), expected.GetPacked());

        // Negative = darken (blend toward black)
        let darker = c.GetLighted(-50.0);
        let expected = c.GetBlended(emColor::rgba(0, 0, 0, c.GetAlpha()), 50.0);
        assert_eq!(darker.GetPacked(), expected.GetPacked());

        // Zero = no change
        let same = c.GetLighted(0.0);
        assert_eq!(same.GetPacked(), c.GetPacked());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emcore --lib emColor -- test_get_lighted`
Expected: FAIL — `GetLighted` method doesn't exist yet

- [ ] **Step 3: Replace lighten/darken with GetLighted**

In `crates/emcore/src/emColor.rs`, replace lines 220-231:

```rust
    /// Lighten or darken the color.
    /// `light` in [-100.0, 100.0]: positive blends toward white, negative toward black.
    /// Matches C++ `emColor::GetLighted(float light)`.
    pub fn GetLighted(self, light: f32) -> emColor {
        if light <= 0.0 {
            self.GetBlended(emColor::rgba(0, 0, 0, self.GetAlpha()), (-light) as f64)
        } else {
            self.GetBlended(
                emColor::rgba(255, 255, 255, self.GetAlpha()),
                light as f64,
            )
        }
    }
```

- [ ] **Step 4: Update callers**

**crates/emcore/src/emLook.rs:**
```
self.bg_color.darken(0.20)         →  self.bg_color.GetLighted(-20.0)
self.button_bg_color.lighten(0.15) →  self.button_bg_color.GetLighted(15.0)
self.button_bg_color.darken(0.15)  →  self.button_bg_color.GetLighted(-15.0)
```

**crates/eaglemode/tests/kani/proofs_generated.rs:**
```
self_val.lighten(p_amount)  →  self_val.GetLighted(p_amount as f32 * 100.0)
self_val.darken(p_amount)   →  self_val.GetLighted(-(p_amount as f32 * 100.0))
```

- [ ] **Step 5: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "fix(emColor): restore GetLighted with C++ [-100,100] range

Replace lighten()/darken() with unified GetLighted(light) matching
C++ emColor::GetLighted. Positive lightens, negative darkens."
```

---

## Task 3: emColor Individual HSV Methods + SetHSVA 4-param + SetGrey 2-param

**Files:**
- Modify: `crates/emcore/src/emColor.rs`

- [ ] **Step 1: Write tests for individual HSV methods**

```rust
    #[test]
    fn test_individual_hsv_accessors() {
        let c = emColor::rgb(255, 0, 0); // pure red
        assert!((c.GetHue() - 0.0).abs() < 1.0);
        assert!((c.GetSat() - 100.0).abs() < 1.0);
        assert!((c.GetVal() - 100.0).abs() < 1.0);

        // Verify consistency with GetHSV tuple
        let (h, s, v) = c.GetHSV();
        assert_eq!(c.GetHue(), h);
        assert_eq!(c.GetSat(), s);
        assert_eq!(c.GetVal(), v);
    }

    #[test]
    fn test_set_hsva_4_param() {
        let c = emColor::SetHSVA_with_alpha(0.0, 100.0, 100.0, 128);
        assert_eq!(c.GetAlpha(), 128);
        assert_eq!(c.GetRed(), 255);
    }

    #[test]
    fn test_set_grey_2_param() {
        let c = emColor::SetGrey_with_alpha(128, 200);
        assert_eq!(c.GetRed(), 128);
        assert_eq!(c.GetGreen(), 128);
        assert_eq!(c.GetBlue(), 128);
        assert_eq!(c.GetAlpha(), 200);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p emcore --lib emColor -- test_individual_hsv`
Expected: FAIL

- [ ] **Step 3: Add individual HSV accessors**

Add after `GetHSV()` in `crates/emcore/src/emColor.rs`:

```rust
    /// Get the hue component. Returns [0.0, 360.0).
    /// C++ `emColor::GetHue()`.
    pub fn GetHue(self) -> f32 {
        self.GetHSV().0
    }

    /// Get the saturation component. Returns [0.0, 100.0].
    /// C++ `emColor::GetSat()`.
    pub fn GetSat(self) -> f32 {
        self.GetHSV().1
    }

    /// Get the value (brightness) component. Returns [0.0, 100.0].
    /// C++ `emColor::GetVal()`.
    pub fn GetVal(self) -> f32 {
        self.GetHSV().2
    }
```

Remove DIVERGED comment on line 163.

- [ ] **Step 4: Add SetHSVA 4-param and SetGrey 2-param**

Add after `SetHSVA(h, s, v)`:

```rust
    /// Create a color from HSV + alpha. C++ `emColor::SetHSVA(h, s, v, alpha)`.
    pub fn SetHSVA_with_alpha(h: f32, s: f32, v: f32, alpha: u8) -> Self {
        Self::SetHSVA(h, s, v).SetAlpha(alpha)
    }
```

Add after `SetGrey(val)`:

```rust
    /// Construct a grey color with explicit alpha. C++ `emColor::SetGrey(val, alpha)`.
    #[inline]
    pub const fn SetGrey_with_alpha(val: u8, alpha: u8) -> emColor {
        emColor::rgba(val, val, val, alpha)
    }
```

Update existing `SetHSVA` DIVERGED comment to note the 4-param variant exists.
Update existing `SetGrey` DIVERGED comment to note the 2-param variant exists.

- [ ] **Step 5: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emColor.rs && git commit -m "feat(emColor): add individual GetHue/GetSat/GetVal and SetHSVA/SetGrey with alpha"
```

---

## Task 4: emATMatrix Individual Trans Methods

**Files:**
- Modify: `crates/emcore/src/emATMatrix.rs:287-314`

- [ ] **Step 1: Write tests**

Add to the test module in `crates/emcore/src/emATMatrix.rs`:

```rust
    #[test]
    fn test_individual_trans_methods() {
        // Identity + translation (10, 20)
        let m = emATMatrix::new_translate(10.0, 20.0);
        let (tx, ty) = m.transform_point(5.0, 7.0);
        assert_eq!(m.TransX(5.0, 7.0), tx);
        assert_eq!(m.TransY(5.0, 7.0), ty);
    }

    #[test]
    fn test_individual_inverse_trans_methods() {
        let m = emATMatrix::new_translate(10.0, 20.0);
        let (ix, iy) = m.inverse_transform_point(15.0, 27.0).unwrap();
        assert_eq!(m.InverseTransX(15.0, 27.0).unwrap(), ix);
        assert_eq!(m.InverseTransY(15.0, 27.0).unwrap(), iy);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p emcore --lib emATMatrix -- test_individual_trans`
Expected: FAIL

- [ ] **Step 3: Add individual methods**

Add after `transform_point` in `crates/emcore/src/emATMatrix.rs`:

```rust
    /// Transform source X coordinate. C++ `emATMatrix::TransX`.
    pub fn TransX(&self, sx: f64, sy: f64) -> f64 {
        self.a[0][0] * sx + self.a[1][0] * sy + self.a[2][0]
    }

    /// Transform source Y coordinate. C++ `emATMatrix::TransY`.
    pub fn TransY(&self, sx: f64, sy: f64) -> f64 {
        self.a[0][1] * sx + self.a[1][1] * sy + self.a[2][1]
    }
```

Add after `inverse_transform_point`:

```rust
    /// Inverse-transform target X coordinate. C++ `emATMatrix::InverseTransX`.
    pub fn InverseTransX(&self, tx: f64, ty: f64) -> Option<f64> {
        self.inverse_transform_point(tx, ty).map(|(sx, _)| sx)
    }

    /// Inverse-transform target Y coordinate. C++ `emATMatrix::InverseTransY`.
    pub fn InverseTransY(&self, tx: f64, ty: f64) -> Option<f64> {
        self.inverse_transform_point(tx, ty).map(|(_, sy)| sy)
    }
```

Update DIVERGED comments on lines 287 and 296: keep `transform_point` and `inverse_transform_point` as primary methods, note individual methods exist for C++ API correspondence.

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emATMatrix.rs && git commit -m "feat(emATMatrix): add individual TransX/TransY/InverseTransX/InverseTransY"
```

---

## Task 5: emArray Custom Comparator Methods

**Files:**
- Modify: `crates/emcore/src/emArray.rs`
- Test: `crates/eaglemode/tests/behavioral/array.rs`

- [ ] **Step 1: Write tests for _by methods**

Add to `crates/eaglemode/tests/behavioral/array.rs`:

```rust
#[test]
fn test_sort_by_custom_comparator() {
    let mut arr = emArray::new();
    arr.Add_one(3);
    arr.Add_one(1);
    arr.Add_one(2);
    // Sort descending
    arr.Sort_by(|a, b| b.cmp(a));
    assert_eq!(arr.Get_at(0), &3);
    assert_eq!(arr.Get_at(1), &2);
    assert_eq!(arr.Get_at(2), &1);
}

#[test]
fn test_binary_search_by() {
    let mut arr = emArray::new();
    for i in [1, 3, 5, 7, 9] {
        arr.Add_one(i);
    }
    assert_eq!(arr.BinarySearch_by(|x| x.cmp(&5)), Ok(2));
    assert_eq!(arr.BinarySearch_by(|x| x.cmp(&4)), Err(2));
}

#[test]
fn test_binary_insert_by() {
    let mut arr = emArray::new();
    // Insert in reverse order
    arr.BinaryInsert_by(3, |a, b| b.cmp(a));
    arr.BinaryInsert_by(1, |a, b| b.cmp(a));
    arr.BinaryInsert_by(2, |a, b| b.cmp(a));
    assert_eq!(arr.Get_at(0), &3);
    assert_eq!(arr.Get_at(1), &2);
    assert_eq!(arr.Get_at(2), &1);
}

#[test]
fn test_binary_search_by_key() {
    let mut arr: emArray<(i32, &str)> = emArray::new();
    arr.Add_one((1, "one"));
    arr.Add_one((3, "three"));
    arr.Add_one((5, "five"));
    assert_eq!(arr.BinarySearchByKey(&3, |item| item.0), Ok(1));
    assert_eq!(arr.BinarySearchByKey(&2, |item| item.0), Err(1));
}

#[test]
fn test_binary_replace() {
    let mut arr = emArray::new();
    arr.Add_one(1);
    arr.Add_one(3);
    arr.Add_one(5);
    assert!(arr.BinaryReplace(3, |a, b| a.cmp(b)));
    assert!(!arr.BinaryReplace(4, |a, b| a.cmp(b)));
}

#[test]
fn test_binary_remove_by_key() {
    let mut arr: emArray<(i32, &str)> = emArray::new();
    arr.Add_one((1, "one"));
    arr.Add_one((3, "three"));
    arr.Add_one((5, "five"));
    assert!(arr.BinaryRemoveByKey(&3, |item| item.0));
    assert_eq!(arr.GetCount(), 2);
    assert!(!arr.BinaryRemoveByKey(&4, |item| item.0));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test behavioral -- test_sort_by_custom`
Expected: FAIL — methods don't exist

- [ ] **Step 3: Implement _by methods**

Add new impl block in `crates/emcore/src/emArray.rs` after the `impl<T: Clone + Ord>` block:

```rust
impl<T: Clone> emArray<T> {
    /// Sort with a custom comparator. Returns `true` if order changed.
    /// C++ `emArray::Sort(int(*)(const OBJ*,const OBJ*,void*), void*)`.
    pub fn Sort_by(&mut self, compare: impl FnMut(&T, &T) -> std::cmp::Ordering) {
        self.make_writable().sort_by(compare);
    }

    /// Binary search with a custom comparator.
    /// C++ custom comparator overload of `BinarySearch`.
    pub fn BinarySearch_by(&self, compare: impl FnMut(&T) -> std::cmp::Ordering) -> Result<usize, usize> {
        self.data.binary_search_by(compare)
    }

    /// Insert maintaining order defined by `compare`.
    pub fn BinaryInsert_by(&mut self, obj: T, mut compare: impl FnMut(&T, &T) -> std::cmp::Ordering) {
        let pos = match self.data.binary_search_by(|probe| compare(probe, &obj)) {
            Ok(i) | Err(i) => i,
        };
        self.make_writable().insert(pos, obj);
    }

    /// Insert only if no equal element exists (per `compare`). Returns `true` if inserted.
    pub fn BinaryInsertIfNew_by(&mut self, obj: T, mut compare: impl FnMut(&T, &T) -> std::cmp::Ordering) -> bool {
        match self.data.binary_search_by(|probe| compare(probe, &obj)) {
            Ok(_) => false,
            Err(i) => {
                self.make_writable().insert(i, obj);
                true
            }
        }
    }

    /// Insert or replace the matching element (per `compare`).
    pub fn BinaryInsertOrReplace_by(&mut self, obj: T, mut compare: impl FnMut(&T, &T) -> std::cmp::Ordering) {
        match self.data.binary_search_by(|probe| compare(probe, &obj)) {
            Ok(i) => {
                self.make_writable()[i] = obj;
            }
            Err(i) => {
                self.make_writable().insert(i, obj);
            }
        }
    }

    /// Remove the matching element (per `compare`). Returns `true` if found.
    pub fn BinaryRemove_by(&mut self, compare: impl FnMut(&T) -> std::cmp::Ordering) -> bool {
        match self.data.binary_search_by(compare) {
            Ok(i) => {
                self.make_writable().remove(i);
                true
            }
            Err(_) => false,
        }
    }

    /// Binary search by extracted key.
    /// C++ `BinarySearchByKey`.
    pub fn BinarySearchByKey<K: Ord>(&self, key: &K, extract: impl Fn(&T) -> K) -> Result<usize, usize> {
        self.data.binary_search_by(|probe| extract(probe).cmp(key))
    }

    /// Find and replace the matching element (per `compare`). Returns `true` if found.
    /// C++ `BinaryReplace`.
    pub fn BinaryReplace(&mut self, obj: T, mut compare: impl FnMut(&T, &T) -> std::cmp::Ordering) -> bool {
        match self.data.binary_search_by(|probe| compare(probe, &obj)) {
            Ok(i) => {
                self.make_writable()[i] = obj;
                true
            }
            Err(_) => false,
        }
    }

    /// Remove by extracted key. Returns `true` if found.
    /// C++ `BinaryRemoveByKey`.
    pub fn BinaryRemoveByKey<K: Ord>(&mut self, key: &K, extract: impl Fn(&T) -> K) -> bool {
        match self.data.binary_search_by(|probe| extract(probe).cmp(key)) {
            Ok(i) => {
                self.make_writable().remove(i);
                true
            }
            Err(_) => false,
        }
    }
}
```

Update DIVERGED comments at top of file to remove mentions of `BinarySearchByKey`, `BinaryReplace`, `BinaryRemoveByKey`, and `custom comparator overloads`.

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emArray.rs crates/eaglemode/tests/behavioral/array.rs && git commit -m "feat(emArray): add custom comparator _by methods and key-based search/remove"
```

---

## Task 6: emArray Default-Insertion Methods + TuningLevel

**Files:**
- Modify: `crates/emcore/src/emArray.rs`
- Test: `crates/eaglemode/tests/behavioral/array.rs`

- [ ] **Step 1: Write tests**

```rust
#[test]
fn test_add_new() {
    let mut arr: emArray<i32> = emArray::new();
    arr.AddNew();
    assert_eq!(arr.GetCount(), 1);
    assert_eq!(arr.Get_at(0), &0); // i32::default() = 0
}

#[test]
fn test_insert_new() {
    let mut arr: emArray<i32> = emArray::new();
    arr.Add_one(10);
    arr.Add_one(30);
    arr.InsertNew(1);
    assert_eq!(arr.GetCount(), 3);
    assert_eq!(arr.Get_at(1), &0);
    assert_eq!(arr.Get_at(2), &30);
}

#[test]
fn test_replace_by_new() {
    let mut arr: emArray<i32> = emArray::new();
    arr.Add_one(10);
    arr.Add_one(20);
    arr.Add_one(30);
    arr.ReplaceByNew(1, 2);
    assert_eq!(arr.GetCount(), 2);
    assert_eq!(arr.Get_at(0), &10);
    assert_eq!(arr.Get_at(1), &0);
}

#[test]
fn test_tuning_level() {
    let mut arr: emArray<i32> = emArray::new();
    assert_eq!(arr.GetTuningLevel(), 0);
    arr.SetTuningLevel(4);
    assert_eq!(arr.GetTuningLevel(), 4);
    // TuningLevel has no effect on behavior — just stored for API correspondence
    arr.Add_one(42);
    assert_eq!(arr.Get_at(0), &42);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test behavioral -- test_add_new`
Expected: FAIL

- [ ] **Step 3: Implement**

Add `tuning_level` field to the `emArray` struct:

```rust
pub struct emArray<T: Clone> {
    data: Rc<Vec<T>>,
    /// C++ TuningLevel — stored for API correspondence, no effect on behavior.
    /// DIVERGED: Rust ownership model makes COW tuning unnecessary; field
    /// exists for API correspondence only.
    tuning_level: u8,
}
```

Update `new()`, `Clone`, and other constructors to initialize `tuning_level: 0`.

Add methods in the `impl<T: Clone> emArray<T>` block:

```rust
    /// Get the tuning level. C++ `GetTuningLevel`.
    pub fn GetTuningLevel(&self) -> u8 {
        self.tuning_level
    }

    /// Set the tuning level. No effect on behavior. C++ `SetTuningLevel`.
    pub fn SetTuningLevel(&mut self, level: u8) {
        self.tuning_level = level;
    }
```

Add in a `impl<T: Clone + Default> emArray<T>` block:

```rust
impl<T: Clone + Default> emArray<T> {
    /// Append a default-constructed element. C++ `AddNew`.
    pub fn AddNew(&mut self) {
        self.make_writable().push(T::default());
    }

    /// Insert a default-constructed element at `index`. C++ `InsertNew`.
    pub fn InsertNew(&mut self, index: usize) {
        self.make_writable().insert(index, T::default());
    }

    /// Replace `count` elements starting at `index` with one default element.
    /// C++ `ReplaceByNew`.
    pub fn ReplaceByNew(&mut self, index: usize, count: usize) {
        let v = self.make_writable();
        v.drain(index..index + count);
        v.insert(index, T::default());
    }
}
```

Update DIVERGED comments at top of file to remove mentions of `AddNew`, `InsertNew`, `ReplaceByNew`, and `TuningLevel`.

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emArray.rs crates/eaglemode/tests/behavioral/array.rs && git commit -m "feat(emArray): add AddNew/InsertNew/ReplaceByNew and TuningLevel"
```

---

## Task 7: emList Method Additions

**Files:**
- Modify: `crates/emcore/src/emList.rs`
- Test: `crates/eaglemode/tests/behavioral/list.rs`

- [ ] **Step 1: Write tests for mutable navigation**

```rust
#[test]
fn test_get_next_writable() {
    let mut list = emList::from_element(10);
    list.Add_one(20);
    list.Add_one(30);
    let (idx, val) = list.GetNextWritable(0).unwrap();
    assert_eq!(idx, 1);
    *val = 99;
    assert_eq!(list.GetAtIndex(1), Some(&99));
    assert!(list.GetNextWritable(2).is_none());
}

#[test]
fn test_get_prev_writable() {
    let mut list = emList::from_element(10);
    list.Add_one(20);
    let (idx, val) = list.GetPrevWritable(1).unwrap();
    assert_eq!(idx, 0);
    *val = 99;
    assert_eq!(list.GetAtIndex(0), Some(&99));
    assert!(list.GetPrevWritable(0).is_none());
}
```

- [ ] **Step 2: Write tests for move operations**

```rust
#[test]
fn test_move_to_beg() {
    let mut list = emList::new();
    list.Add_one(1); list.Add_one(2); list.Add_one(3);
    list.MoveToBeg(2); // move 3 to front
    assert_eq!(list.GetAtIndex(0), Some(&3));
    assert_eq!(list.GetAtIndex(1), Some(&1));
    assert_eq!(list.GetAtIndex(2), Some(&2));
}

#[test]
fn test_move_to_end() {
    let mut list = emList::new();
    list.Add_one(1); list.Add_one(2); list.Add_one(3);
    list.MoveToEnd(0); // move 1 to end
    assert_eq!(list.GetAtIndex(0), Some(&2));
    assert_eq!(list.GetAtIndex(1), Some(&3));
    assert_eq!(list.GetAtIndex(2), Some(&1));
}

#[test]
fn test_move_before() {
    let mut list = emList::new();
    list.Add_one(1); list.Add_one(2); list.Add_one(3);
    list.MoveBefore(2, 1); // move 3 before 2
    assert_eq!(list.GetAtIndex(0), Some(&1));
    assert_eq!(list.GetAtIndex(1), Some(&3));
    assert_eq!(list.GetAtIndex(2), Some(&2));
}

#[test]
fn test_move_after() {
    let mut list = emList::new();
    list.Add_one(1); list.Add_one(2); list.Add_one(3);
    list.MoveAfter(0, 1); // move 1 after 2
    assert_eq!(list.GetAtIndex(0), Some(&2));
    assert_eq!(list.GetAtIndex(1), Some(&1));
    assert_eq!(list.GetAtIndex(2), Some(&3));
}
```

- [ ] **Step 3: Write tests for sublist operations**

```rust
#[test]
fn test_get_sub_list() {
    let mut list = emList::new();
    for i in 0..5 { list.Add_one(i); }
    let sub = list.GetSubList(1, 3);
    assert_eq!(sub.GetCount(), 3);
    assert_eq!(sub.GetAtIndex(0), Some(&1));
    assert_eq!(sub.GetAtIndex(2), Some(&3));
}

#[test]
fn test_get_sub_list_of_first() {
    let mut list = emList::new();
    for i in 0..5 { list.Add_one(i); }
    let sub = list.GetSubListOfFirst(2);
    assert_eq!(sub.GetCount(), 2);
    assert_eq!(sub.GetAtIndex(0), Some(&0));
    assert_eq!(sub.GetAtIndex(1), Some(&1));
}

#[test]
fn test_get_sub_list_of_last() {
    let mut list = emList::new();
    for i in 0..5 { list.Add_one(i); }
    let sub = list.GetSubListOfLast(2);
    assert_eq!(sub.GetCount(), 2);
    assert_eq!(sub.GetAtIndex(0), Some(&3));
    assert_eq!(sub.GetAtIndex(1), Some(&4));
}

#[test]
fn test_extract() {
    let mut list = emList::new();
    for i in 0..5 { list.Add_one(i); }
    let extracted = list.Extract(1, 3);
    assert_eq!(extracted.GetCount(), 3);
    assert_eq!(list.GetCount(), 2);
    assert_eq!(list.GetAtIndex(0), Some(&0));
    assert_eq!(list.GetAtIndex(1), Some(&4));
}
```

- [ ] **Step 4: Write tests for multi-variant insertion and constructors**

```rust
#[test]
fn test_insert_at_beg_slice() {
    let mut list = emList::from_element(3);
    list.InsertAtBeg_slice(&[1, 2]);
    assert_eq!(list.GetCount(), 3);
    assert_eq!(list.GetAtIndex(0), Some(&1));
    assert_eq!(list.GetAtIndex(2), Some(&3));
}

#[test]
fn test_insert_at_end_fill() {
    let mut list = emList::new();
    list.InsertAtEnd_fill(42, 3);
    assert_eq!(list.GetCount(), 3);
    assert_eq!(list.GetAtIndex(2), Some(&42));
}

#[test]
fn test_sort_by() {
    let mut list = emList::new();
    list.Add_one(3); list.Add_one(1); list.Add_one(2);
    list.Sort_by(|a, b| b.cmp(a)); // descending
    assert_eq!(list.GetAtIndex(0), Some(&3));
    assert_eq!(list.GetAtIndex(2), Some(&1));
}

#[test]
fn test_from_two() {
    let mut a = emList::new();
    a.Add_one(1); a.Add_one(2);
    let mut b = emList::new();
    b.Add_one(3); b.Add_one(4);
    let merged = emList::from_two(&a, &b);
    assert_eq!(merged.GetCount(), 4);
    assert_eq!(merged.GetAtIndex(0), Some(&1));
    assert_eq!(merged.GetAtIndex(3), Some(&4));
}
```

- [ ] **Step 5: Run all list tests to verify they fail**

Run: `cargo test --test behavioral -- list`
Expected: FAIL — methods don't exist

- [ ] **Step 6: Implement all methods**

Add to `impl<T: Clone> emList<T>` in `crates/emcore/src/emList.rs`:

```rust
    // --- Mutable navigation ---

    /// Get mutable reference to the next element after `index`.
    pub fn GetNextWritable(&mut self, index: usize) -> Option<(usize, &mut T)> {
        let next = index + 1;
        if next < self.data.len() {
            let v = Rc::make_mut(&mut self.data);
            Some((next, &mut v[next]))
        } else {
            None
        }
    }

    /// Get mutable reference to the previous element before `index`.
    pub fn GetPrevWritable(&mut self, index: usize) -> Option<(usize, &mut T)> {
        if index == 0 {
            return None;
        }
        let prev = index - 1;
        let v = Rc::make_mut(&mut self.data);
        Some((prev, &mut v[prev]))
    }

    // --- Move operations (O(n) in Vec) ---
    // DIVERGED: C++ O(1) pointer relinks vs Rust O(n) Vec operations.
    // Methods exist for API correspondence; complexity differs.

    /// Move element at `index` to the beginning.
    pub fn MoveToBeg(&mut self, index: usize) {
        if index == 0 { return; }
        let v = Rc::make_mut(&mut self.data);
        let elem = v.remove(index);
        v.insert(0, elem);
    }

    /// Move element at `index` to the end.
    pub fn MoveToEnd(&mut self, index: usize) {
        let v = Rc::make_mut(&mut self.data);
        if index >= v.len() - 1 { return; }
        let elem = v.remove(index);
        v.push(elem);
    }

    /// Move element at `src` to before `dst`.
    pub fn MoveBefore(&mut self, src: usize, dst: usize) {
        let v = Rc::make_mut(&mut self.data);
        let elem = v.remove(src);
        let insert_at = if src < dst { dst - 1 } else { dst };
        v.insert(insert_at, elem);
    }

    /// Move element at `src` to after `dst`.
    pub fn MoveAfter(&mut self, src: usize, dst: usize) {
        let v = Rc::make_mut(&mut self.data);
        let elem = v.remove(src);
        let insert_at = if src <= dst { dst } else { dst + 1 };
        v.insert(insert_at, elem);
    }

    // --- SubList operations ---

    /// Get a copy of elements from `first` to `last` (inclusive).
    pub fn GetSubList(&self, first: usize, last: usize) -> emList<T> {
        emList {
            data: Rc::new(self.data[first..=last].to_vec()),
        }
    }

    /// Get a copy of the first `count` elements.
    pub fn GetSubListOfFirst(&self, count: usize) -> emList<T> {
        emList {
            data: Rc::new(self.data[..count].to_vec()),
        }
    }

    /// Get a copy of the last `count` elements.
    pub fn GetSubListOfLast(&self, count: usize) -> emList<T> {
        let start = self.data.len().saturating_sub(count);
        emList {
            data: Rc::new(self.data[start..].to_vec()),
        }
    }

    /// Remove and return elements from `first` to `last` (inclusive).
    pub fn Extract(&mut self, first: usize, last: usize) -> emList<T> {
        let v = Rc::make_mut(&mut self.data);
        let extracted: Vec<T> = v.drain(first..=last).collect();
        emList {
            data: Rc::new(extracted),
        }
    }

    // --- Multi-variant insertion ---

    /// Insert a slice at the beginning.
    pub fn InsertAtBeg_slice(&mut self, elements: &[T]) {
        let v = Rc::make_mut(&mut self.data);
        for (i, e) in elements.iter().enumerate() {
            v.insert(i, e.clone());
        }
    }

    /// Insert elements from another list at the beginning.
    pub fn InsertAtBeg_list(&mut self, other: &emList<T>) {
        self.InsertAtBeg_slice(&other.data);
    }

    /// Insert `count` copies of `element` at the beginning.
    pub fn InsertAtBeg_fill(&mut self, element: T, count: usize) {
        let v = Rc::make_mut(&mut self.data);
        for i in 0..count {
            v.insert(i, element.clone());
        }
    }

    /// Insert a slice at the end.
    pub fn InsertAtEnd_slice(&mut self, elements: &[T]) {
        let v = Rc::make_mut(&mut self.data);
        v.extend_from_slice(elements);
    }

    /// Insert elements from another list at the end.
    pub fn InsertAtEnd_list(&mut self, other: &emList<T>) {
        self.InsertAtEnd_slice(&other.data);
    }

    /// Insert `count` copies of `element` at the end.
    pub fn InsertAtEnd_fill(&mut self, element: T, count: usize) {
        let v = Rc::make_mut(&mut self.data);
        v.extend(std::iter::repeat_n(element, count));
    }

    /// Insert a slice before `index`.
    pub fn InsertBefore_slice(&mut self, index: usize, elements: &[T]) {
        let v = Rc::make_mut(&mut self.data);
        for (i, e) in elements.iter().enumerate() {
            v.insert(index + i, e.clone());
        }
    }

    /// Insert a slice after `index`.
    pub fn InsertAfter_slice(&mut self, index: usize, elements: &[T]) {
        self.InsertBefore_slice(index + 1, elements);
    }

    /// Add a slice at the end (alias for InsertAtEnd_slice).
    pub fn Add_slice(&mut self, elements: &[T]) {
        self.InsertAtEnd_slice(elements);
    }

    // --- Constructor variants ---

    /// Construct from two lists concatenated.
    pub fn from_two(a: &emList<T>, b: &emList<T>) -> Self {
        let mut v = a.data.as_ref().clone();
        v.extend_from_slice(&b.data);
        emList { data: Rc::new(v) }
    }

    /// Construct from a subrange of another list (first..=last).
    pub fn from_sublist(src: &emList<T>, first: usize, last: usize) -> Self {
        emList {
            data: Rc::new(src.data[first..=last].to_vec()),
        }
    }

    // --- Custom comparator sort ---

    /// Sort with a custom comparator. C++ `Sort(int(*)(const OBJ*,const OBJ*,void*), void*)`.
    pub fn Sort_by(&mut self, compare: impl FnMut(&T, &T) -> std::cmp::Ordering) {
        Rc::make_mut(&mut self.data).sort_by(compare);
    }
```

Update DIVERGED comments at top of file to remove mentions of `GetSubList`, `Extract`, `Move*`, multi-variant insertion.

- [ ] **Step 7: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 8: Commit**

```bash
git add crates/emcore/src/emList.rs crates/eaglemode/tests/behavioral/list.rs && git commit -m "feat(emList): add mutable nav, move ops, sublists, multi-variant insertion, constructors, Sort_by"
```

---

## Task 8: emAvlTreeMap Index + emAvlTreeSet Operator Traits

**Files:**
- Modify: `crates/emcore/src/emAvlTreeMap.rs`
- Modify: `crates/emcore/src/emAvlTreeSet.rs`
- Test: `crates/eaglemode/tests/behavioral/avl_tree_map.rs`
- Test: `crates/eaglemode/tests/behavioral/avl_tree_set.rs`

- [ ] **Step 1: Write tests**

In `crates/eaglemode/tests/behavioral/avl_tree_map.rs`:

```rust
#[test]
fn test_index_operator() {
    let mut map = emAvlTreeMap::new();
    map.Insert("a".to_string(), 1);
    map.Insert("b".to_string(), 2);
    assert_eq!(map[&"a".to_string()], 1);
    assert_eq!(map[&"b".to_string()], 2);
}

#[test]
#[should_panic]
fn test_index_missing_key_panics() {
    let map: emAvlTreeMap<String, i32> = emAvlTreeMap::new();
    let _ = map[&"missing".to_string()];
}
```

In `crates/eaglemode/tests/behavioral/avl_tree_set.rs`:

```rust
#[test]
fn test_bitor_union() {
    let mut a = emAvlTreeSet::from_element(1);
    a.Insert(2);
    let mut b = emAvlTreeSet::from_element(2);
    b.Insert(3);
    let c = &a | &b;
    assert_eq!(c.GetCount(), 3);
    assert!(c.Contains(&1));
    assert!(c.Contains(&2));
    assert!(c.Contains(&3));
}

#[test]
fn test_bitand_intersection() {
    let mut a = emAvlTreeSet::from_element(1);
    a.Insert(2);
    a.Insert(3);
    let mut b = emAvlTreeSet::from_element(2);
    b.Insert(3);
    b.Insert(4);
    let c = &a & &b;
    assert_eq!(c.GetCount(), 2);
    assert!(c.Contains(&2));
    assert!(c.Contains(&3));
}

#[test]
fn test_sub_difference() {
    let mut a = emAvlTreeSet::from_element(1);
    a.Insert(2);
    a.Insert(3);
    let mut b = emAvlTreeSet::from_element(2);
    let c = &a - &b;
    assert_eq!(c.GetCount(), 2);
    assert!(c.Contains(&1));
    assert!(c.Contains(&3));
}

#[test]
fn test_bitor_assign() {
    let mut a = emAvlTreeSet::from_element(1);
    let mut b = emAvlTreeSet::from_element(2);
    a |= &b;
    assert_eq!(a.GetCount(), 2);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test behavioral -- test_bitor_union`
Expected: FAIL

- [ ] **Step 3: Implement Index for emAvlTreeMap**

Add to `crates/emcore/src/emAvlTreeMap.rs`:

```rust
use std::ops::Index;

/// DIVERGED: C++ `operator[]` creates a default entry if missing.
/// Rust panics if key not found (no `Default` bound on `V`).
impl<K: Clone + Ord, V: Clone> Index<&K> for emAvlTreeMap<K, V> {
    type Output = V;
    fn index(&self, key: &K) -> &V {
        self.GetValue(key).expect("emAvlTreeMap: key not found")
    }
}
```

- [ ] **Step 4: Implement operator traits for emAvlTreeSet**

Add to `crates/emcore/src/emAvlTreeSet.rs`:

```rust
use std::ops::{BitOr, BitOrAssign, BitAnd, BitAndAssign, Sub, SubAssign, Add, AddAssign};

impl<T: Clone + Ord> BitOr<&emAvlTreeSet<T>> for &emAvlTreeSet<T> {
    type Output = emAvlTreeSet<T>;
    fn bitor(self, rhs: &emAvlTreeSet<T>) -> emAvlTreeSet<T> {
        let mut result = self.clone();
        result.InsertSet(rhs);
        result
    }
}

impl<T: Clone + Ord> BitOrAssign<&emAvlTreeSet<T>> for emAvlTreeSet<T> {
    fn bitor_assign(&mut self, rhs: &emAvlTreeSet<T>) {
        self.InsertSet(rhs);
    }
}

impl<T: Clone + Ord> BitAnd<&emAvlTreeSet<T>> for &emAvlTreeSet<T> {
    type Output = emAvlTreeSet<T>;
    fn bitand(self, rhs: &emAvlTreeSet<T>) -> emAvlTreeSet<T> {
        let mut result = self.clone();
        result.Intersect(rhs);
        result
    }
}

impl<T: Clone + Ord> BitAndAssign<&emAvlTreeSet<T>> for emAvlTreeSet<T> {
    fn bitand_assign(&mut self, rhs: &emAvlTreeSet<T>) {
        self.Intersect(rhs);
    }
}

impl<T: Clone + Ord> Sub<&emAvlTreeSet<T>> for &emAvlTreeSet<T> {
    type Output = emAvlTreeSet<T>;
    fn sub(self, rhs: &emAvlTreeSet<T>) -> emAvlTreeSet<T> {
        let mut result = self.clone();
        result.RemoveSet(rhs);
        result
    }
}

impl<T: Clone + Ord> SubAssign<&emAvlTreeSet<T>> for emAvlTreeSet<T> {
    fn sub_assign(&mut self, rhs: &emAvlTreeSet<T>) {
        self.RemoveSet(rhs);
    }
}

impl<T: Clone + Ord> Add<T> for &emAvlTreeSet<T> {
    type Output = emAvlTreeSet<T>;
    fn add(self, rhs: T) -> emAvlTreeSet<T> {
        let mut result = self.clone();
        result.Insert(rhs);
        result
    }
}

impl<T: Clone + Ord> AddAssign<T> for emAvlTreeSet<T> {
    fn add_assign(&mut self, rhs: T) {
        self.Insert(rhs);
    }
}
```

Remove DIVERGED comment about operator overloads (line 22 area).

- [ ] **Step 5: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emAvlTreeMap.rs crates/emcore/src/emAvlTreeSet.rs crates/eaglemode/tests/behavioral/ && git commit -m "feat(collections): add Index for emAvlTreeMap, operator traits for emAvlTreeSet"
```

---

## Task 9: emCursor Get + emButton EOI Signal

**Files:**
- Modify: `crates/emcore/src/emCursor.rs`
- Modify: `crates/emcore/src/emButton.rs`

- [ ] **Step 1: Write tests**

In `crates/emcore/src/emCursor.rs` test module (create if needed):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_returns_self() {
        let c = emCursor::Hand;
        assert_eq!(c.Get(), emCursor::Hand);
    }
}
```

In `crates/emcore/src/emButton.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eoi_callback_fires() {
        let fired = std::rc::Rc::new(std::cell::Cell::new(false));
        let fired_clone = fired.clone();
        let look = std::rc::Rc::new(emLook::default());
        let mut btn = emButton::new("test", look);
        btn.on_eoi = Some(Box::new(move || { fired_clone.set(true); }));
        btn.Click();
        assert!(fired.get(), "EOI callback should fire after Click");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p emcore --lib -- emCursor::tests::test_get`
Expected: FAIL

- [ ] **Step 3: Add emCursor::Get**

In `crates/emcore/src/emCursor.rs`, add to the impl block:

```rust
    /// Return this cursor variant. C++ `emCursor::Get()` returns the int id;
    /// Rust returns the enum variant itself.
    pub fn Get(self) -> Self {
        self
    }
```

Update DIVERGED comment on line 28 to explain Get returns Self (identity for enum).

- [ ] **Step 4: Add emButton on_eoi field and wiring**

In `crates/emcore/src/emButton.rs`:

Add field to struct:
```rust
    pub on_eoi: Option<Box<dyn FnMut()>>,
```

Initialize in `new()`:
```rust
    on_eoi: None,
```

Update `Click()` method to fire EOI after click:
```rust
    pub fn Click(&mut self) {
        if !self.enabled {
            return;
        }
        if let Some(cb) = &mut self.on_click {
            cb();
        }
        if !self.no_eoi {
            if let Some(eoi) = &mut self.on_eoi {
                eoi();
            }
        }
    }
```

Update `Input()` — fire EOI on mouse release (after the existing click firing):
In the `InputVariant::Release` arm, after firing `on_click`, add:
```rust
                if !self.no_eoi {
                    if let Some(eoi) = &mut self.on_eoi {
                        eoi();
                    }
                }
```

Remove "EOI signal not implemented" comment on line 401.

- [ ] **Step 5: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add crates/emcore/src/emCursor.rs crates/emcore/src/emButton.rs && git commit -m "feat: add emCursor::Get and emButton EOI signal callback"
```

---

## Task 10: emTmpFileMaster

**Files:**
- Modify: `crates/emcore/src/emTmpFile.rs`
- Test: `crates/eaglemode/tests/behavioral/tmp_file.rs`

- [ ] **Step 1: Write tests**

Add to `crates/eaglemode/tests/behavioral/tmp_file.rs`:

```rust
#[test]
fn test_master_singleton_acquires_lock() {
    let dir = tempfile::tempdir().unwrap();
    let master = emTmpFileMaster::acquire(dir.path());
    assert!(master.is_some(), "first acquisition should succeed");
}

#[test]
fn test_master_registers_and_cleans() {
    let dir = tempfile::tempdir().unwrap();
    let tmp_path = dir.path().join("test_tmp_file.dat");
    std::fs::write(&tmp_path, b"data").unwrap();
    assert!(tmp_path.exists());

    let mut master = emTmpFileMaster::acquire(dir.path()).unwrap();
    master.register(&tmp_path);
    assert!(master.is_registered(&tmp_path));

    master.unregister(&tmp_path);
    assert!(!master.is_registered(&tmp_path));
}

#[test]
fn test_master_cleans_on_drop() {
    let dir = tempfile::tempdir().unwrap();
    let tmp_path = dir.path().join("cleanup_test.dat");
    std::fs::write(&tmp_path, b"data").unwrap();

    {
        let mut master = emTmpFileMaster::acquire(dir.path()).unwrap();
        master.register(&tmp_path);
    } // master drops here

    // Registered file should be cleaned up
    assert!(!tmp_path.exists(), "registered file should be deleted on master drop");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test behavioral -- test_master_singleton`
Expected: FAIL — `emTmpFileMaster` doesn't exist

- [ ] **Step 3: Implement emTmpFileMaster**

Add to `crates/emcore/src/emTmpFile.rs`:

```rust
use std::collections::HashSet;

/// Singleton manager for temporary file cleanup.
/// Uses file locking to ensure only one master per temp directory.
///
/// C++ uses IPC (emMiniIpc) for the singleton. Rust uses flock.
pub struct emTmpFileMaster {
    lock_file: std::fs::File,
    lock_path: PathBuf,
    registered: HashSet<PathBuf>,
    base_dir: PathBuf,
}

impl emTmpFileMaster {
    /// Try to acquire the master lock for the given temp directory.
    /// Returns `None` if another process holds the lock.
    pub fn acquire(base_dir: &Path) -> Option<Self> {
        let lock_path = base_dir.join(".emTmpFileMaster.lock");
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .ok()?;

        // Try non-blocking exclusive lock
        use std::os::unix::io::AsRawFd;
        let rc = unsafe { libc::flock(lock_file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if rc != 0 {
            return None; // Another process holds the lock
        }

        let mut master = Self {
            lock_file,
            lock_path,
            registered: HashSet::new(),
            base_dir: base_dir.to_path_buf(),
        };

        // Clean orphaned temp files (from crashed processes)
        master.clean_orphans();

        Some(master)
    }

    /// Register a temp file path for cleanup tracking.
    pub fn register(&mut self, path: &Path) {
        self.registered.insert(path.to_path_buf());
    }

    /// Unregister a temp file path.
    pub fn unregister(&mut self, path: &Path) {
        self.registered.remove(path);
    }

    /// Check if a path is registered.
    pub fn is_registered(&self, path: &Path) -> bool {
        self.registered.contains(path)
    }

    /// Scan for and remove orphaned temp files.
    fn clean_orphans(&mut self) {
        if let Ok(entries) = std::fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // Match emTmpFile naming pattern: em_tmp_*
                if name_str.starts_with("em_tmp_") {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

impl Drop for emTmpFileMaster {
    fn drop(&mut self) {
        // Clean up all registered temp files
        for path in &self.registered {
            if path.is_dir() {
                let _ = std::fs::remove_dir_all(path);
            } else {
                let _ = std::fs::remove_file(path);
            }
        }

        // Release the lock file
        let _ = std::fs::remove_file(&self.lock_path);
    }
}
```

Remove the DIVERGED comment about emTmpFileMaster being deferred (lines 6-10).

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emTmpFile.rs crates/eaglemode/tests/behavioral/tmp_file.rs && git commit -m "feat(emTmpFile): port emTmpFileMaster with flock-based singleton"
```

---

## Task 11: emArray Cursor Auto-Adjustment

**Files:**
- Modify: `crates/emcore/src/emArray.rs`
- Test: `crates/eaglemode/tests/behavioral/array.rs`

The C++ emArray::Iterator auto-adjusts its index when elements are inserted/removed before the cursor position. The Rust Cursor does not. This is a behavioral divergence.

- [ ] **Step 1: Write test for cursor auto-adjustment**

```rust
#[test]
fn test_cursor_adjusts_on_insert_before() {
    let mut arr = emArray::new();
    arr.Add_one(10);
    arr.Add_one(20);
    arr.Add_one(30);

    let cursor = arr.cursor_at(1); // points to 20
    assert_eq!(cursor.Get(&arr), Some(&20));

    // Insert before cursor position
    arr.Insert_one(0, 5); // [5, 10, 20, 30]

    // Cursor should auto-adjust: still points to 20 (now at index 2)
    assert_eq!(cursor.Get(&arr), Some(&20));
}

#[test]
fn test_cursor_adjusts_on_remove_before() {
    let mut arr = emArray::new();
    arr.Add_one(10);
    arr.Add_one(20);
    arr.Add_one(30);

    let cursor = arr.cursor_at(2); // points to 30
    assert_eq!(cursor.Get(&arr), Some(&30));

    arr.Remove(0); // [20, 30]

    // Cursor should auto-adjust: still points to 30 (now at index 1)
    assert_eq!(cursor.Get(&arr), Some(&30));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test behavioral -- test_cursor_adjusts`
Expected: FAIL — cursor doesn't auto-adjust

- [ ] **Step 3: Implement cursor auto-adjustment**

This requires emArray to track mutations and Cursor to check them. The approach from the spec: add an adjustment log to emArray, tracked via shared `Rc`.

Add to `emArray` struct:
```rust
    /// Adjustment log for cursor auto-adjustment.
    /// Each entry: (delta, at_index) — cursor indices >= at_index shift by delta.
    adjustments: Rc<RefCell<Vec<(isize, usize)>>>,
```

Modify `Cursor` to hold a reference to the adjustment log:
```rust
pub struct Cursor {
    index: Option<usize>,
    adjustments: Rc<RefCell<Vec<(isize, usize)>>>,
    last_adj_len: usize, // how many adjustments we've already applied
}
```

In every mutation method (`Insert_*`, `Remove`, `Add_*`, etc.), append to the adjustment log:
```rust
    // In Insert_one(index, obj):
    self.adjustments.borrow_mut().push((1, index));

    // In Remove(index):
    self.adjustments.borrow_mut().push((-1, index));
```

In `Cursor::Get()`, `SetNext()`, `SetPrev()`, apply pending adjustments:
```rust
    fn apply_adjustments(&mut self) {
        let adjs = self.adjustments.borrow();
        for i in self.last_adj_len..adjs.len() {
            let (delta, at_index) = adjs[i];
            if let Some(ref mut idx) = self.index {
                if *idx >= at_index {
                    *idx = (*idx as isize + delta).max(0) as usize;
                }
            }
        }
        self.last_adj_len = adjs.len();
    }
```

Remove DIVERGED comment at line 33 of emArray.rs.

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emcore/src/emArray.rs crates/eaglemode/tests/behavioral/array.rs && git commit -m "feat(emArray): implement cursor auto-adjustment on insert/remove"
```

---

## Task 12: Update DIVERGED Comments

**Files:**
- Modify: All files touched in Tasks 1-10

- [ ] **Step 1: Audit remaining DIVERGED comments**

After all previous tasks, grep for remaining DIVERGED comments in modified files. For each:
- If the gap was closed → remove the comment
- If it's a genuine Rust constraint (Copy semantics, no null refs, no pointer arithmetic) → update the comment with clearer reasoning
- If it was partially closed → update to reflect remaining gap

- [ ] **Step 2: Run final verification**

Run: `cargo-nextest ntr`
Run: `cargo clippy -- -D warnings`
Expected: All PASS, no warnings

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "chore: update DIVERGED comments after Phase 1 gap closure"
```
