// Minimal f64 operation comparison — Rust side.
// Run: rustc -O fp_ops.rs -o fp_ops_rust && ./fp_ops_rust
// Or:  rustc fp_ops.rs -o fp_ops_rust_debug && ./fp_ops_rust_debug
// Output must match fp_ops.cpp exactly if f64 arithmetic agrees.

fn to_bits(d: f64) -> u64 {
    d.to_bits()
}

fn emit(label: &str, val: f64) {
    println!("{} {:016x} {}", label, to_bits(val), val as i32);
}

fn main() {
    // === 1. FMA-sensitive pattern ===
    {
        let a: f64 = 1.0000000000000002; // 1 + 1 ULP
        let b: f64 = 1.0000000000000002;
        let c: f64 = -1.0;
        emit("fma_sensitive", a * b + c);
    }

    // === 2. Edge intersection ===
    {
        let x1: f64 = 127.3;
        let y1: f64 = 45.7;
        let x2: f64 = 389.1;
        let y2: f64 = 201.4;
        let dx = x2 - x1;
        let dy = y2 - y1;
        let gx = if dy >= 0.0001 { dx / dy } else { 0.0 };
        let py1: f64 = 46.0;
        let qx1 = x1 + (py1 - y1) * gx;
        emit("edge_intersect_qx1", qx1);
    }

    // === 3. Trapezoid area ===
    {
        let px1: f64 = 127.0;
        let px2: f64 = 128.0;
        let _py1: f64 = 45.0;
        let py2: f64 = 46.0;
        let qx1: f64 = 127.35;
        let qy1: f64 = 45.12;
        let qx2: f64 = 127.89;
        let qy2: f64 = 45.98;
        let mut a2 = py2 - qy2;
        a2 = a2 * (px2 - px1) + (qy2 - qy1) * ((qx1 + qx2) * 0.5 - px1);
        emit("trapezoid_area", a2);
    }

    // === 4. Fixed12 conversion ===
    {
        let vals: [f64; 6] = [
            127.4999999999999,
            127.5000000000001,
            0.000244140625,      // exactly 1/4096
            300.75,
            -0.001,
            1023.999755859375,
        ];
        for (i, &v) in vals.iter().enumerate() {
            let label = format!("fixed12_{}", i);
            let fp12 = v * 4096.0;
            emit(&label, fp12);
        }
    }

    // === 5. Vertex transform ===
    {
        let x: f64 = 0.3456789;
        let y: f64 = 0.7654321;
        let scale_x: f64 = 800.0;
        let scale_y: f64 = 600.0;
        let offset_x: f64 = 0.0;
        let offset_y: f64 = 0.0;
        let x_px = x * scale_x + offset_x;
        let y_px = y * scale_y + offset_y;
        emit("transform_x", x_px);
        emit("transform_y", y_px);
    }

    // === 6. Division chain ===
    {
        let x1: f64 = 100.123456789;
        let y1: f64 = 200.987654321;
        let x2: f64 = 300.111111111;
        let y2: f64 = 400.222222222;
        let dx = x2 - x1;
        let dy = y2 - y1;
        let slope = dx / dy;
        let inv_slope = dy / dx;
        emit("slope", slope);
        emit("inv_slope", inv_slope);
    }

    // === 7. Accumulated sum ===
    {
        let widths: [f64; 20] = [
            0.056, 0.044, 0.056, 0.056, 0.039, 0.056, 0.033,
            0.056, 0.056, 0.022, 0.056, 0.050, 0.056, 0.067,
            0.056, 0.056, 0.056, 0.056, 0.039, 0.044,
        ];
        let mut total: f64 = 0.0;
        for &w in &widths {
            total += w;
        }
        emit("text_width_sum", total);
        let tallness = 0.15 / total;
        emit("label_tallness", tallness);
    }

    // === 8. Near-integer truncation boundary ===
    {
        let near_128 = 128.0 - 1e-15;
        let near_128b = 128.0 + 1e-15;
        let near_neg = -0.0000001;
        emit("trunc_below_128", near_128);
        emit("trunc_above_128", near_128b);
        emit("trunc_near_neg", near_neg);
    }

    // === 9. GetContentRoundRect cascade ===
    {
        let w: f64 = 1.0;
        let h: f64 = 0.5;
        let s = w.min(h) * 1.0;
        let d = s * 0.043;
        let _rnd_x = d;
        let rnd_y = d;
        let rnd_w = w - 2.0 * d;
        let rnd_h = h - 2.0 * d;
        let _rnd_r = s * 0.20;
        let ms = rnd_w.min(rnd_h) * 1.0;
        let label_h = 0.17 * ms;
        let content_y = rnd_y + label_h + ms * 0.023;
        let content_h = rnd_h - label_h - ms * 0.023;
        emit("content_y", content_y);
        emit("content_h", content_h);
    }

    // === 10. floor/ceil near boundary ===
    {
        let v1: f64 = 127.9999999999999;
        let v2: f64 = 128.0000000000001;
        emit("floor_v1", v1.floor());
        emit("ceil_v1", v1.ceil());
        emit("floor_v2", v2.floor());
        emit("ceil_v2", v2.ceil());
    }
}
