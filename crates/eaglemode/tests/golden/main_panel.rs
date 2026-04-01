use std::rc::Rc;

use emMain::emMainPanel::emMainPanel;

use super::common::*;

/// Skip test if golden data hasn't been generated yet.
macro_rules! require_golden {
    () => {
        if !golden_available() {
            eprintln!("SKIP: golden/ directory not found — run `make -C golden_gen run` first");
            return;
        }
    };
}

fn check_layout(name: &str, h: f64, slider_pos: f64, control_tallness: f64) {
    let expected = load_layout_golden(name);
    let ctx = emcore::emContext::emContext::NewRoot();
    let mut panel = emMainPanel::new(Rc::clone(&ctx), control_tallness);
    let rects = panel.compute_layout_for_test(h, slider_pos, false);
    let actual: Vec<(f64, f64, f64, f64)> = rects.to_vec();
    compare_rects(&actual, &expected, 1e-12).unwrap();
}

#[test]
fn main_panel_layout_normal() {
    require_golden!();
    check_layout("main_panel_layout_normal", 2.0, 0.5, 0.0538);
}

#[test]
fn main_panel_layout_collapsed() {
    require_golden!();
    check_layout("main_panel_layout_collapsed", 2.0, 0.0, 0.0538);
}

#[test]
fn main_panel_layout_wide() {
    require_golden!();
    check_layout("main_panel_layout_wide", 0.5, 0.7, 0.0538);
}
