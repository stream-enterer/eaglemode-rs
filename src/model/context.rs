use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A tree node for service/singleton lookup.
///
/// # Typed-singleton pattern (for later phases)
///
/// Each concrete singleton (e.g. `Clipboard`, `CoreConfig`) will be added as:
///
/// ```ignore
/// // Field on Context:
/// clipboard: RefCell<Option<Rc<Clipboard>>>,
///
/// // Getter walking parent chain:
/// pub fn clipboard(&self) -> Option<Rc<Clipboard>> {
///     if let Some(val) = self.clipboard.borrow().clone() {
///         return Some(val);
///     }
///     self.parent().and_then(|p| p.clipboard())
/// }
///
/// // Setter:
/// pub fn set_clipboard(&self, val: Rc<Clipboard>) {
///     *self.clipboard.borrow_mut() = Some(val);
/// }
/// ```
pub struct Context {
    parent: Option<Weak<Context>>,
    children: RefCell<Vec<Rc<Context>>>,
}

impl Context {
    pub fn new_root() -> Rc<Self> {
        Rc::new(Self {
            parent: None,
            children: RefCell::new(Vec::new()),
        })
    }

    pub fn new_child(parent: &Rc<Context>) -> Rc<Self> {
        let child = Rc::new(Self {
            parent: Some(Rc::downgrade(parent)),
            children: RefCell::new(Vec::new()),
        });
        parent.children.borrow_mut().push(child.clone());
        child
    }

    pub fn parent(&self) -> Option<Rc<Context>> {
        self.parent.as_ref().and_then(|w| w.upgrade())
    }

    pub fn child_count(&self) -> usize {
        self.children.borrow().len()
    }
}
