// Minimal f64 operation comparison between C++ and Rust.
// Tests the exact expressions used in the emPainter rasterizer pipeline.
// Compile: g++ -O2 -o fp_ops fp_ops.cpp -lm
//          clang++ -O2 -o fp_ops_clang fp_ops.cpp -lm
// Output: one line per test case, hex f64 bits + i32 truncation result.

#include <cstdio>
#include <cstdint>
#include <cmath>
#include <cstring>

static uint64_t to_bits(double d) {
    uint64_t bits;
    memcpy(&bits, &d, 8);
    return bits;
}

static void emit(const char* label, double val) {
    printf("%s %016llx %d\n", label, (unsigned long long)to_bits(val), (int)val);
}

int main() {
    // === 1. Basic FMA-sensitive pattern: a*b+c ===
    // If one compiler fuses and the other doesn't, results differ.
    {
        double a = 1.0000000000000002; // 1 + 1 ULP
        double b = 1.0000000000000002;
        double c = -1.0;
        // Without FMA: a*b rounds to nearest, then +c
        // With FMA: (a*b+c) computed in one step, no intermediate rounding
        emit("fma_sensitive", a * b + c);
    }

    // === 2. Scanline rasterizer: edge intersection ===
    // From emPainter.cpp PaintEdgeCorrection:
    //   gx = dx / dy;  qx1 = x1 + (py1 - y1) * gx;
    {
        double x1 = 127.3, y1 = 45.7, x2 = 389.1, y2 = 201.4;
        double dx = x2 - x1;
        double dy = y2 - y1;
        double gx = (dy >= 0.0001) ? dx / dy : 0.0;
        double py1 = 46.0;  // pixel boundary
        double qx1 = x1 + (py1 - y1) * gx;
        emit("edge_intersect_qx1", qx1);
    }

    // === 3. Trapezoid area (PaintEdgeCorrection coverage) ===
    // a2 = a2*(px2-px1) + (qy2-qy1)*((qx1+qx2)*0.5 - px1);
    {
        double px1 = 127.0, px2 = 128.0;
        double py1 = 45.0, py2 = 46.0;
        double qx1 = 127.35, qy1 = 45.12;
        double qx2 = 127.89, qy2 = 45.98;
        double a2 = py2 - qy2;
        a2 = a2 * (px2 - px1) + (qy2 - qy1) * ((qx1 + qx2) * 0.5 - px1);
        emit("trapezoid_area", a2);
    }

    // === 4. Fixed12 conversion: (x * 4096.0) as i32 ===
    // The boundary case where ULP matters for truncation.
    {
        double vals[] = {
            127.4999999999999,
            127.5000000000001,
            0.000244140625,      // exactly 1/4096
            300.75,
            -0.001,
            1023.999755859375,   // 4095.999/4096 * 1024
        };
        for (int i = 0; i < 6; i++) {
            char label[64];
            snprintf(label, sizeof(label), "fixed12_%d", i);
            double fp12 = vals[i] * 4096.0;
            emit(label, fp12);
        }
    }

    // === 5. Chained multiply-add (polygon vertex transform) ===
    // x_px = x * scale_x + offset_x; y_px = y * scale_y + offset_y;
    {
        double x = 0.3456789, y = 0.7654321;
        double scale_x = 800.0, scale_y = 600.0;
        double offset_x = 0.0, offset_y = 0.0;
        double x_px = x * scale_x + offset_x;
        double y_px = y * scale_y + offset_y;
        emit("transform_x", x_px);
        emit("transform_y", y_px);
    }

    // === 6. Division chain (slope computation) ===
    {
        double x1 = 100.123456789, y1 = 200.987654321;
        double x2 = 300.111111111, y2 = 400.222222222;
        double dx = x2 - x1;
        double dy = y2 - y1;
        double slope = dx / dy;
        double inv_slope = dy / dx;
        emit("slope", slope);
        emit("inv_slope", inv_slope);
    }

    // === 7. Accumulated sum (text width measurement) ===
    // Simulates measure_text_width summing ~20 character widths.
    {
        double widths[] = {
            0.056, 0.044, 0.056, 0.056, 0.039, 0.056, 0.033,
            0.056, 0.056, 0.022, 0.056, 0.050, 0.056, 0.067,
            0.056, 0.056, 0.056, 0.056, 0.039, 0.044
        };
        double total = 0.0;
        for (int i = 0; i < 20; i++) total += widths[i];
        emit("text_width_sum", total);
        // Also test: total / some_value (as in GetBestLabelTallness)
        double tallness = 0.15 / total;
        emit("label_tallness", tallness);
    }

    // === 8. Near-integer f64 -> i32 truncation boundary ===
    // These are the cases where 1 ULP of f64 difference changes the i32 result.
    {
        // Just below and above integer boundaries
        double near_128 = 128.0 - 1e-15;
        double near_128b = 128.0 + 1e-15;
        double near_neg = -0.0000001;
        emit("trunc_below_128", near_128);
        emit("trunc_above_128", near_128b);
        emit("trunc_near_neg", near_neg);
    }

    // === 9. The exact GetContentRoundRect cascade ===
    // Simplified version of the computation chain that causes D-CHECKBOX-OFFSET.
    {
        double w = 1.0, h = 0.5;  // normalized panel
        double s = (w < h ? w : h) * 1.0;  // min(w,h)
        double d = s * 0.043;
        double rndX = d, rndY = d, rndW = w - 2*d, rndH = h - 2*d;
        double rndR = s * 0.20;
        double ms = (rndW < rndH ? rndW : rndH) * 1.0;
        double label_h = 0.17 * ms;
        double content_y = rndY + label_h + ms * 0.023;
        double content_h = rndH - label_h - ms * 0.023;
        emit("content_y", content_y);
        emit("content_h", content_h);
    }

    // === 10. floor/ceil near boundary (scanline rasterizer) ===
    {
        double v1 = 127.9999999999999;
        double v2 = 128.0000000000001;
        emit("floor_v1", floor(v1));
        emit("ceil_v1", ceil(v1));
        emit("floor_v2", floor(v2));
        emit("ceil_v2", ceil(v2));
    }

    return 0;
}
