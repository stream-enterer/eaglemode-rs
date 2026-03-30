// Trace checkbox geometry through the Rust pipeline.
// Must match trace_checkbox.cpp exactly.
// Run: rustc trace_checkbox.rs -o trace_checkbox_rust && ./trace_checkbox_rust

fn hex(label: &str, v: f64) {
    println!("{} = {:.17} [{:016x}]", label, v, v.to_bits());
}

fn main() {
    // Step 1: GetBestLabelTallness for "Check Option" (no icon, no desc)
    let caption = "Check Option";
    let char_height: f64 = 1.0;
    let char_box_tallness: f64 = 1.77;

    // measure_text_width: char_height * chars / CHAR_BOX_TALLNESS
    let cap_w = char_height * caption.chars().count() as f64 / char_box_tallness;
    let cap_h: f64 = 1.0; // Rust hardcodes this
    hex("capW", cap_w);
    hex("capH", cap_h);
    let tallness_raw = cap_h / cap_w;
    hex("tallness_raw", tallness_raw);
    let tallness = tallness_raw.max(0.2);
    hex("tallness_clamped", tallness);

    // Step 2: GetContentRect for OBT_MARGIN, no inner border
    let w: f64 = 1.0;
    let h: f64 = 0.5;

    let s_pre = w.min(h) * 1.0;
    let d_margin = s_pre * 0.057;
    hex("d_margin", d_margin);

    let mut rnd_x = d_margin;
    let mut rnd_y = d_margin;
    let mut rnd_w = w - 2.0 * d_margin;
    let mut rnd_h = h - 2.0 * d_margin;
    let mut _rnd_r: f64 = 0.0;

    let ms = s_pre * 0.023;
    hex("ms", ms);

    // No-label path: symmetric minSpace
    rnd_x += ms;
    rnd_y += ms;
    rnd_w -= 2.0 * ms;
    rnd_h -= 2.0 * ms;
    _rnd_r -= ms;

    let rec_x = rnd_x;
    let rec_y = rnd_y;
    let rec_w = rnd_w;
    let rec_h = rnd_h;

    hex("recX", rec_x);
    hex("recY", rec_y);
    hex("recW", rec_w);
    hex("recH", rec_h);

    // Step 3: box_label_geometry (emButton.cpp:237-260)
    let _lw: f64 = 1.0;
    let lh = tallness;
    let mut bw = lh;
    let mut d_box = bw * 0.1;
    let f = (rec_w / (bw + d_box + 1.0)).min(rec_h / lh);
    hex("f_scale", f);

    bw *= f;
    d_box *= f;
    let _lw2 = rec_w - bw - d_box;
    let _lh2 = bw;

    hex("bw", bw);
    hex("bx", rec_x);
    hex("by", rec_y + (rec_h - bw) * 0.5);

    // Image inset
    let d_img = bw * 0.13;
    let mut bx = rec_x + d_img;
    let by = rec_y + (rec_h - bw) * 0.5 + d_img;
    bw -= 2.0 * d_img;
    let bh = bw;

    hex("face_bx", bx);
    hex("face_by", by);
    hex("face_bw", bw);

    // Face inset
    let d2 = bw * 30.0 / 380.0;
    let fx = bx + d2;
    let fy = by + d2;
    let fw = bw - 2.0 * d2;
    let fh = bh - 2.0 * d2;

    hex("fx", fx);
    hex("fy", fy);
    hex("fw", fw);

    // Checkmark vertices
    hex("check_x0", fx + fw * 0.2);
    hex("check_y0", fy + fh * 0.6);
    hex("check_x1", fx + fw * 0.4);
    hex("check_y1", fy + fh * 0.8);
    hex("check_x2", fx + fw * 0.8);
    hex("check_y2", fy + fh * 0.2);
}
