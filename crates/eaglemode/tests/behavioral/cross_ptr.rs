use std::cell::RefCell;
use std::rc::Rc;

use emcore::emCrossPtr::{emCrossPtr, emCrossPtrList};

#[test]
fn null_cross_ptr() {
    let ptr: emCrossPtr<String> = emCrossPtr::new();
    assert!(!ptr.is_valid());
    assert!(ptr.Get().is_none());
}

#[test]
fn link_and_access() {
    let target = Rc::new(RefCell::new(42_i32));
    let mut list = emCrossPtrList::new();
    let ptr = emCrossPtr::from_target(&target, &mut list);
    assert!(ptr.is_valid());
    let got = ptr.Get().expect("should return target");
    assert_eq!(*got.borrow(), 42);
}

#[test]
fn break_cross_ptrs_invalidates() {
    let target = Rc::new(RefCell::new("hello".to_string()));
    let mut list = emCrossPtrList::new();
    let ptr = emCrossPtr::from_target(&target, &mut list);
    assert!(ptr.is_valid());

    list.BreakCrossPtrs();

    // Target is still alive (we hold an Rc), but cross pointer is invalidated.
    assert_eq!(Rc::strong_count(&target), 1);
    assert!(!ptr.is_valid());
    assert!(ptr.Get().is_none());
}

#[test]
fn multiple_ptrs_all_invalidated() {
    let target = Rc::new(RefCell::new(99_u32));
    let mut list = emCrossPtrList::new();
    let ptr1 = emCrossPtr::from_target(&target, &mut list);
    let ptr2 = emCrossPtr::from_target(&target, &mut list);
    let ptr3 = emCrossPtr::from_target(&target, &mut list);

    assert!(ptr1.is_valid());
    assert!(ptr2.is_valid());
    assert!(ptr3.is_valid());

    list.BreakCrossPtrs();

    assert!(!ptr1.is_valid());
    assert!(!ptr2.is_valid());
    assert!(!ptr3.is_valid());
}

#[test]
fn rebind_to_different_target() {
    let target_a = Rc::new(RefCell::new(1_i32));
    let target_b = Rc::new(RefCell::new(2_i32));
    let mut list_a = emCrossPtrList::new();
    let mut list_b = emCrossPtrList::new();

    let mut ptr = emCrossPtr::from_target(&target_a, &mut list_a);
    assert_eq!(*ptr.Get().expect("valid").borrow(), 1);

    ptr.Set(&target_b, &mut list_b);
    assert_eq!(*ptr.Get().expect("valid").borrow(), 2);

    // Breaking old list does NOT affect new binding.
    list_a.BreakCrossPtrs();
    assert!(ptr.is_valid());
    assert_eq!(*ptr.Get().expect("still valid").borrow(), 2);

    // Breaking new list DOES affect the pointer.
    list_b.BreakCrossPtrs();
    assert!(!ptr.is_valid());
}

#[test]
fn drop_list_breaks_ptrs() {
    let target = Rc::new(RefCell::new(7_u8));
    let ptr;
    {
        let mut list = emCrossPtrList::new();
        ptr = emCrossPtr::from_target(&target, &mut list);
        assert!(ptr.is_valid());
        // list drops here
    }
    assert!(!ptr.is_valid());
    assert!(ptr.Get().is_none());
}

#[test]
fn reset_unbinds() {
    let target = Rc::new(RefCell::new(10_i32));
    let mut list = emCrossPtrList::new();
    let mut ptr = emCrossPtr::from_target(&target, &mut list);
    assert!(ptr.is_valid());

    ptr.Reset();
    assert!(!ptr.is_valid());
    assert!(ptr.Get().is_none());
}
