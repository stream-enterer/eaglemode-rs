// emModel.h: abstract base class for named/registered models.
//
// In C++, emModel inherits emEngine and provides name-based registration
// in emContext, common lifetime management, and type-erased lookup.
//
// In Rust, this functionality is absorbed into Context (context.rs):
// Context::register() / Context::lookup() handle registration,
// Rc<RefCell<T>> replaces C++ ref-counting.
//
// This file exists for 1:1 header correspondence.
