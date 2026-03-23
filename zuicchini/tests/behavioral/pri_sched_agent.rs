use std::cell::RefCell;
use std::rc::Rc;

use zuicchini::emCore::emPriSchedAgent::PriSchedModel;
use zuicchini::emCore::emScheduler::EngineScheduler;

#[test]
fn highest_priority_gets_access_first() {
    let mut sched = EngineScheduler::new();
    let mut model = PriSchedModel::new(&mut sched);

    let got_a = Rc::new(RefCell::new(false));
    let got_b = Rc::new(RefCell::new(false));
    let got_c = Rc::new(RefCell::new(false));

    let ga = Rc::clone(&got_a);
    let agent_a = model.add_agent(1.0, Box::new(move || *ga.borrow_mut() = true));
    let gb = Rc::clone(&got_b);
    let agent_b = model.add_agent(5.0, Box::new(move || *gb.borrow_mut() = true));
    let gc = Rc::clone(&got_c);
    let agent_c = model.add_agent(10.0, Box::new(move || *gc.borrow_mut() = true));

    model.request_access(agent_a, &mut sched);
    model.request_access(agent_b, &mut sched);
    model.request_access(agent_c, &mut sched);

    sched.do_time_slice();

    // Agent C (priority 10) should get access
    assert!(!*got_a.borrow());
    assert!(!*got_b.borrow());
    assert!(*got_c.borrow());
    assert!(model.has_access(agent_c));

    // Release C, B should get access next
    model.release_access(agent_c, &mut sched);
    sched.do_time_slice();

    assert!(!*got_a.borrow());
    assert!(*got_b.borrow());
    assert!(model.has_access(agent_b));

    // Release B, A should get access
    model.release_access(agent_b, &mut sched);
    sched.do_time_slice();

    assert!(*got_a.borrow());
    assert!(model.has_access(agent_a));

    model.release_access(agent_a, &mut sched);
    model.remove(&mut sched);
}

#[test]
fn request_while_active_requeues() {
    let mut sched = EngineScheduler::new();
    let mut model = PriSchedModel::new(&mut sched);

    let count = Rc::new(RefCell::new(0u32));
    let c = Rc::clone(&count);
    let agent = model.add_agent(1.0, Box::new(move || *c.borrow_mut() += 1));

    model.request_access(agent, &mut sched);
    sched.do_time_slice();
    assert_eq!(*count.borrow(), 1);
    assert!(model.has_access(agent));

    // Re-request while active clears active and requeues
    model.request_access(agent, &mut sched);
    assert!(!model.has_access(agent));
    assert!(model.is_waiting_for_access(agent));

    sched.do_time_slice();
    assert_eq!(*count.borrow(), 2);
    assert!(model.has_access(agent));

    model.release_access(agent, &mut sched);
    model.remove(&mut sched);
}

#[test]
fn release_without_access_is_noop() {
    let mut sched = EngineScheduler::new();
    let mut model = PriSchedModel::new(&mut sched);

    let agent = model.add_agent(1.0, Box::new(|| {}));

    // Release when not active and not waiting — should not panic
    model.release_access(agent, &mut sched);
    assert!(!model.has_access(agent));
    assert!(!model.is_waiting_for_access(agent));

    model.remove(&mut sched);
}

#[test]
fn no_grant_when_active_exists() {
    let mut sched = EngineScheduler::new();
    let mut model = PriSchedModel::new(&mut sched);

    let got_a = Rc::new(RefCell::new(false));
    let got_b = Rc::new(RefCell::new(false));

    let ga = Rc::clone(&got_a);
    let agent_a = model.add_agent(1.0, Box::new(move || *ga.borrow_mut() = true));
    let gb = Rc::clone(&got_b);
    let agent_b = model.add_agent(2.0, Box::new(move || *gb.borrow_mut() = true));

    // A gets access
    model.request_access(agent_a, &mut sched);
    sched.do_time_slice();
    assert!(model.has_access(agent_a));

    // B requests but A is still active — B should not get access
    model.request_access(agent_b, &mut sched);
    sched.do_time_slice();
    assert!(!*got_b.borrow());
    assert!(!model.has_access(agent_b));
    assert!(model.is_waiting_for_access(agent_b));

    model.release_access(agent_a, &mut sched);
    sched.do_time_slice();
    assert!(*got_b.borrow());
    assert!(model.has_access(agent_b));

    model.release_access(agent_b, &mut sched);
    model.remove(&mut sched);
}

#[test]
fn set_access_priority_changes_grant_order() {
    let mut sched = EngineScheduler::new();
    let mut model = PriSchedModel::new(&mut sched);

    let got_a = Rc::new(RefCell::new(false));
    let got_b = Rc::new(RefCell::new(false));

    let ga = Rc::clone(&got_a);
    let agent_a = model.add_agent(1.0, Box::new(move || *ga.borrow_mut() = true));
    let gb = Rc::clone(&got_b);
    let agent_b = model.add_agent(10.0, Box::new(move || *gb.borrow_mut() = true));

    // Boost A above B
    model.set_access_priority(agent_a, 20.0);

    model.request_access(agent_a, &mut sched);
    model.request_access(agent_b, &mut sched);
    sched.do_time_slice();

    // A should win now despite originally being lower
    assert!(*got_a.borrow());
    assert!(!*got_b.borrow());
    assert!(model.has_access(agent_a));

    model.release_access(agent_a, &mut sched);
    model.release_access(agent_b, &mut sched);
    model.remove(&mut sched);
}

#[test]
fn is_waiting_tracks_state() {
    let mut sched = EngineScheduler::new();
    let mut model = PriSchedModel::new(&mut sched);

    let agent = model.add_agent(1.0, Box::new(|| {}));

    assert!(!model.is_waiting_for_access(agent));

    model.request_access(agent, &mut sched);
    assert!(model.is_waiting_for_access(agent));

    sched.do_time_slice();
    // After grant, no longer waiting
    assert!(!model.is_waiting_for_access(agent));
    assert!(model.has_access(agent));

    model.release_access(agent, &mut sched);
    model.remove(&mut sched);
}
