use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A tree node for service/singleton lookup.
///
/// Typed singletons (e.g. `Clipboard`, `CoreConfig`) are added as
/// `RefCell<Option<Rc<T>>>` fields with getter methods that walk the parent
/// chain (inherited lookup). Dynamic resources use `ResourceCache<V>` stored
/// as typed singletons.
///
/// Children are stored as `Weak` references to avoid memory leaks.
/// The child `Rc` is owned by whoever created it (typically a View or Panel).
pub struct Context {
    parent: Option<Weak<Context>>,
    children: RefCell<Vec<Weak<Context>>>,
    // Typed singletons will be added here as the framework develops.
    // Example:
    //   clipboard: RefCell<Option<Rc<Clipboard>>>,
    //   core_config: RefCell<Option<Rc<CoreConfig>>>,
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
        parent.children.borrow_mut().push(Rc::downgrade(&child));
        child
    }

    pub fn parent(&self) -> Option<Rc<Context>> {
        self.parent.as_ref().and_then(|w| w.upgrade())
    }

    /// Number of live children (expired weak references are not counted).
    pub fn child_count(&self) -> usize {
        self.children.borrow().iter().filter(|w| w.strong_count() > 0).count()
    }

    /// Purge expired weak references from the children list.
    pub fn purge_dead_children(&self) {
        self.children.borrow_mut().retain(|w| w.strong_count() > 0);
    }
}
