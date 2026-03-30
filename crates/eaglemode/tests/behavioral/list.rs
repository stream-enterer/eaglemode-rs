use emcore::emList::emList;

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

#[test]
fn sort_already_sorted() {
    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(1);
    l.InsertAtEnd_one(2);
    l.InsertAtEnd_one(3);

    let changed = l.Sort();
    assert!(!changed);
}

#[test]
fn navigation_by_index() {
    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(10);
    l.InsertAtEnd_one(20);
    l.InsertAtEnd_one(30);

    assert_eq!(l.GetAtIndex(0), Some(&10));
    assert_eq!(l.GetAtIndex(1), Some(&20));
    assert_eq!(l.GetAtIndex(2), Some(&30));
    assert_eq!(l.GetAtIndex(3), None);

    assert_eq!(l.GetNext(0), Some((1, &20)));
    assert_eq!(l.GetNext(2), None);
    assert_eq!(l.GetPrev(2), Some((1, &20)));
    assert_eq!(l.GetPrev(0), None);
}

#[test]
fn writable_and_set() {
    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(1);
    l.InsertAtEnd_one(2);
    l.InsertAtEnd_one(3);

    *l.GetWritable(1) = 42;
    assert_eq!(l.GetAtIndex(1), Some(&42));

    l.Set(0, 99);
    assert_eq!(l.GetFirst(), Some(&99));
}

#[test]
fn extract() {
    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(1);
    l.InsertAtEnd_one(2);
    l.InsertAtEnd_one(3);

    assert_eq!(l.ExtractFirst(), Some(1));
    assert_eq!(l.GetCount(), 2);
    assert_eq!(l.ExtractLast(), Some(3));
    assert_eq!(l.GetCount(), 1);
    assert_eq!(l.GetFirst(), Some(&2));
}

#[test]
fn clear() {
    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(1);
    l.InsertAtEnd_one(2);
    l.Clear();
    assert!(l.IsEmpty());
    assert_eq!(l.GetCount(), 0);
}

#[test]
fn add_alias() {
    let mut l: emList<i32> = emList::new();
    l.Add_one(10);
    l.Add_one(20);
    assert_eq!(l.GetCount(), 2);
    assert_eq!(l.GetFirst(), Some(&10));
    assert_eq!(l.GetLast(), Some(&20));
}

#[test]
fn from_element() {
    let l: emList<i32> = emList::from_element(42);
    assert_eq!(l.GetCount(), 1);
    assert_eq!(l.GetFirst(), Some(&42));
}

#[test]
fn remove_at_index() {
    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(10);
    l.InsertAtEnd_one(20);
    l.InsertAtEnd_one(30);

    l.Remove(1);
    assert_eq!(l.GetCount(), 2);
    assert_eq!(l.GetFirst(), Some(&10));
    assert_eq!(l.GetLast(), Some(&30));
}

#[test]
fn make_non_shared() {
    let mut a: emList<i32> = emList::new();
    a.InsertAtEnd_one(1);
    let _b = a.clone();
    assert_eq!(a.GetDataRefCount(), 2);
    a.MakeNonShared();
    assert_eq!(a.GetDataRefCount(), 1);
}

#[test]
fn cursor_basic() {
    use emcore::emList::ListCursor;

    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(10);
    l.InsertAtEnd_one(20);
    l.InsertAtEnd_one(30);

    let mut c = l.cursor_first();
    assert!(c.IsValid(&l));
    assert_eq!(c.Get(&l), Some(&10));

    c.SetNext(&l);
    assert_eq!(c.Get(&l), Some(&20));

    c.SetNext(&l);
    assert_eq!(c.Get(&l), Some(&30));

    c.SetNext(&l);
    assert!(!c.IsValid(&l));

    let mut c2 = l.cursor_last();
    assert_eq!(c2.Get(&l), Some(&30));
    c2.SetPrev(&l);
    assert_eq!(c2.Get(&l), Some(&20));

    let c3 = l.cursor_at(1);
    assert_eq!(c3.Get(&l), Some(&20));
}

#[test]
fn cursor_detach() {
    use emcore::emList::ListCursor;

    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(1);

    let mut c = l.cursor_first();
    assert!(c.IsValid(&l));
    c.Detach();
    assert!(!c.IsValid(&l));
    assert_eq!(c.Get(&l), None);
}

#[test]
fn insert_before_after() {
    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(1);
    l.InsertAtEnd_one(3);

    l.InsertBefore(1, 2);
    assert_eq!(l.GetCount(), 3);
    assert_eq!(l.GetAtIndex(0), Some(&1));
    assert_eq!(l.GetAtIndex(1), Some(&2));
    assert_eq!(l.GetAtIndex(2), Some(&3));

    l.InsertAfter(0, 15);
    assert_eq!(l.GetCount(), 4);
    assert_eq!(l.GetAtIndex(1), Some(&15));
}

#[test]
fn get_index_of() {
    let mut l: emList<i32> = emList::new();
    l.InsertAtEnd_one(10);
    l.InsertAtEnd_one(20);
    l.InsertAtEnd_one(30);

    // GetIndexOf finds by value match on the data vec
    assert_eq!(l.GetIndexOf(&20), Some(1));
    assert_eq!(l.GetIndexOf(&99), None);
}

#[test]
fn default_is_empty() {
    let l: emList<i32> = emList::default();
    assert!(l.IsEmpty());
}
