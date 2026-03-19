//! Systematic interaction test for ListBox at 1x and 2x zoom, driven through
//! the full input dispatch pipeline (PipelineTestHarness).
//!
//! Verifies that clicking on different items selects the correct item at both
//! zoom levels, using view-space coordinates derived from the panel's viewed
//! geometry and the border's content rect.


use std::cell::RefCell;
use std::rc::Rc;

use zuicchini::input::{Cursor, InputEvent, InputKey, InputState};
use zuicchini::panel::{NoticeFlags, PanelBehavior, PanelState};
use zuicchini::render::{Painter, SoftwareCompositor};
use zuicchini::widget::{
    Border, InnerBorderType, ListBox, Look, OuterBorderType, SelectionMode,
};

use super::support::pipeline::PipelineTestHarness;

/// PanelBehavior wrapper for ListBox, allowing shared access via Rc<RefCell>.
///
/// Copied from `behavioral_interaction.rs` SharedListBoxPanel pattern.
struct SharedListBoxPanel {
    inner: Rc<RefCell<ListBox>>,
}

impl PanelBehavior for SharedListBoxPanel {
    fn paint(&mut self, painter: &mut Painter, w: f64, h: f64, _state: &PanelState) {
        self.inner.borrow_mut().paint(painter, w, h);
    }

    fn input(
        &mut self,
        event: &InputEvent,
        state: &PanelState,
        input_state: &InputState,
    ) -> bool {
        self.inner.borrow_mut().input(event, state, input_state)
    }

    fn notice(&mut self, flags: NoticeFlags, state: &PanelState) {
        if flags.intersects(NoticeFlags::FOCUS_CHANGED) {
            self.inner
                .borrow_mut()
                .on_focus_changed(state.in_active_path);
        }
        if flags.intersects(NoticeFlags::ENABLE_CHANGED) {
            self.inner.borrow_mut().on_enable_changed(state.enabled);
        }
    }

    fn is_opaque(&self) -> bool {
        true
    }

    fn get_cursor(&self) -> Cursor {
        Cursor::Normal
    }
}

/// Compute the view-space Y coordinate for the vertical center of item `n`
/// (0-indexed) in a ListBox with `item_count` items.
///
/// The items are positioned within the border's content rect in panel-local
/// space (x in [0,1], y in [0,tallness]). This function:
///   1. Constructs a border matching ListBox's default config
///   2. Queries content_rect_unobscured in normalized panel-local space
///   3. Computes item N's center within the content rect
///   4. Maps the panel-local coordinate to view space using the viewed rect
fn item_center_view_y(
    vr: &zuicchini::foundation::Rect,
    pixel_tallness: f64,
    n: usize,
    item_count: usize,
) -> f64 {
    let look = Look::new();

    // Reconstruct the border with the same config as ListBox::new.
    let border = Border::new(OuterBorderType::Instrument)
        .with_inner(InnerBorderType::InputField)
        .with_how_to(true);

    // Panel-local coordinate space: x in [0, 1], y in [0, tallness].
    // tallness = (panel_pixel_h / panel_pixel_w) * pixel_tallness
    let tallness = (vr.h / vr.w) * pixel_tallness;

    let cr = border.content_rect_unobscured(1.0, tallness, &look);

    // Item N's center Y in panel-local space.
    let item_local_y = cr.y + (n as f64 + 0.5) / item_count as f64 * cr.h;

    // Map panel-local Y to view-space Y.
    // panel-local y in [0, tallness] maps to view-space [vr.y, vr.y + vr.h].
    vr.y + (item_local_y / tallness) * vr.h
}

/// Compute the view-space X coordinate at the horizontal center of the
/// content rect.
fn content_center_view_x(
    vr: &zuicchini::foundation::Rect,
    pixel_tallness: f64,
) -> f64 {
    let look = Look::new();
    let border = Border::new(OuterBorderType::Instrument)
        .with_inner(InnerBorderType::InputField)
        .with_how_to(true);

    let tallness = (vr.h / vr.w) * pixel_tallness;
    let cr = border.content_rect_unobscured(1.0, tallness, &look);

    let local_x = cr.x + cr.w * 0.5;
    vr.x + local_x * vr.w
}

#[test]
fn listbox_click_items_1x_and_2x() {
    // 1. Create PipelineTestHarness (800x600 viewport).
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    // 2. Create ListBox with 5 items, SelectionMode::Single.
    let look = Look::new();
    let mut lb = ListBox::new(look);
    lb.set_selection_mode(SelectionMode::Single);
    lb.add_item("item0".to_string(), "Alpha".to_string());
    lb.add_item("item1".to_string(), "Beta".to_string());
    lb.add_item("item2".to_string(), "Gamma".to_string());
    lb.add_item("item3".to_string(), "Delta".to_string());
    lb.add_item("item4".to_string(), "Epsilon".to_string());

    let lb_ref = Rc::new(RefCell::new(lb));

    // 3. Wrap in SharedListBoxPanel and add to tree.
    let panel_id = h.add_panel_with(
        root,
        "listbox",
        Box::new(SharedListBoxPanel {
            inner: lb_ref.clone(),
        }),
    );

    // 4. Tick + render (SoftwareCompositor) to populate last_w/last_h.
    h.tick_n(5);
    let mut compositor = SoftwareCompositor::new(800, 600);
    compositor.render(&mut h.tree, &h.view);

    let pt = h.view.pixel_tallness();

    // ---------- 5. At 1x zoom ----------

    let state = h.tree.build_panel_state(
        panel_id,
        h.view.window_focused(),
        pt,
    );
    let vr = state.viewed_rect;
    let click_x = content_center_view_x(&vr, pt);

    // Click item 0
    h.click(click_x, item_center_view_y(&vr, pt, 0, 5));
    assert_eq!(
        lb_ref.borrow().selected_index(),
        Some(0),
        "At 1x zoom: clicking item 0 should select it"
    );

    // Click item 2
    h.click(click_x, item_center_view_y(&vr, pt, 2, 5));
    assert_eq!(
        lb_ref.borrow().selected_index(),
        Some(2),
        "At 1x zoom: clicking item 2 should select it"
    );

    // Click item 4
    h.click(click_x, item_center_view_y(&vr, pt, 4, 5));
    assert_eq!(
        lb_ref.borrow().selected_index(),
        Some(4),
        "At 1x zoom: clicking item 4 should select it"
    );

    // ---------- 6. At 2x zoom ----------

    h.set_zoom(2.0);
    h.tick_n(5);
    compositor.render(&mut h.tree, &h.view);

    let state_2x = h.tree.build_panel_state(
        panel_id,
        h.view.window_focused(),
        pt,
    );
    let vr2 = state_2x.viewed_rect;
    let click_x_2x = content_center_view_x(&vr2, pt);

    // Click item 0
    h.click(click_x_2x, item_center_view_y(&vr2, pt, 0, 5));
    assert_eq!(
        lb_ref.borrow().selected_index(),
        Some(0),
        "At 2x zoom: clicking item 0 should select it"
    );

    // Click item 2
    h.click(click_x_2x, item_center_view_y(&vr2, pt, 2, 5));
    assert_eq!(
        lb_ref.borrow().selected_index(),
        Some(2),
        "At 2x zoom: clicking item 2 should select it"
    );

    // Click item 4
    h.click(click_x_2x, item_center_view_y(&vr2, pt, 4, 5));
    assert_eq!(
        lb_ref.borrow().selected_index(),
        Some(4),
        "At 2x zoom: clicking item 4 should select it"
    );
}

// ── BP-1: ListBox selection mode behavioral parity tests ─────────────────
//
// These tests exercise every branch in C++ emListBox::SelectByInput across
// all four SelectionMode variants (ReadOnly, Single, Multi, Toggle), driven
// through the full PipelineTestHarness dispatch pipeline.
//
// C++ ref: emListBox.cpp:786-848 (SelectByInput)
//          emListBox.cpp:751-783 (ProcessItemInput)

/// Helper: create a PipelineTestHarness with a ListBox containing 5 items
/// in the given SelectionMode, render once to populate geometry, and return
/// (harness, lb_ref, panel_id, click_x, item_ys).
fn setup_listbox_harness(
    mode: SelectionMode,
) -> (
    PipelineTestHarness,
    Rc<RefCell<ListBox>>,
    zuicchini::panel::PanelId,
    f64,
    [f64; 5],
) {
    let mut h = PipelineTestHarness::new();
    let root = h.root();

    let look = Look::new();
    let mut lb = ListBox::new(look);
    lb.set_selection_mode(mode);
    lb.add_item("i0".to_string(), "Alpha".to_string());
    lb.add_item("i1".to_string(), "Beta".to_string());
    lb.add_item("i2".to_string(), "Gamma".to_string());
    lb.add_item("i3".to_string(), "Delta".to_string());
    lb.add_item("i4".to_string(), "Epsilon".to_string());

    let lb_ref = Rc::new(RefCell::new(lb));
    let panel_id = h.add_panel_with(
        root,
        "listbox",
        Box::new(SharedListBoxPanel {
            inner: lb_ref.clone(),
        }),
    );

    h.tick_n(5);
    let mut compositor = SoftwareCompositor::new(800, 600);
    compositor.render(&mut h.tree, &h.view);

    let pt = h.view.pixel_tallness();
    let state = h.tree.build_panel_state(panel_id, h.view.window_focused(), pt);
    let vr = state.viewed_rect;
    let click_x = content_center_view_x(&vr, pt);
    let item_ys = [
        item_center_view_y(&vr, pt, 0, 5),
        item_center_view_y(&vr, pt, 1, 5),
        item_center_view_y(&vr, pt, 2, 5),
        item_center_view_y(&vr, pt, 3, 5),
        item_center_view_y(&vr, pt, 4, 5),
    ];

    (h, lb_ref, panel_id, click_x, item_ys)
}

// ── Single mode ──────────────────────────────────────────────────────────

#[test]
fn listbox_single_mode_click_selects() {
    // C++ ref: SelectByInput SINGLE_SELECTION branch — Select(itemIndex, true)
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Single);

    h.click(cx, ys[2]);
    assert_eq!(lb.borrow().selected_index(), Some(2));

    // Clicking another item replaces the selection (solely=true).
    h.click(cx, ys[4]);
    assert_eq!(lb.borrow().selected_index(), Some(4));
    assert!(!lb.borrow().is_selected(2));
}

#[test]
fn listbox_single_mode_shift_click_still_selects_solely() {
    // C++ ref: SINGLE_SELECTION ignores shift/ctrl — always Select(itemIndex, true).
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Single);

    h.click(cx, ys[1]);
    assert_eq!(lb.borrow().selected_index(), Some(1));

    // Shift+click in Single mode still selects solely.
    h.input_state.press(InputKey::Shift);
    h.click(cx, ys[3]);
    h.input_state.release(InputKey::Shift);
    assert_eq!(lb.borrow().selected_index(), Some(3));
    assert!(!lb.borrow().is_selected(1));
}

#[test]
fn listbox_single_mode_ctrl_click_still_selects_solely() {
    // C++ ref: SINGLE_SELECTION ignores ctrl — always Select(itemIndex, true).
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Single);

    h.click(cx, ys[0]);
    h.input_state.press(InputKey::Ctrl);
    h.click(cx, ys[2]);
    h.input_state.release(InputKey::Ctrl);
    assert_eq!(lb.borrow().selected_index(), Some(2));
    assert!(!lb.borrow().is_selected(0));
}

// ── Multi mode ──────────────────────────────────────────────────────────

#[test]
fn listbox_multi_mode_click_selects_solely() {
    // C++ ref: MULTI_SELECTION, no shift, no ctrl -> Select(itemIndex, true)
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Multi);

    h.click(cx, ys[1]);
    assert_eq!(lb.borrow().selected_index(), Some(1));

    h.click(cx, ys[3]);
    assert_eq!(lb.borrow().selected_index(), Some(3));
    assert!(!lb.borrow().is_selected(1), "plain click in Multi replaces selection");
}

#[test]
fn listbox_multi_shift_click_extends_range() {
    // C++ ref: MULTI_SELECTION, shift=true, ctrl=false ->
    //   range from prev+1..=item (or item..=prev-1), Select(i, false)
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Multi);

    // Click item 1 to set prev_input_index.
    h.click(cx, ys[1]);
    assert_eq!(lb.borrow().selected_index(), Some(1));

    // Shift+click item 3: should select range 2..=3 (prev+1..=clicked),
    // and item 1 stays selected since those were Select(i, false) calls.
    h.input_state.press(InputKey::Shift);
    h.click(cx, ys[3]);
    h.input_state.release(InputKey::Shift);

    assert!(lb.borrow().is_selected(1), "item 1 still selected");
    assert!(lb.borrow().is_selected(2), "item 2 selected by shift range");
    assert!(lb.borrow().is_selected(3), "item 3 selected by shift range");
    assert!(!lb.borrow().is_selected(0));
    assert!(!lb.borrow().is_selected(4));
}

#[test]
fn listbox_multi_shift_click_extends_range_backward() {
    // C++ ref: MULTI_SELECTION, shift=true, prev > itemIndex ->
    //   range item..=prev-1, Select(i, false)
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Multi);

    // Click item 3 to set prev_input_index.
    h.click(cx, ys[3]);
    assert_eq!(lb.borrow().selected_index(), Some(3));

    // Shift+click item 1: range is 1..=2 (item..=prev-1).
    h.input_state.press(InputKey::Shift);
    h.click(cx, ys[1]);
    h.input_state.release(InputKey::Shift);

    assert!(lb.borrow().is_selected(1), "item 1 in backward range");
    assert!(lb.borrow().is_selected(2), "item 2 in backward range");
    assert!(lb.borrow().is_selected(3), "item 3 still selected");
    assert!(!lb.borrow().is_selected(0));
    assert!(!lb.borrow().is_selected(4));
}

#[test]
fn listbox_multi_ctrl_click_toggles() {
    // C++ ref: MULTI_SELECTION, shift=false, ctrl=true -> ToggleSelection(itemIndex)
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Multi);

    // Click item 1 (solely selects it).
    h.click(cx, ys[1]);
    assert_eq!(lb.borrow().selected_indices(), &[1]);

    // Ctrl+click item 3: toggles item 3 on.
    h.input_state.press(InputKey::Ctrl);
    h.click(cx, ys[3]);
    assert!(lb.borrow().is_selected(1), "item 1 stays");
    assert!(lb.borrow().is_selected(3), "item 3 toggled on");

    // Ctrl+click item 1 again: toggles item 1 off.
    h.click(cx, ys[1]);
    h.input_state.release(InputKey::Ctrl);
    assert!(!lb.borrow().is_selected(1), "item 1 toggled off");
    assert!(lb.borrow().is_selected(3), "item 3 remains");
}

#[test]
fn listbox_multi_shift_ctrl_click_toggles_range() {
    // C++ ref: MULTI_SELECTION, shift=true, ctrl=true ->
    //   range toggle: ToggleSelection(i) for each i in range
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Multi);

    // Click item 1 (sets prev_input_index, selects solely).
    h.click(cx, ys[1]);

    // Shift+Ctrl click item 3: toggles items 2..=3.
    h.input_state.press(InputKey::Shift);
    h.input_state.press(InputKey::Ctrl);
    h.click(cx, ys[3]);
    h.input_state.release(InputKey::Shift);
    h.input_state.release(InputKey::Ctrl);

    // Items 2 and 3 were unselected, so toggle turns them on.
    assert!(lb.borrow().is_selected(1), "item 1 stays from initial click");
    assert!(lb.borrow().is_selected(2), "item 2 toggled on");
    assert!(lb.borrow().is_selected(3), "item 3 toggled on");
}

// ── Toggle mode ──────────────────────────────────────────────────────────

#[test]
fn listbox_toggle_mode_click_toggles() {
    // C++ ref: TOGGLE_SELECTION, no shift -> ToggleSelection(itemIndex)
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Toggle);

    h.click(cx, ys[0]);
    assert!(lb.borrow().is_selected(0), "first click toggles on");

    h.click(cx, ys[2]);
    assert!(lb.borrow().is_selected(0), "item 0 stays on");
    assert!(lb.borrow().is_selected(2), "item 2 toggled on");

    // Click item 0 again to toggle it off.
    h.click(cx, ys[0]);
    assert!(!lb.borrow().is_selected(0), "second click toggles off");
    assert!(lb.borrow().is_selected(2), "item 2 unaffected");
}

#[test]
fn listbox_toggle_mode_ctrl_click_also_toggles() {
    // C++ ref: TOGGLE_SELECTION, ctrl has no special behavior — still
    // goes to the else branch which calls ToggleSelection(itemIndex).
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Toggle);

    h.click(cx, ys[1]);
    assert!(lb.borrow().is_selected(1));

    // Ctrl+click should still toggle (ctrl is not special in Toggle mode).
    h.input_state.press(InputKey::Ctrl);
    h.click(cx, ys[1]);
    h.input_state.release(InputKey::Ctrl);
    assert!(!lb.borrow().is_selected(1), "ctrl+click still toggles off");
}

#[test]
fn listbox_toggle_shift_click_toggles_range() {
    // C++ ref: TOGGLE_SELECTION, shift=true ->
    //   range prev+1..=item, ToggleSelection(i)
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Toggle);

    // Click item 1 (toggles on, sets prev).
    h.click(cx, ys[1]);
    assert!(lb.borrow().is_selected(1));

    // Shift+click item 4: toggles range 2..=4.
    h.input_state.press(InputKey::Shift);
    h.click(cx, ys[4]);
    h.input_state.release(InputKey::Shift);

    assert!(lb.borrow().is_selected(1), "item 1 from initial click");
    assert!(lb.borrow().is_selected(2), "item 2 toggled on by range");
    assert!(lb.borrow().is_selected(3), "item 3 toggled on by range");
    assert!(lb.borrow().is_selected(4), "item 4 toggled on by range");
    assert!(!lb.borrow().is_selected(0));
}

// ── ReadOnly mode ────────────────────────────────────────────────────────

#[test]
fn listbox_readonly_rejects_click() {
    // C++ ref: READ_ONLY_SELECTION branch is empty (break), so no selection change.
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::ReadOnly);

    h.click(cx, ys[0]);
    assert!(lb.borrow().selected_indices().is_empty(), "ReadOnly rejects click");
}

#[test]
fn listbox_readonly_rejects_shift_click() {
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::ReadOnly);

    h.input_state.press(InputKey::Shift);
    h.click(cx, ys[2]);
    h.input_state.release(InputKey::Shift);
    assert!(lb.borrow().selected_indices().is_empty(), "ReadOnly rejects shift+click");
}

#[test]
fn listbox_readonly_rejects_ctrl_click() {
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::ReadOnly);

    h.input_state.press(InputKey::Ctrl);
    h.click(cx, ys[2]);
    h.input_state.release(InputKey::Ctrl);
    assert!(lb.borrow().selected_indices().is_empty(), "ReadOnly rejects ctrl+click");
}

// ── Double-click (trigger) ──────────────────────────────────────────────

/// Helper: dispatch a double-click (repeat=1) at view-space coordinates through
/// the full pipeline. This simulates what the windowing system sends for a
/// rapid second click.
fn double_click(h: &mut PipelineTestHarness, view_x: f64, view_y: f64) {
    // First click (repeat=0).
    let press1 = InputEvent::press(InputKey::MouseLeft).with_mouse(view_x, view_y);
    let release1 = InputEvent::release(InputKey::MouseLeft).with_mouse(view_x, view_y);
    h.dispatch(&press1);
    h.dispatch(&release1);
    // Second click (repeat=1 = double-click).
    let press2 = InputEvent::press(InputKey::MouseLeft)
        .with_mouse(view_x, view_y)
        .with_repeat(1);
    let release2 = InputEvent::release(InputKey::MouseLeft).with_mouse(view_x, view_y);
    h.dispatch(&press2);
    h.dispatch(&release2);
}

#[test]
fn listbox_single_mode_double_click_triggers() {
    // C++ ref: SINGLE_SELECTION — if (trigger) TriggerItem(itemIndex)
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Single);

    let triggered = Rc::new(RefCell::new(None::<usize>));
    let trig_clone = triggered.clone();
    lb.borrow_mut().on_trigger = Some(Box::new(move |idx| {
        *trig_clone.borrow_mut() = Some(idx);
    }));

    double_click(&mut h, cx, ys[2]);
    assert_eq!(lb.borrow().selected_index(), Some(2));
    assert_eq!(*triggered.borrow(), Some(2), "double-click triggers in Single mode");
}

#[test]
fn listbox_multi_mode_double_click_triggers() {
    // C++ ref: MULTI_SELECTION — if (trigger) TriggerItem(itemIndex)
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Multi);

    let triggered = Rc::new(RefCell::new(None::<usize>));
    let trig_clone = triggered.clone();
    lb.borrow_mut().on_trigger = Some(Box::new(move |idx| {
        *trig_clone.borrow_mut() = Some(idx);
    }));

    double_click(&mut h, cx, ys[3]);
    assert_eq!(*triggered.borrow(), Some(3), "double-click triggers in Multi mode");
}

#[test]
fn listbox_toggle_mode_double_click_triggers() {
    // C++ ref: TOGGLE_SELECTION — if (trigger) TriggerItem(itemIndex)
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Toggle);

    let triggered = Rc::new(RefCell::new(None::<usize>));
    let trig_clone = triggered.clone();
    lb.borrow_mut().on_trigger = Some(Box::new(move |idx| {
        *trig_clone.borrow_mut() = Some(idx);
    }));

    double_click(&mut h, cx, ys[1]);
    assert_eq!(*triggered.borrow(), Some(1), "double-click triggers in Toggle mode");
}

#[test]
fn listbox_readonly_double_click_no_trigger() {
    // C++ ref: READ_ONLY_SELECTION branch does NOT call TriggerItem.
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::ReadOnly);

    let triggered = Rc::new(RefCell::new(None::<usize>));
    let trig_clone = triggered.clone();
    lb.borrow_mut().on_trigger = Some(Box::new(move |idx| {
        *trig_clone.borrow_mut() = Some(idx);
    }));

    double_click(&mut h, cx, ys[2]);
    assert!(lb.borrow().selected_indices().is_empty(), "ReadOnly: no selection");
    assert_eq!(*triggered.borrow(), None, "ReadOnly: no trigger on double-click");
}

// ── Enter key trigger (all modes) ────────────────────────────────────────

#[test]
fn listbox_single_mode_enter_triggers() {
    // C++ ref: ProcessItemInput EM_KEY_ENTER -> SelectByInput(..., trigger=true)
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Single);

    let triggered = Rc::new(RefCell::new(None::<usize>));
    let trig_clone = triggered.clone();
    lb.borrow_mut().on_trigger = Some(Box::new(move |idx| {
        *trig_clone.borrow_mut() = Some(idx);
    }));

    // First click to select and focus item 2.
    h.click(cx, ys[2]);
    assert_eq!(lb.borrow().selected_index(), Some(2));

    // Enter triggers the focused item.
    h.press_key(InputKey::Enter);
    assert_eq!(*triggered.borrow(), Some(2), "Enter triggers in Single mode");
}

#[test]
fn listbox_readonly_enter_no_trigger() {
    // C++ ref: READ_ONLY_SELECTION -> no trigger
    let (mut h, lb, _pid, _cx, _ys) = setup_listbox_harness(SelectionMode::ReadOnly);

    let triggered = Rc::new(RefCell::new(None::<usize>));
    let trig_clone = triggered.clone();
    lb.borrow_mut().on_trigger = Some(Box::new(move |idx| {
        *trig_clone.borrow_mut() = Some(idx);
    }));

    h.press_key(InputKey::Enter);
    assert_eq!(*triggered.borrow(), None, "ReadOnly: Enter does not trigger");
}

// ── Ctrl+A / Shift+Ctrl+A (select all / clear) ──────────────────────────

#[test]
fn listbox_multi_ctrl_a_selects_all() {
    // C++ ref: emListBox::Input Key('A') + Ctrl -> SelectAll()
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Multi);

    // Click first to activate the panel (keyboard events require active path).
    h.click(cx, ys[0]);

    h.input_state.press(InputKey::Ctrl);
    let press = InputEvent::press(InputKey::Key('a')).with_chars("a");
    h.dispatch(&press);
    let release = InputEvent::release(InputKey::Key('a'));
    h.dispatch(&release);
    h.input_state.release(InputKey::Ctrl);

    assert_eq!(lb.borrow().selected_indices(), &[0, 1, 2, 3, 4]);
}

#[test]
fn listbox_multi_shift_ctrl_a_clears() {
    // C++ ref: emListBox::Input Shift+Ctrl+A -> ClearSelection()
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Multi);

    // Select some items first.
    h.click(cx, ys[0]);
    assert_eq!(lb.borrow().selected_indices(), &[0]);

    h.input_state.press(InputKey::Shift);
    h.input_state.press(InputKey::Ctrl);
    let press = InputEvent::press(InputKey::Key('a')).with_chars("a");
    h.dispatch(&press);
    let release = InputEvent::release(InputKey::Key('a'));
    h.dispatch(&release);
    h.input_state.release(InputKey::Shift);
    h.input_state.release(InputKey::Ctrl);

    assert!(lb.borrow().selected_indices().is_empty(), "Shift+Ctrl+A clears in Multi");
}

#[test]
fn listbox_toggle_ctrl_a_selects_all() {
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Toggle);

    // Click first to activate the panel (keyboard events require active path).
    h.click(cx, ys[0]);

    h.input_state.press(InputKey::Ctrl);
    let press = InputEvent::press(InputKey::Key('a')).with_chars("a");
    h.dispatch(&press);
    let release = InputEvent::release(InputKey::Key('a'));
    h.dispatch(&release);
    h.input_state.release(InputKey::Ctrl);

    assert_eq!(lb.borrow().selected_indices(), &[0, 1, 2, 3, 4]);
}

#[test]
fn listbox_single_ctrl_a_no_effect() {
    // C++ ref: Ctrl+A only works in Multi/Toggle modes.
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Single);

    h.click(cx, ys[1]);
    assert_eq!(lb.borrow().selected_indices(), &[1]);

    h.input_state.press(InputKey::Ctrl);
    let press = InputEvent::press(InputKey::Key('a')).with_chars("a");
    h.dispatch(&press);
    let release = InputEvent::release(InputKey::Key('a'));
    h.dispatch(&release);
    h.input_state.release(InputKey::Ctrl);

    // Single mode: Ctrl+A should not select all.
    assert_eq!(
        lb.borrow().selected_indices(),
        &[1],
        "Ctrl+A has no effect in Single mode"
    );
}

// ── Space key selection ──────────────────────────────────────────────────

#[test]
fn listbox_multi_space_selects_solely() {
    // C++ ref: EM_KEY_SPACE -> SelectByInput(idx, shift=false, ctrl=false, trigger=false)
    // In Multi mode without modifiers -> Select(itemIndex, true)
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Multi);

    // Click item 1 to set focus.
    h.click(cx, ys[1]);
    assert_eq!(lb.borrow().selected_indices(), &[1]);

    // Press ArrowDown to move focus to item 2 (no selection change in Multi).
    h.press_key(InputKey::ArrowDown);
    // Press Space: selects focused item solely.
    h.press_key(InputKey::Space);
    assert_eq!(lb.borrow().selected_indices(), &[2], "Space selects solely in Multi");
}

#[test]
fn listbox_toggle_space_toggles() {
    // C++ ref: EM_KEY_SPACE -> SelectByInput with no shift -> ToggleSelection
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Toggle);

    // Click item 0 to focus and toggle on.
    h.click(cx, ys[0]);
    assert!(lb.borrow().is_selected(0));

    // Space toggles off.
    h.press_key(InputKey::Space);
    assert!(!lb.borrow().is_selected(0), "Space toggles off in Toggle mode");

    // Space toggles on again.
    h.press_key(InputKey::Space);
    assert!(lb.borrow().is_selected(0), "Space toggles on again");
}

#[test]
fn listbox_multi_shift_space_extends_range() {
    // C++ ref: EM_KEY_SPACE + Shift -> SelectByInput(idx, shift=true, ctrl=false, false)
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Multi);

    // Click item 1 to set prev_input_index and select.
    h.click(cx, ys[1]);

    // Move focus to item 3 without selecting.
    h.press_key(InputKey::ArrowDown); // focus 2
    h.press_key(InputKey::ArrowDown); // focus 3

    // Shift+Space: extends range from prev_input(1) to focus(3).
    h.input_state.press(InputKey::Shift);
    h.press_key(InputKey::Space);
    h.input_state.release(InputKey::Shift);

    assert!(lb.borrow().is_selected(1), "item 1 from initial click");
    assert!(lb.borrow().is_selected(2), "item 2 from shift+space range");
    assert!(lb.borrow().is_selected(3), "item 3 from shift+space range");
}

#[test]
fn listbox_multi_ctrl_space_toggles() {
    // C++ ref: EM_KEY_SPACE + Ctrl -> SelectByInput(idx, shift=false, ctrl=true, false)
    // In Multi mode, ctrl=true -> ToggleSelection
    let (mut h, lb, _pid, cx, ys) = setup_listbox_harness(SelectionMode::Multi);

    h.click(cx, ys[1]);
    assert_eq!(lb.borrow().selected_indices(), &[1]);

    // Ctrl+Space on same item toggles it off.
    h.input_state.press(InputKey::Ctrl);
    h.press_key(InputKey::Space);
    h.input_state.release(InputKey::Ctrl);
    assert!(!lb.borrow().is_selected(1), "Ctrl+Space toggles off in Multi");
}
