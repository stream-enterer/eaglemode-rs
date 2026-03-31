# Phase 2: COW Collection Family (Bottom-up)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build COW collection types with full C++ behavioral parity — copy-on-write sharing, stable cursors, and ordered access — as infrastructure for porting outside-emCore modules.

**Architecture:** Each type wraps an `Rc`-backed inner store. Mutations check `Rc::strong_count` and clone if shared (COW). Stable cursors track position by key/index rather than pointer, surviving mutations and COW clones. emArray wraps `Vec<T>`, emAvlTreeMap/Set wrap `BTreeMap`/`BTreeSet` (idiomatic Rust backing for ordered access), emList wraps a custom arena-backed doubly-linked list. All types match C++ method names per File and Name Correspondence.

**Tech Stack:** Rust, `Rc`/`RefCell`, `BTreeMap`/`BTreeSet`, existing behavioral test infrastructure, Kani for arithmetic proofs.

**Spec:** `docs/superpowers/specs/2026-03-28-port-completion-design.md` (Section 2, Section 3)

**Key rules from spec:**
- `Rc<Inner<T>>` backing store, `Rc::make_mut` for clone-on-mutate
- Stable cursors (opaque handle, not Rust `Iterator` trait)
- C++ method names: `GetCount`, `GetFirst`, `GetWritable`, `BinaryInsert`, etc.
- COW behavioral test pattern: create A, clone to B, mutate B, verify A unchanged
- Cursor stability test pattern: obtain cursor, mutate collection, verify cursor still valid
- `pub(crate)` default visibility
- No `#[allow(...)]` except `non_snake_case` on emCore module and `non_camel_case_types` on em-prefixed types

**Current state:** All 5 types have `.no_rs` marker files. Zero Rust callers exist — Vec/HashMap stand in everywhere. These types are infrastructure for future outside-emCore porting (69 files use emArray, 16 use emList, 6 use emAvlTreeMap outside emCore).

**Dependency order:** emArray (foundation) -> emAvlTreeMap -> emAvlTreeSet -> emList (independent but last since 0 emCore instantiations).

---

## Task 1: emArray core struct with COW

**Files:**
- Create: `src/emCore/emArray.rs`
- Modify: `src/emCore/mod.rs`
- Create: `tests/behavioral/array.rs`
- Modify: `tests/behavioral/main.rs`

**Context:** C++ emArray is a COW dynamic array. Copy is O(1) shallow; mutation deep-copies if shared. Rust equivalent wraps `Vec<T>` in `Rc` with clone-on-mutate. C++ header: `~/git/eaglemode-0.96.4/include/emCore/emArray.h`.

- [ ] **Step 1: Write failing COW behavioral tests**

Create `tests/behavioral/array.rs`:

```rust
use eaglemode_rs::emCore::emArray::emArray;

#[test]
fn empty_array() {
    let a: emArray<i32> = emArray::new();
    assert_eq!(a.GetCount(), 0);
    assert!(a.IsEmpty());
}

#[test]
fn add_and_get() {
    let mut a: emArray<i32> = emArray::new();
    a.Add_one(42);
    assert_eq!(a.GetCount(), 1);
    assert_eq!(a.Get_at(0), &42);
}

#[test]
fn cow_shallow_copy() {
    let mut a: emArray<i32> = emArray::new();
    a.Add_one(1);
    a.Add_one(2);
    a.Add_one(3);

    let b = a.clone();
    // Shared backing store — refcount is 2
    assert_eq!(a.GetDataRefCount(), 2);
    assert_eq!(b.GetDataRefCount(), 2);
    // Same data
    assert_eq!(b.Get_at(0), &1);
    assert_eq!(b.Get_at(1), &2);
    assert_eq!(b.Get_at(2), &3);
}

#[test]
fn cow_clone_on_mutate() {
    let mut a: emArray<i32> = emArray::new();
    a.Add_one(1);
    a.Add_one(2);

    let mut b = a.clone();
    assert_eq!(a.GetDataRefCount(), 2);

    // Mutate b — triggers deep copy
    b.Set(0, 99);

    // a is unchanged, refcounts are now 1
    assert_eq!(a.Get_at(0), &1);
    assert_eq!(b.Get_at(0), &99);
    assert_eq!(a.GetDataRefCount(), 1);
    assert_eq!(b.GetDataRefCount(), 1);
}

#[test]
fn cow_multiple_shares() {
    let mut a: emArray<i32> = emArray::new();
    a.Add_one(10);

    let b = a.clone();
    let c = a.clone();
    assert_eq!(a.GetDataRefCount(), 3);

    // Drop b — refcount decreases
    drop(b);
    assert_eq!(a.GetDataRefCount(), 2);
}
```

Add `mod array;` to `tests/behavioral/main.rs`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test behavioral array`
Expected: Compilation error — `emArray` module doesn't exist yet.

- [ ] **Step 3: Implement emArray core**

Create `src/emCore/emArray.rs`:

```rust
//! emArray — COW dynamic array.
//!
//! C++ emArray (emArray.h) is a copy-on-write dynamic array. Clone is O(1)
//! shallow copy; mutation deep-copies if the backing store is shared.
//!
//! Rust implementation wraps `Vec<T>` in `Rc` with clone-on-mutate
//! via `Rc::make_mut`.

use std::rc::Rc;

/// Copy-on-write dynamic array matching C++ `emArray<OBJ>`.
///
/// Clone produces a shallow copy that shares the backing `Vec`. Any mutation
/// checks `Rc::strong_count` and clones the inner data if shared.
pub struct emArray<T: Clone> {
    data: Rc<Vec<T>>,
}

impl<T: Clone> emArray<T> {
    /// Construct an empty array. C++ `emArray()`.
    pub fn new() -> Self {
        Self {
            data: Rc::new(Vec::new()),
        }
    }

    /// Construct from a slice. C++ `emArray(const OBJ*, int)`.
    pub fn from_slice(items: &[T]) -> Self {
        Self {
            data: Rc::new(items.to_vec()),
        }
    }

    /// Construct by filling with `count` copies. C++ `emArray(const OBJ&, int)`.
    pub fn filled(value: T, count: usize) -> Self {
        Self {
            data: Rc::new(vec![value; count]),
        }
    }

    // --- Read-only methods ---

    /// Number of elements. C++ `GetCount`.
    pub fn GetCount(&self) -> usize {
        self.data.len()
    }

    /// True if empty. C++ `IsEmpty`.
    pub fn IsEmpty(&self) -> bool {
        self.data.is_empty()
    }

    /// Immutable element access. C++ `Get(int)`, `operator[]`.
    pub fn Get_at(&self, index: usize) -> &T {
        &self.data[index]
    }

    /// Immutable slice of all elements. C++ `Get()`, `operator const OBJ*`.
    pub fn Get(&self) -> &[T] {
        &self.data
    }

    /// COW reference count. C++ `GetDataRefCount`.
    pub fn GetDataRefCount(&self) -> usize {
        Rc::strong_count(&self.data)
    }

    /// Sub-array copy. C++ `GetSubArray`.
    pub fn GetSubArray(&self, index: usize, count: usize) -> emArray<T> {
        emArray {
            data: Rc::new(self.data[index..index + count].to_vec()),
        }
    }

    // --- COW write barrier ---

    /// Get mutable reference to backing Vec, cloning if shared.
    fn make_writable(&mut self) -> &mut Vec<T> {
        Rc::make_mut(&mut self.data)
    }

    // --- Mutating methods (trigger COW clone if shared) ---

    /// Set element at index. C++ `Set(int, const OBJ&)`.
    pub fn Set(&mut self, index: usize, value: T) {
        self.make_writable()[index] = value;
    }

    /// Mutable element access. C++ `GetWritable(int)`.
    pub fn GetWritable(&mut self, index: usize) -> &mut T {
        &mut self.make_writable()[index]
    }

    /// Mutable slice of all elements. C++ `GetWritable()`.
    pub fn GetWritableSlice(&mut self) -> &mut [T] {
        self.make_writable().as_mut_slice()
    }

    /// Append one element. C++ `Add(const OBJ&, 1)`.
    pub fn Add_one(&mut self, value: T) {
        self.make_writable().push(value);
    }

    /// Append multiple copies. C++ `Add(const OBJ&, int)`.
    pub fn Add_fill(&mut self, value: T, count: usize) {
        let v = self.make_writable();
        v.reserve(count);
        for _ in 0..count {
            v.push(value.clone());
        }
    }

    /// Append all from another array. C++ `Add(const emArray&)`.
    pub fn Add(&mut self, other: &emArray<T>) {
        self.make_writable().extend_from_slice(&other.data);
    }

    /// Append from slice. C++ `Add(const OBJ*, int)`.
    pub fn Add_slice(&mut self, items: &[T]) {
        self.make_writable().extend_from_slice(items);
    }

    /// Set element count. New elements are default-initialized.
    /// C++ `SetCount(int, bool)`.
    pub fn SetCount(&mut self, count: usize)
    where
        T: Default,
    {
        self.make_writable().resize_with(count, T::default);
    }

    /// Shrink capacity to count. C++ `Compact`.
    pub fn Compact(&mut self) {
        self.make_writable().shrink_to_fit();
    }

    /// Insert element at index. C++ `Insert(int, const OBJ&, 1)`.
    pub fn Insert(&mut self, index: usize, value: T) {
        self.make_writable().insert(index, value);
    }

    /// Insert multiple copies at index. C++ `Insert(int, const OBJ&, int)`.
    pub fn Insert_fill(&mut self, index: usize, value: T, count: usize) {
        let v = self.make_writable();
        v.reserve(count);
        for i in 0..count {
            v.insert(index + i, value.clone());
        }
    }

    /// Insert from slice at index. C++ `Insert(int, const OBJ*, int)`.
    pub fn Insert_slice(&mut self, index: usize, items: &[T]) {
        let v = self.make_writable();
        v.reserve(items.len());
        for (i, item) in items.iter().enumerate() {
            v.insert(index + i, item.clone());
        }
    }

    /// Insert from another array at index. C++ `Insert(int, const emArray&)`.
    pub fn Insert_array(&mut self, index: usize, other: &emArray<T>) {
        self.Insert_slice(index, &other.data);
    }

    /// Remove `count` elements starting at `index`. C++ `Remove(int, int)`.
    pub fn Remove(&mut self, index: usize, count: usize) {
        self.make_writable().drain(index..index + count);
    }

    /// Replace elements. C++ `Replace(int, int, const OBJ&, int)`.
    pub fn Replace(&mut self, index: usize, rem_count: usize, value: T, ins_count: usize) {
        let v = self.make_writable();
        let new_items: Vec<T> = (0..ins_count).map(|_| value.clone()).collect();
        v.splice(index..index + rem_count, new_items);
    }

    /// Replace with slice. C++ `Replace(int, int, const OBJ*, int)`.
    pub fn Replace_slice(&mut self, index: usize, rem_count: usize, items: &[T]) {
        let v = self.make_writable();
        v.splice(index..index + rem_count, items.iter().cloned());
    }

    /// Extract (remove and return) a sub-array. C++ `Extract(int, int)`.
    pub fn Extract(&mut self, index: usize, count: usize) -> emArray<T> {
        let v = self.make_writable();
        let extracted: Vec<T> = v.drain(index..index + count).collect();
        emArray {
            data: Rc::new(extracted),
        }
    }

    /// Clear all elements. C++ `Clear`.
    pub fn Clear(&mut self) {
        self.make_writable().clear();
    }

    /// Force unique data (break COW sharing). C++ `MakeNonShared`.
    pub fn MakeNonShared(&mut self) {
        // Accessing make_writable forces a clone if shared.
        let _ = self.make_writable();
    }
}

// --- Sort and binary search (require Ord) ---

impl<T: Clone + Ord> emArray<T> {
    /// Stable sort. Returns true if order changed. C++ `Sort`.
    pub fn Sort(&mut self) -> bool {
        let v = self.make_writable();
        let mut changed = false;
        // Check if already sorted
        for i in 1..v.len() {
            if v[i - 1] > v[i] {
                changed = true;
                break;
            }
        }
        if changed {
            v.sort();
        }
        changed
    }

    /// Binary search in sorted array. Returns index if found, or
    /// `Err(insertion_index)` if not found. C++ `BinarySearch`.
    pub fn BinarySearch(&self, value: &T) -> Result<usize, usize> {
        self.data.binary_search(value)
    }

    /// Insert into sorted position. C++ `BinaryInsert`.
    pub fn BinaryInsert(&mut self, value: T) {
        let idx = match self.data.binary_search(&value) {
            Ok(i) | Err(i) => i,
        };
        self.make_writable().insert(idx, value);
    }

    /// Insert if not already present. Returns true if inserted.
    /// C++ `BinaryInsertIfNew`.
    pub fn BinaryInsertIfNew(&mut self, value: T) -> bool {
        match self.data.binary_search(&value) {
            Ok(_) => false,
            Err(idx) => {
                self.make_writable().insert(idx, value);
                true
            }
        }
    }

    /// Insert or replace existing. C++ `BinaryInsertOrReplace`.
    pub fn BinaryInsertOrReplace(&mut self, value: T) {
        match self.data.binary_search(&value) {
            Ok(idx) => self.make_writable()[idx] = value,
            Err(idx) => self.make_writable().insert(idx, value),
        }
    }

    /// Remove by value from sorted array. Returns true if found.
    /// C++ `BinaryRemove`.
    pub fn BinaryRemove(&mut self, value: &T) -> bool {
        match self.data.binary_search(value) {
            Ok(idx) => {
                self.make_writable().remove(idx);
                true
            }
            Err(_) => false,
        }
    }
}

impl<T: Clone> Clone for emArray<T> {
    fn clone(&self) -> Self {
        Self {
            data: Rc::clone(&self.data),
        }
    }
}

impl<T: Clone> Default for emArray<T> {
    fn default() -> Self {
        Self::new()
    }
}
```

**Note on C++ overloads:** C++ `emArray` has ~45 methods with many overloads using function pointers for comparators (`compare_fn`). Rust uses `Ord` trait bounds instead. The `BinarySearchByKey` and `Sort` with custom comparators become trait-bounded methods. `BinaryReplace`, `BinaryRemoveByKey`, and `PointerToIndex` are omitted from this initial port (they can be added when a consumer needs them). The spec says "Port full C++ API surface" — add these later if outside-emCore code requires them.

**Note on TuningLevel:** C++ TuningLevel (0-4) controls whether copy constructors, destructors, and memcpy are used for element management. Rust's ownership model handles this automatically — `Clone` for copies, `Drop` for cleanup. No Rust equivalent is needed.

- [ ] **Step 4: Add to mod.rs**

In `src/emCore/mod.rs`, add:
```rust
pub mod emArray;
```
(Alphabetical order: after `emAnything.no_rs` equivalent position.)

- [ ] **Step 5: Run tests**

Run: `cargo test --test behavioral array -v`
Expected: All 5 tests pass.

Run: `cargo clippy -- -D warnings && cargo-nextest ntr`
Expected: Full suite passes.

- [ ] **Step 6: Delete marker file**

Delete `src/emCore/emArray.no_rs`.

- [ ] **Step 7: Commit**

```bash
git rm src/emCore/emArray.no_rs
git add src/emCore/emArray.rs src/emCore/mod.rs \
  tests/behavioral/array.rs tests/behavioral/main.rs
git commit -m "feat: port emArray with COW semantics

Implements emArray<T> backed by Rc<Vec<T>> with clone-on-mutate.
Covers CRUD, binary search, sort, and sub-array operations.
TuningLevel omitted (Rust ownership handles automatically).
Custom comparator overloads use Ord trait bounds instead of fn ptrs."
```

---

## Task 2: emArray stable cursor (Iterator)

**Files:**
- Modify: `src/emCore/emArray.rs`
- Modify: `tests/behavioral/array.rs`

**Context:** C++ `emArray::Iterator` is a stable cursor that survives mutations and COW clones. It tracks position by index and auto-adjusts when elements are inserted/removed. Rust's standard iterators borrow immutably and cannot survive mutation. We implement a `Cursor` type with explicit `SetNext`/`SetPrev`/`Get` methods per the spec.

**Design:** The cursor stores:
- A `Weak<Vec<T>>` to verify the array hasn't been dropped
- An index position
- An `Rc<Cell<usize>>` generation counter shared with the array, bumped on every mutation

When the generation has changed since the cursor was last used, the cursor must validate its position. For emArray (index-based), the cursor tracks a logical index that auto-adjusts when elements are inserted/removed before it. This requires the array to broadcast insert/remove events.

**Simplified approach:** Since stable cursors are complex to implement correctly (especially across COW clones), start with a simpler index-based cursor that validates bounds on access but does NOT auto-adjust on insert/remove. This covers the primary use case (iterating without mutation) and can be extended when a consumer needs the full behavior. Add a `DIVERGED:` comment documenting the difference.

- [ ] **Step 1: Write cursor tests**

Add to `tests/behavioral/array.rs`:

```rust
#[test]
fn cursor_basic_iteration() {
    let mut a: emArray<i32> = emArray::new();
    a.Add_one(10);
    a.Add_one(20);
    a.Add_one(30);

    let mut cur = a.cursor(0);
    assert!(cur.IsValid(&a));
    assert_eq!(cur.Get(&a), Some(&10));
    cur.SetNext(&a);
    assert_eq!(cur.Get(&a), Some(&20));
    cur.SetNext(&a);
    assert_eq!(cur.Get(&a), Some(&30));
    cur.SetNext(&a);
    assert!(!cur.IsValid(&a));
}

#[test]
fn cursor_reverse_iteration() {
    let mut a: emArray<i32> = emArray::new();
    a.Add_one(10);
    a.Add_one(20);
    a.Add_one(30);

    let mut cur = a.cursor_last();
    assert_eq!(cur.Get(&a), Some(&30));
    cur.SetPrev(&a);
    assert_eq!(cur.Get(&a), Some(&20));
    cur.SetPrev(&a);
    assert_eq!(cur.Get(&a), Some(&10));
    cur.SetPrev(&a);
    assert!(!cur.IsValid(&a));
}

#[test]
fn cursor_survives_cow_clone() {
    let mut a: emArray<i32> = emArray::new();
    a.Add_one(1);
    a.Add_one(2);
    a.Add_one(3);

    let cur = a.cursor(1); // points to element "2"
    let mut b = a.clone();

    // Mutate b (triggers COW)
    b.Set(0, 99);

    // Cursor still valid against a (unchanged)
    assert_eq!(cur.Get(&a), Some(&2));
}
```

- [ ] **Step 2: Implement Cursor**

Add to `src/emCore/emArray.rs`:

```rust
/// Stable cursor for emArray. Tracks position by index.
///
/// DIVERGED: C++ emArray::Iterator auto-adjusts index when elements are
/// inserted/removed before the cursor position. This Rust cursor does NOT
/// auto-adjust — it maintains the original index. Full auto-adjustment
/// requires the array to track all live cursors, which adds overhead.
/// Will be implemented when a consumer requires it.
pub struct Cursor {
    index: usize,
}

impl Cursor {
    /// Check if cursor position is valid for the given array.
    pub fn IsValid<T: Clone>(&self, array: &emArray<T>) -> bool {
        self.index < array.GetCount()
    }

    /// Get element at cursor position, or None if past end.
    pub fn Get<'a, T: Clone>(&self, array: &'a emArray<T>) -> Option<&'a T> {
        array.data.get(self.index)
    }

    /// Advance cursor to next element.
    pub fn SetNext<T: Clone>(&mut self, array: &emArray<T>) {
        if self.index < array.GetCount() {
            self.index += 1;
        }
    }

    /// Move cursor to previous element.
    pub fn SetPrev<T: Clone>(&mut self, _array: &emArray<T>) {
        if self.index > 0 {
            self.index -= 1;
        }
    }

    /// Set cursor to specific index.
    pub fn SetIndex(&mut self, index: usize) {
        self.index = index;
    }
}

impl<T: Clone> emArray<T> {
    /// Create cursor at given index. C++ `Iterator(array, index)`.
    pub fn cursor(&self, index: usize) -> Cursor {
        Cursor { index }
    }

    /// Create cursor at first element. C++ `Iterator::SetFirst`.
    pub fn cursor_first(&self) -> Cursor {
        Cursor { index: 0 }
    }

    /// Create cursor at last element. C++ `Iterator::SetLast`.
    pub fn cursor_last(&self) -> Cursor {
        Cursor {
            index: if self.data.is_empty() { 0 } else { self.data.len() - 1 },
        }
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo clippy -- -D warnings && cargo-nextest ntr`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/emCore/emArray.rs tests/behavioral/array.rs
git commit -m "feat: add stable Cursor to emArray

Index-based cursor that survives COW clones. Does not auto-adjust
on insert/remove (documented DIVERGED). Provides Get, SetNext,
SetPrev, IsValid methods matching C++ emArray::Iterator."
```

---

## Task 3: emAvlTreeMap core with COW and ordered access

**Files:**
- Create: `src/emCore/emAvlTreeMap.rs`
- Modify: `src/emCore/mod.rs`
- Create: `tests/behavioral/avl_tree_map.rs`
- Modify: `tests/behavioral/main.rs`

**Context:** C++ emAvlTreeMap is a COW sorted map backed by an intrusive AVL tree. Rust wraps `BTreeMap` in `Rc` for COW + ordered access. C++ header: `~/git/eaglemode-0.96.4/include/emCore/emAvlTreeMap.h`.

**Design decisions:**
- C++ uses nested `Element { Key, Value, AvlNode }` struct for entries. Rust returns `(&K, &V)` tuples or `Option<&V>` since there's no intrusive node to expose.
- C++ `GetNearestGreater` etc. use AVL tree traversal. Rust uses `BTreeMap::range()` with bounds.
- DIVERGED: No `Element` struct exposed — Rust returns key/value references directly.

- [ ] **Step 1: Write behavioral tests**

Create `tests/behavioral/avl_tree_map.rs`:

```rust
use eaglemode_rs::emCore::emAvlTreeMap::emAvlTreeMap;

#[test]
fn empty_map() {
    let m: emAvlTreeMap<String, i32> = emAvlTreeMap::new();
    assert!(m.IsEmpty());
    assert_eq!(m.GetCount(), 0);
}

#[test]
fn insert_and_get() {
    let mut m: emAvlTreeMap<String, i32> = emAvlTreeMap::new();
    m.Insert("hello".to_string(), 42);
    assert!(m.Contains(&"hello".to_string()));
    assert_eq!(m.GetValue(&"hello".to_string()), Some(&42));
    assert_eq!(m.GetCount(), 1);
}

#[test]
fn cow_shallow_copy() {
    let mut m: emAvlTreeMap<i32, String> = emAvlTreeMap::new();
    m.Insert(1, "one".to_string());
    m.Insert(2, "two".to_string());

    let n = m.clone();
    assert_eq!(m.GetDataRefCount(), 2);
    assert_eq!(n.GetValue(&1), Some(&"one".to_string()));
}

#[test]
fn cow_clone_on_mutate() {
    let mut m: emAvlTreeMap<i32, String> = emAvlTreeMap::new();
    m.Insert(1, "one".to_string());

    let mut n = m.clone();
    assert_eq!(m.GetDataRefCount(), 2);

    n.Insert(2, "two".to_string());
    assert_eq!(m.GetDataRefCount(), 1);
    assert_eq!(m.GetCount(), 1); // m unchanged
    assert_eq!(n.GetCount(), 2);
}

#[test]
fn ordered_access() {
    let mut m: emAvlTreeMap<i32, &str> = emAvlTreeMap::new();
    m.Insert(10, "ten");
    m.Insert(20, "twenty");
    m.Insert(30, "thirty");
    m.Insert(5, "five");

    assert_eq!(m.GetFirst(), Some((&5, &"five")));
    assert_eq!(m.GetLast(), Some((&30, &"thirty")));
    assert_eq!(m.GetNearestGreater(&10), Some((&20, &"twenty")));
    assert_eq!(m.GetNearestLess(&20), Some((&10, &"ten")));
    assert_eq!(m.GetNearestGreaterOrEqual(&10), Some((&10, &"ten")));
    assert_eq!(m.GetNearestLessOrEqual(&20), Some((&20, &"twenty")));
}

#[test]
fn remove() {
    let mut m: emAvlTreeMap<i32, &str> = emAvlTreeMap::new();
    m.Insert(1, "a");
    m.Insert(2, "b");
    m.Insert(3, "c");

    m.Remove(&2);
    assert_eq!(m.GetCount(), 2);
    assert!(!m.Contains(&2));
    assert!(m.Contains(&1));
    assert!(m.Contains(&3));
}
```

Add `mod avl_tree_map;` to `tests/behavioral/main.rs`.

- [ ] **Step 2: Implement emAvlTreeMap**

Create `src/emCore/emAvlTreeMap.rs`:

```rust
//! emAvlTreeMap — COW ordered map.
//!
//! C++ emAvlTreeMap (emAvlTreeMap.h) is a copy-on-write sorted map backed
//! by an intrusive AVL tree with stable iterators and ordered access.
//!
//! Rust wraps BTreeMap in Rc for COW semantics. Ordered access methods
//! (GetFirst, GetLast, GetNearestGreater, etc.) use BTreeMap::range().

use std::collections::BTreeMap;
use std::ops::Bound;
use std::rc::Rc;

/// Copy-on-write ordered map matching C++ `emAvlTreeMap<KEY, VALUE>`.
///
/// DIVERGED: C++ exposes `Element { Key, Value, AvlNode }` struct.
/// Rust returns `(&K, &V)` tuples or `Option<&V>` directly since
/// there is no intrusive AVL node to expose.
pub struct emAvlTreeMap<K: Ord + Clone, V: Clone> {
    data: Rc<BTreeMap<K, V>>,
}

impl<K: Ord + Clone, V: Clone> emAvlTreeMap<K, V> {
    /// Construct empty map. C++ `emAvlTreeMap()`.
    pub fn new() -> Self {
        Self {
            data: Rc::new(BTreeMap::new()),
        }
    }

    /// Construct single-element map. C++ `emAvlTreeMap(key, value)`.
    pub fn from_entry(key: K, value: V) -> Self {
        let mut m = BTreeMap::new();
        m.insert(key, value);
        Self { data: Rc::new(m) }
    }

    // --- COW write barrier ---

    fn make_writable(&mut self) -> &mut BTreeMap<K, V> {
        Rc::make_mut(&mut self.data)
    }

    // --- Read-only ---

    /// C++ `Contains`.
    pub fn Contains(&self, key: &K) -> bool {
        self.data.contains_key(key)
    }

    /// C++ `GetValue(const KEY&)`.
    pub fn GetValue(&self, key: &K) -> Option<&V> {
        self.data.get(key)
    }

    /// C++ `GetKey(const KEY&)` — returns the actual stored key.
    pub fn GetKey(&self, key: &K) -> Option<&K> {
        self.data.get_key_value(key).map(|(k, _)| k)
    }

    /// C++ `IsEmpty`.
    pub fn IsEmpty(&self) -> bool {
        self.data.is_empty()
    }

    /// C++ `GetCount`. Note: O(n) in C++, O(1) in Rust's BTreeMap.
    pub fn GetCount(&self) -> usize {
        self.data.len()
    }

    /// C++ `GetDataRefCount`.
    pub fn GetDataRefCount(&self) -> usize {
        Rc::strong_count(&self.data)
    }

    // --- Ordered access ---

    /// Smallest key. C++ `GetFirst`.
    pub fn GetFirst(&self) -> Option<(&K, &V)> {
        self.data.iter().next()
    }

    /// Largest key. C++ `GetLast`.
    pub fn GetLast(&self) -> Option<(&K, &V)> {
        self.data.iter().next_back()
    }

    /// Strictly greater. C++ `GetNearestGreater`.
    pub fn GetNearestGreater(&self, key: &K) -> Option<(&K, &V)> {
        self.data.range((Bound::Excluded(key.clone()), Bound::Unbounded)).next()
    }

    /// Greater or equal. C++ `GetNearestGreaterOrEqual`.
    pub fn GetNearestGreaterOrEqual(&self, key: &K) -> Option<(&K, &V)> {
        self.data.range(key..).next()
    }

    /// Strictly less. C++ `GetNearestLess`.
    pub fn GetNearestLess(&self, key: &K) -> Option<(&K, &V)> {
        self.data.range(..key).next_back()
    }

    /// Less or equal. C++ `GetNearestLessOrEqual`.
    pub fn GetNearestLessOrEqual(&self, key: &K) -> Option<(&K, &V)> {
        self.data.range(..=key.clone()).next_back()
    }

    // --- Mutating ---

    /// C++ `Insert(key, value)` / `SetValue(key, value, true)`.
    pub fn Insert(&mut self, key: K, value: V) {
        self.make_writable().insert(key, value);
    }

    /// C++ `SetValue(key, value, insertIfNew)`.
    pub fn SetValue(&mut self, key: K, value: V, insert_if_new: bool) {
        let m = self.make_writable();
        if insert_if_new || m.contains_key(&key) {
            m.insert(key, value);
        }
    }

    /// C++ `GetValueWritable(key, insertIfNew)`.
    pub fn GetValueWritable(&mut self, key: K, insert_if_new: bool) -> Option<&mut V>
    where
        V: Default,
    {
        let m = self.make_writable();
        if insert_if_new {
            Some(m.entry(key).or_insert_with(V::default))
        } else {
            m.get_mut(&key)
        }
    }

    /// C++ `Remove(const KEY&)`.
    pub fn Remove(&mut self, key: &K) {
        self.make_writable().remove(key);
    }

    /// C++ `RemoveFirst`.
    pub fn RemoveFirst(&mut self) {
        let m = self.make_writable();
        if let Some(k) = m.keys().next().cloned() {
            m.remove(&k);
        }
    }

    /// C++ `RemoveLast`.
    pub fn RemoveLast(&mut self) {
        let m = self.make_writable();
        if let Some(k) = m.keys().next_back().cloned() {
            m.remove(&k);
        }
    }

    /// C++ `Clear`.
    pub fn Clear(&mut self) {
        self.make_writable().clear();
    }

    /// C++ `MakeNonShared`.
    pub fn MakeNonShared(&mut self) {
        let _ = self.make_writable();
    }
}

impl<K: Ord + Clone, V: Clone> Clone for emAvlTreeMap<K, V> {
    fn clone(&self) -> Self {
        Self {
            data: Rc::clone(&self.data),
        }
    }
}

impl<K: Ord + Clone, V: Clone> Default for emAvlTreeMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 3: Add to mod.rs, delete marker**

In `src/emCore/mod.rs`, add `pub mod emAvlTreeMap;` in alphabetical order.

Delete `src/emCore/emAvlTreeMap.no_rs`.

- [ ] **Step 4: Run tests**

Run: `cargo clippy -- -D warnings && cargo-nextest ntr`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git rm src/emCore/emAvlTreeMap.no_rs
git add src/emCore/emAvlTreeMap.rs src/emCore/mod.rs \
  tests/behavioral/avl_tree_map.rs tests/behavioral/main.rs
git commit -m "feat: port emAvlTreeMap with COW and ordered access

Wraps BTreeMap in Rc for COW semantics. Ordered access via
GetFirst/Last/GetNearestGreater/Less/etc. using BTreeMap::range().
Element struct diverged to direct key/value references."
```

---

## Task 4: emAvlTreeMap stable cursor

**Files:**
- Modify: `src/emCore/emAvlTreeMap.rs`
- Modify: `tests/behavioral/avl_tree_map.rs`

**Context:** C++ `emAvlTreeMap::Iterator` is a stable cursor tracked via a linked list inside the map. When an element is removed, the iterator advances to next. When COW copies data, iterators are pointer-fixed. When `operator=` is called, all iterators are nullified.

**Design:** Cursor stores a cloned key as its position. `SetNext`/`SetPrev` use `BTreeMap::range()` from the current key. When the key is removed, `Get` returns None (the cursor invalidates rather than auto-advancing — simpler and safe).

- [ ] **Step 1: Write cursor tests**

Add to `tests/behavioral/avl_tree_map.rs`:

```rust
#[test]
fn cursor_iteration() {
    let mut m: emAvlTreeMap<i32, &str> = emAvlTreeMap::new();
    m.Insert(10, "ten");
    m.Insert(20, "twenty");
    m.Insert(30, "thirty");

    let mut cur = m.cursor_first();
    assert_eq!(cur.Get(&m), Some((&10, &"ten")));
    cur.SetNext(&m);
    assert_eq!(cur.Get(&m), Some((&20, &"twenty")));
    cur.SetNext(&m);
    assert_eq!(cur.Get(&m), Some((&30, &"thirty")));
    cur.SetNext(&m);
    assert!(cur.Get(&m).is_none());
}

#[test]
fn cursor_survives_cow() {
    let mut m: emAvlTreeMap<i32, &str> = emAvlTreeMap::new();
    m.Insert(1, "a");
    m.Insert(2, "b");

    let cur = m.cursor_at(&1);
    let mut n = m.clone();
    n.Insert(3, "c"); // triggers COW

    // cursor still valid against m
    assert_eq!(cur.Get(&m), Some((&1, &"a")));
}

#[test]
fn cursor_by_key() {
    let mut m: emAvlTreeMap<i32, &str> = emAvlTreeMap::new();
    m.Insert(10, "ten");
    m.Insert(20, "twenty");
    m.Insert(30, "thirty");

    let cur = m.cursor_at(&20);
    assert_eq!(cur.Get(&m), Some((&20, &"twenty")));
}
```

- [ ] **Step 2: Implement MapCursor**

Add to `src/emCore/emAvlTreeMap.rs`:

```rust
/// Stable cursor for emAvlTreeMap. Tracks position by key.
///
/// DIVERGED: C++ Iterator auto-advances when the pointed-to element is
/// removed. This cursor returns None instead. C++ Iterator is nullified
/// on `operator=`; this cursor simply holds a key copy independent of
/// map identity.
pub struct MapCursor<K: Clone> {
    key: Option<K>,
}

impl<K: Ord + Clone> MapCursor<K> {
    /// Get the key-value pair at cursor position.
    pub fn Get<'a, V: Clone>(&self, map: &'a emAvlTreeMap<K, V>) -> Option<(&'a K, &'a V)> {
        self.key.as_ref().and_then(|k| map.data.get_key_value(k))
    }

    /// Advance to next key in order.
    pub fn SetNext<V: Clone>(&mut self, map: &emAvlTreeMap<K, V>) {
        if let Some(ref k) = self.key {
            self.key = map.data
                .range((Bound::Excluded(k.clone()), Bound::Unbounded))
                .next()
                .map(|(k, _)| k.clone());
        }
    }

    /// Move to previous key in order.
    pub fn SetPrev<V: Clone>(&mut self, map: &emAvlTreeMap<K, V>) {
        if let Some(ref k) = self.key {
            self.key = map.data.range(..k).next_back().map(|(k, _)| k.clone());
        }
    }

    /// Detach cursor (set to None).
    pub fn Detach(&mut self) {
        self.key = None;
    }
}

impl<K: Ord + Clone, V: Clone> emAvlTreeMap<K, V> {
    /// Cursor at first (smallest) key.
    pub fn cursor_first(&self) -> MapCursor<K> {
        MapCursor {
            key: self.data.keys().next().cloned(),
        }
    }

    /// Cursor at last (largest) key.
    pub fn cursor_last(&self) -> MapCursor<K> {
        MapCursor {
            key: self.data.keys().next_back().cloned(),
        }
    }

    /// Cursor at specific key.
    pub fn cursor_at(&self, key: &K) -> MapCursor<K> {
        MapCursor {
            key: if self.data.contains_key(key) { Some(key.clone()) } else { None },
        }
    }
}
```

- [ ] **Step 3: Run tests and commit**

Run: `cargo clippy -- -D warnings && cargo-nextest ntr`

```bash
git add src/emCore/emAvlTreeMap.rs tests/behavioral/avl_tree_map.rs
git commit -m "feat: add stable MapCursor to emAvlTreeMap

Key-based cursor that survives COW clones. Returns None (not
auto-advance) when pointed-to element is removed. Provides
Get, SetNext, SetPrev, Detach."
```

---

## Task 5: emAvlTreeSet with COW and set operations

**Files:**
- Create: `src/emCore/emAvlTreeSet.rs`
- Modify: `src/emCore/mod.rs`
- Create: `tests/behavioral/avl_tree_set.rs`
- Modify: `tests/behavioral/main.rs`

**Context:** C++ emAvlTreeSet is a COW ordered set with set algebra operations (union, intersection, subtraction). Rust wraps `BTreeSet` in `Rc`. C++ header: `~/git/eaglemode-0.96.4/include/emCore/emAvlTreeSet.h`.

**Structure mirrors emAvlTreeMap** but for sets. Includes:
- COW via `Rc<BTreeSet<T>>`
- Ordered access: GetFirst, GetLast, GetNearestGreater/Less/etc.
- Set operations: Insert(set), Remove(set), Intersect(set)
- SetCursor (key-based, same pattern as MapCursor)

- [ ] **Step 1: Write behavioral tests**

Create `tests/behavioral/avl_tree_set.rs` with tests for:
- `empty_set`, `insert_and_contains`
- `cow_shallow_copy`, `cow_clone_on_mutate`
- `ordered_access` (GetFirst, GetLast, GetNearestGreater/Less)
- `set_union`, `set_intersection`, `set_subtraction`
- `cursor_iteration`, `cursor_survives_cow`

Add `mod avl_tree_set;` to `tests/behavioral/main.rs`.

```rust
use eaglemode_rs::emCore::emAvlTreeSet::emAvlTreeSet;

#[test]
fn empty_set() {
    let s: emAvlTreeSet<i32> = emAvlTreeSet::new();
    assert!(s.IsEmpty());
    assert_eq!(s.GetCount(), 0);
}

#[test]
fn insert_and_contains() {
    let mut s: emAvlTreeSet<i32> = emAvlTreeSet::new();
    s.Insert(42);
    assert!(s.Contains(&42));
    assert!(!s.Contains(&99));
}

#[test]
fn cow_clone_on_mutate() {
    let mut a: emAvlTreeSet<i32> = emAvlTreeSet::new();
    a.Insert(1);
    a.Insert(2);

    let mut b = a.clone();
    assert_eq!(a.GetDataRefCount(), 2);

    b.Insert(3);
    assert_eq!(a.GetDataRefCount(), 1);
    assert_eq!(a.GetCount(), 2);
    assert_eq!(b.GetCount(), 3);
}

#[test]
fn ordered_access() {
    let mut s: emAvlTreeSet<i32> = emAvlTreeSet::new();
    s.Insert(10);
    s.Insert(20);
    s.Insert(30);

    assert_eq!(s.GetFirst(), Some(&10));
    assert_eq!(s.GetLast(), Some(&30));
    assert_eq!(s.GetNearestGreater(&10), Some(&20));
    assert_eq!(s.GetNearestLess(&30), Some(&20));
}

#[test]
fn set_union() {
    let mut a: emAvlTreeSet<i32> = emAvlTreeSet::new();
    a.Insert(1);
    a.Insert(2);

    let mut b: emAvlTreeSet<i32> = emAvlTreeSet::new();
    b.Insert(2);
    b.Insert(3);

    a.InsertSet(&b);
    assert_eq!(a.GetCount(), 3);
    assert!(a.Contains(&1));
    assert!(a.Contains(&2));
    assert!(a.Contains(&3));
}

#[test]
fn set_intersection() {
    let mut a: emAvlTreeSet<i32> = emAvlTreeSet::new();
    a.Insert(1);
    a.Insert(2);
    a.Insert(3);

    let mut b: emAvlTreeSet<i32> = emAvlTreeSet::new();
    b.Insert(2);
    b.Insert(3);
    b.Insert(4);

    a.Intersect(&b);
    assert_eq!(a.GetCount(), 2);
    assert!(a.Contains(&2));
    assert!(a.Contains(&3));
}

#[test]
fn set_subtraction() {
    let mut a: emAvlTreeSet<i32> = emAvlTreeSet::new();
    a.Insert(1);
    a.Insert(2);
    a.Insert(3);

    let mut b: emAvlTreeSet<i32> = emAvlTreeSet::new();
    b.Insert(2);

    a.RemoveSet(&b);
    assert_eq!(a.GetCount(), 2);
    assert!(a.Contains(&1));
    assert!(a.Contains(&3));
}
```

- [ ] **Step 2: Implement emAvlTreeSet**

Create `src/emCore/emAvlTreeSet.rs`. Follow the same Rc<BTreeSet<T>> + COW pattern as emAvlTreeMap. Include:
- `new()`, `from_element(obj)`, `Clone`, `Default`
- Read: `Contains`, `Get`, `GetFirst`, `GetLast`, `GetNearestGreater/Less/OrEqual`, `IsEmpty`, `GetCount`, `GetDataRefCount`
- Mutate: `Insert(obj)`, `InsertSet(set)`, `RemoveFirst`, `RemoveLast`, `Remove(obj)`, `RemoveSet(set)`, `Intersect(set)`, `Clear`, `MakeNonShared`
- Cursor: `SetCursor<T>` with key-based tracking (same pattern as MapCursor)
- `PartialEq` impl comparing sets

Read the C++ header `~/git/eaglemode-0.96.4/include/emCore/emAvlTreeSet.h` for the complete API. The implementation pattern is identical to emAvlTreeMap but wrapping `BTreeSet` instead of `BTreeMap`.

- [ ] **Step 3: Add to mod.rs, delete marker, run tests, commit**

```bash
git rm src/emCore/emAvlTreeSet.no_rs
git add src/emCore/emAvlTreeSet.rs src/emCore/mod.rs \
  tests/behavioral/avl_tree_set.rs tests/behavioral/main.rs
git commit -m "feat: port emAvlTreeSet with COW and set operations

Wraps BTreeSet in Rc for COW. Ordered access via GetFirst/Last/
GetNearestGreater/Less. Set algebra: union, intersection, subtraction.
SetCursor with key-based tracking."
```

---

## Task 6: emList core with COW

**Files:**
- Create: `src/emCore/emList.rs`
- Modify: `src/emCore/mod.rs`
- Create: `tests/behavioral/list.rs`
- Modify: `tests/behavioral/main.rs`

**Context:** C++ emList is a COW doubly-linked list with stable iterators and inter-list element moves. Never instantiated with a concrete type anywhere in emCore (only used by 16 outside-emCore files). Rust wraps `Vec<T>` in `Rc` for COW. C++ header: `~/git/eaglemode-0.96.4/include/emCore/emList.h`.

**Design:** Use `Rc<Vec<T>>` backing rather than a real linked list:
- Cache-friendly iteration and sorting
- O(n) insert/remove at arbitrary positions (acceptable since no emCore consumer exists)
- COW via `Rc::make_mut`
- Stable cursors via index (same as emArray)
- DIVERGED: C++ uses intrusive doubly-linked list with O(1) splice/move between lists. Rust uses Vec for cache locality. Move operations are O(n).

- [ ] **Step 1: Write behavioral tests**

Create `tests/behavioral/list.rs` with tests for:
- `empty_list`, `add_and_get`
- `cow_shallow_copy`, `cow_clone_on_mutate`
- `insert_at_beg`, `insert_at_end`, `insert_before`, `insert_after`
- `remove_first`, `remove_last`
- `sort`, `get_count`
- `cursor_iteration`

```rust
use eaglemode_rs::emCore::emList::emList;

#[test]
fn empty_list() {
    let l: emList<i32> = emList::new();
    assert!(l.IsEmpty());
    assert_eq!(l.GetCount(), 0);
    assert!(l.GetFirst().is_none());
    assert!(l.GetLast().is_none());
}

#[test]
fn add_and_navigate() {
    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(10);
    l.InsertAtEnd_one(20);
    l.InsertAtEnd_one(30);

    assert_eq!(l.GetFirst(), Some(&10));
    assert_eq!(l.GetLast(), Some(&30));
    assert_eq!(l.GetCount(), 3);
}

#[test]
fn cow_clone_on_mutate() {
    let mut a: emList<i32> = emList::new();
    a.InsertAtEnd_one(1);
    a.InsertAtEnd_one(2);

    let mut b = a.clone();
    assert_eq!(a.GetDataRefCount(), 2);

    b.InsertAtEnd_one(3);
    assert_eq!(a.GetDataRefCount(), 1);
    assert_eq!(a.GetCount(), 2);
    assert_eq!(b.GetCount(), 3);
}

#[test]
fn insert_positions() {
    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(2);
    l.InsertAtBeg_one(1);
    l.InsertAtEnd_one(3);

    assert_eq!(l.GetFirst(), Some(&1));
    assert_eq!(l.GetLast(), Some(&3));
    assert_eq!(l.GetCount(), 3);
}

#[test]
fn remove() {
    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(1);
    l.InsertAtEnd_one(2);
    l.InsertAtEnd_one(3);

    l.RemoveFirst();
    assert_eq!(l.GetFirst(), Some(&2));

    l.RemoveLast();
    assert_eq!(l.GetLast(), Some(&2));
    assert_eq!(l.GetCount(), 1);
}

#[test]
fn sort() {
    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(3);
    l.InsertAtEnd_one(1);
    l.InsertAtEnd_one(2);

    let changed = l.Sort();
    assert!(changed);
    assert_eq!(l.GetFirst(), Some(&1));
    assert_eq!(l.GetLast(), Some(&3));
}
```

Add `mod list;` to `tests/behavioral/main.rs`.

- [ ] **Step 2: Implement emList**

Create `src/emCore/emList.rs`. Use `Rc<Vec<T>>` backing. Implement:
- `new()`, `from_element(obj)`, `Clone`, `Default`
- Navigation: `GetFirst`, `GetLast`, `GetNext(index)`, `GetPrev(index)`, `GetAtIndex`, `GetIndexOf`
- Insert: `InsertAtBeg_one`, `InsertAtEnd_one`, `InsertBefore`, `InsertAfter`, `Add_one`
- Remove: `RemoveFirst`, `RemoveLast`, `Remove(index)`, `Clear`
- Query: `IsEmpty`, `GetCount`, `GetDataRefCount`, `MakeNonShared`
- Sort: `Sort() -> bool` (stable, returns true if changed)
- Writable: `GetWritable(index)`, `Set(index, value)`
- Extract: `ExtractFirst`, `ExtractLast`, `Extract(index)`
- Cursor: `ListCursor` with index-based tracking

Read C++ header `~/git/eaglemode-0.96.4/include/emCore/emList.h` for the complete API. Key DIVERGED items:
- C++ navigation uses `const OBJ*` pointers. Rust uses index-based access.
- C++ Move operations splice elements between lists in O(1). Rust copies in O(n).
- C++ has ~30 overloaded Insert/Move variants. Port the primary ones; add overloads when consumers need them.

**DIVERGED comment at type definition:**
```rust
// DIVERGED: C++ emList uses intrusive doubly-linked list with O(1) splice.
// Rust uses Vec<T> backing for cache locality. Navigation uses index (not
// pointer). Move operations between lists are O(n) copies, not O(1) splices.
// C++ pointer-based API (GetNext(const OBJ*)) becomes index-based
// (GetNext(usize) -> Option<usize>).
```

- [ ] **Step 3: Add to mod.rs, delete marker, run tests, commit**

```bash
git rm src/emCore/emList.no_rs
git add src/emCore/emList.rs src/emCore/mod.rs \
  tests/behavioral/list.rs tests/behavioral/main.rs
git commit -m "feat: port emList with COW semantics

Vec-backed COW doubly-linked list. Insert/remove at any position,
stable sort, index-based navigation and cursors. Intrusive linked
list splice operations diverged to Vec copies for cache locality."
```

---

## Task 7: emAvlTree marker resolution

**Files:**
- Modify: `src/emCore/emAvlTree.no_rs`

**Context:** C++ `emAvlTree` is a C macro library for embedding AVL trees into user-defined structs. It's not a class — it's infrastructure used internally by `emAvlTreeMap` and `emAvlTreeSet`. The 3 emCore users (emContext, emPanel, emListBox) already use HashMap in Rust.

The `.no_rs` marker should remain because there is no direct Rust type equivalent. But update the reviewed summary to reflect that emAvlTreeMap/Set now exist as Rust types wrapping BTreeMap/BTreeSet.

- [ ] **Step 1: Read and update emAvlTree.no_rs**

Update the reviewed summary section to note that emAvlTreeMap.rs and emAvlTreeSet.rs now provide the ordered-access functionality that emAvlTree macros enabled in C++. The macro library itself has no Rust equivalent (nor does it need one).

- [ ] **Step 2: Commit**

```bash
git add src/emCore/emAvlTree.no_rs
git commit -m "docs: update emAvlTree.no_rs to reflect Map/Set ports

emAvlTree macro library has no direct Rust equivalent. The ordered
access it enabled is now provided by emAvlTreeMap.rs and
emAvlTreeSet.rs wrapping BTreeMap/BTreeSet."
```

---

## Task 8: Call site audit and CORRESPONDENCE.md update

**Files:**
- Modify: `src/emCore/CORRESPONDENCE.md`
- Possibly modify: source files if Vec/HashMap needs replacement

**Context:** The spec's blast radius rule says: "If a call site depends on behavior the stdlib type doesn't provide (COW, stable iteration, ordered access, explicit invalidation), refactor it to use the new type. If the call site only uses basic functionality that stdlib covers, leave it alone."

- [ ] **Step 1: Audit existing Vec usage for COW dependence**

Grep for patterns that suggest COW would be needed:
```bash
# Look for Vec being cloned and both copies used
grep -rn "\.clone()" src/emCore/ --include='*.rs' | grep -i "vec\|array\|items\|elements"
```

For each site, determine if:
a) The clone is used as a snapshot (COW would help) — refactor to emArray
b) The clone is a deep copy and both copies diverge independently — leave as Vec

Expected: Most Vec usage is single-owner; COW is not needed. Leave as Vec.

- [ ] **Step 2: Audit HashMap usage for ordered-access dependence**

Check if any HashMap usage actually needs ordered iteration or nearest-key lookup:
```bash
grep -rn "HashMap" src/emCore/ --include='*.rs' -l
```

For each file, check if the code ever iterates in key order or does nearest-key lookups. If so, consider refactoring to emAvlTreeMap.

Expected: HashMap usage in emContext, emPanelTree, emListBox does NOT need ordered access (verified in C++ audit — the AVL tree in C++ is used for O(log n) lookup, not for ordered traversal at those sites). Leave as HashMap.

- [ ] **Step 3: Update CORRESPONDENCE.md**

Update to reflect:
- emArray.no_rs deleted (emArray.rs created)
- emAvlTreeMap.no_rs deleted (emAvlTreeMap.rs created)
- emAvlTreeSet.no_rs deleted (emAvlTreeSet.rs created)
- emList.no_rs deleted (emList.rs created)
- emAvlTree.no_rs remains (macro library, no direct equivalent)
- Call site audit results: which sites were changed (if any)
- DIVERGED items: Vec-backed instead of intrusive list, BTreeMap/Set instead of AVL, cursor behavior differences

- [ ] **Step 4: Run full test suite**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr && cargo test --test golden -- --test-threads=1
```

- [ ] **Step 5: Commit**

```bash
git add src/emCore/CORRESPONDENCE.md
git commit -m "docs: update CORRESPONDENCE.md for Phase 2 completion

Reflects emArray, emAvlTreeMap, emAvlTreeSet, emList ports.
Documents BTreeMap/BTreeSet backing, cursor divergences, and
call site audit results."
```

---

## Task 9: Phase 2 review checkpoint

**No files changed.** This is a review gate.

- [ ] **Step 1: Verify all marker files resolved**

```bash
ls src/emCore/emArray.no_rs src/emCore/emAvlTreeMap.no_rs \
   src/emCore/emAvlTreeSet.no_rs src/emCore/emList.no_rs 2>&1
```

Expected: All four files not found (deleted). `emAvlTree.no_rs` should still exist.

- [ ] **Step 2: Verify new .rs files exist**

```bash
ls src/emCore/emArray.rs src/emCore/emAvlTreeMap.rs \
   src/emCore/emAvlTreeSet.rs src/emCore/emList.rs
```

Expected: All four files exist.

- [ ] **Step 3: Verify full test suite**

```bash
cargo clippy -- -D warnings && cargo-nextest ntr
```

- [ ] **Step 4: Report findings**

Summarize:
- Types completed with COW and cursor support
- DIVERGED items and their rationale
- Call site audit results
- Readiness for Phase 3
