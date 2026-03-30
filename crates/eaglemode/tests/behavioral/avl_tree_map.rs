use emcore::emAvlTreeMap::emAvlTreeMap;

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
    assert_eq!(m.GetCount(), 1);
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
fn ordered_access_edge_cases() {
    let mut m: emAvlTreeMap<i32, &str> = emAvlTreeMap::new();
    m.Insert(10, "ten");
    m.Insert(20, "twenty");

    // No element greater than max
    assert_eq!(m.GetNearestGreater(&20), None);
    // No element less than min
    assert_eq!(m.GetNearestLess(&10), None);
    // GreaterOrEqual on exact match at max
    assert_eq!(m.GetNearestGreaterOrEqual(&20), Some((&20, &"twenty")));
    // LessOrEqual on exact match at min
    assert_eq!(m.GetNearestLessOrEqual(&10), Some((&10, &"ten")));
    // Greater than non-existent key between elements
    assert_eq!(m.GetNearestGreater(&15), Some((&20, &"twenty")));
    // Less than non-existent key between elements
    assert_eq!(m.GetNearestLess(&15), Some((&10, &"ten")));
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

#[test]
fn remove_first_last() {
    let mut m: emAvlTreeMap<i32, &str> = emAvlTreeMap::new();
    m.Insert(1, "a");
    m.Insert(2, "b");
    m.Insert(3, "c");

    m.RemoveFirst();
    assert_eq!(m.GetCount(), 2);
    assert!(!m.Contains(&1));

    m.RemoveLast();
    assert_eq!(m.GetCount(), 1);
    assert!(!m.Contains(&3));
    assert!(m.Contains(&2));
}

#[test]
fn clear() {
    let mut m: emAvlTreeMap<i32, i32> = emAvlTreeMap::new();
    m.Insert(1, 10);
    m.Insert(2, 20);
    m.Clear();
    assert!(m.IsEmpty());
    assert_eq!(m.GetCount(), 0);
}

#[test]
fn set_value_insert_if_new() {
    let mut m: emAvlTreeMap<i32, String> = emAvlTreeMap::new();

    // insert_if_new = true, key doesn't exist => inserts
    m.SetValue(1, "one".to_string(), true);
    assert_eq!(m.GetValue(&1), Some(&"one".to_string()));

    // insert_if_new = true, key exists => updates
    m.SetValue(1, "ONE".to_string(), true);
    assert_eq!(m.GetValue(&1), Some(&"ONE".to_string()));

    // insert_if_new = false, key doesn't exist => no-op
    m.SetValue(2, "two".to_string(), false);
    assert!(!m.Contains(&2));

    // insert_if_new = false, key exists => updates
    m.SetValue(1, "uno".to_string(), false);
    assert_eq!(m.GetValue(&1), Some(&"uno".to_string()));
}

#[test]
fn get_value_writable() {
    let mut m: emAvlTreeMap<i32, String> = emAvlTreeMap::new();

    // insert_if_new = true, key doesn't exist => inserts default
    {
        let v = m.GetValueWritable(&1, true);
        assert!(v.is_some());
        *v.unwrap() = "one".to_string();
    }
    assert_eq!(m.GetValue(&1), Some(&"one".to_string()));

    // insert_if_new = false, key doesn't exist => None
    assert!(m.GetValueWritable(&2, false).is_none());

    // insert_if_new = false, key exists => Some
    {
        let v = m.GetValueWritable(&1, false);
        assert!(v.is_some());
        *v.unwrap() = "ONE".to_string();
    }
    assert_eq!(m.GetValue(&1), Some(&"ONE".to_string()));
}

#[test]
fn get_key() {
    let mut m: emAvlTreeMap<String, i32> = emAvlTreeMap::new();
    m.Insert("hello".to_string(), 42);
    assert_eq!(m.GetKey(&"hello".to_string()), Some(&"hello".to_string()));
    assert_eq!(m.GetKey(&"world".to_string()), None);
}

#[test]
fn make_non_shared() {
    let mut m: emAvlTreeMap<i32, i32> = emAvlTreeMap::new();
    m.Insert(1, 10);
    let _n = m.clone();
    assert_eq!(m.GetDataRefCount(), 2);
    m.MakeNonShared();
    assert_eq!(m.GetDataRefCount(), 1);
}

#[test]
fn from_entry() {
    let m = emAvlTreeMap::from_entry(42, "answer");
    assert_eq!(m.GetCount(), 1);
    assert_eq!(m.GetValue(&42), Some(&"answer"));
}

#[test]
fn default_is_empty() {
    let m: emAvlTreeMap<i32, i32> = Default::default();
    assert!(m.IsEmpty());
}

#[test]
fn empty_map_ordered_access() {
    let m: emAvlTreeMap<i32, i32> = emAvlTreeMap::new();
    assert_eq!(m.GetFirst(), None);
    assert_eq!(m.GetLast(), None);
    assert_eq!(m.GetNearestGreater(&0), None);
    assert_eq!(m.GetNearestLess(&0), None);
}

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

#[test]
fn cursor_detach() {
    let mut m: emAvlTreeMap<i32, &str> = emAvlTreeMap::new();
    m.Insert(1, "a");

    let mut cur = m.cursor_first();
    assert!(cur.Get(&m).is_some());
    cur.Detach();
    assert!(cur.Get(&m).is_none());
}

#[test]
fn cursor_missing_key_returns_none() {
    let m: emAvlTreeMap<i32, &str> = emAvlTreeMap::new();
    let cur = m.cursor_at(&99);
    assert!(cur.Get(&m).is_none());
}

#[test]
fn cursor_set_prev() {
    let mut m: emAvlTreeMap<i32, &str> = emAvlTreeMap::new();
    m.Insert(10, "ten");
    m.Insert(20, "twenty");
    m.Insert(30, "thirty");

    let mut cur = m.cursor_last();
    assert_eq!(cur.Get(&m), Some((&30, &"thirty")));
    cur.SetPrev(&m);
    assert_eq!(cur.Get(&m), Some((&20, &"twenty")));
    cur.SetPrev(&m);
    assert_eq!(cur.Get(&m), Some((&10, &"ten")));
    cur.SetPrev(&m);
    assert!(cur.Get(&m).is_none());
}

#[test]
fn cursor_key_removed_returns_none() {
    let mut m: emAvlTreeMap<i32, &str> = emAvlTreeMap::new();
    m.Insert(1, "a");
    m.Insert(2, "b");

    let cur = m.cursor_at(&1);
    assert_eq!(cur.Get(&m), Some((&1, &"a")));
    m.Remove(&1);
    // cursor returns None — no auto-advance (DIVERGED from C++ Iterator)
    assert!(cur.Get(&m).is_none());
}
