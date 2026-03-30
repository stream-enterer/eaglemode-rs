// Trace checkbox geometry through the C++ pipeline.
// Links against libemCore to use the real emBorder/emPainter.
// Compile: g++ -O2 -std=c++17 -I$EM_ROOT/include -I$EM_ROOT \
//          -o trace_checkbox trace_checkbox.cpp \
//          -L$EM_ROOT/lib -Wl,-rpath,$EM_ROOT/lib -lemCore -lm -lpthread -ldl

#include <emCore/emContext.h>
#include <emCore/emPanel.h>
#include <emCore/emToolkit.h>
#include <cstdio>
#include <cstring>
#include <cstdint>

static void hex(const char* label, double v) {
    uint64_t bits;
    memcpy(&bits, &v, 8);
    printf("%s = %.17g [%016llx]\n", label, v, (unsigned long long)bits);
}

int main() {
    // emCheckBox uses OuterBorderType OBT_MARGIN, no inner border.
    // Caption: "Check Option" (same as golden test).
    // label_in_border = false.
    //
    // We need to trace:
    //   1. GetBestLabelTallness()
    //   2. GetContentRect() for a specific (w, h)
    //   3. The box_label_geometry computation
    //
    // Since we can't easily instantiate emCheckBox without a full panel tree,
    // extract the relevant computations directly.

    // Step 1: GetBestLabelTallness for "Check Option" (no icon, no desc)
    const char* caption = "Check Option";
    double charHeight = 1.0;
    double capH;
    double capW = emPainter::GetTextSize(caption, charHeight, true, 0.0, &capH);
    hex("capW", capW);
    hex("capH", capH);
    double tallness = capH / capW;
    hex("tallness_raw", tallness);
    tallness = (tallness > 0.2) ? tallness : 0.2;
    hex("tallness_clamped", tallness);

    // Step 2: GetContentRect for OBT_MARGIN, no inner border
    // Margin outer border: C++ emBorder.cpp line 659-669
    // s_pre = min(w, h) * bordScaling
    // For OBT_MARGIN: d = s_pre * 0.057
    // rndX = d, rndY = d, rndW = w - 2d, rndH = h - 2d
    // Then: no round rect (rndR = 0 for margin), no inner border, no label-in-border.
    //
    // For the golden test: widget rendered at 800x600 viewport, panel at (0,0,800/600,1.0)
    // normalized to w=1.0, h=tallness. Let's use the actual checkbox test dimensions.
    //
    // The checkbox golden test uses 800x600 viewport. Panel layout is
    // (0, 0, w_panel, h_panel) where w_panel = 800/600 * 1.0 and h_panel determined by layout.
    // But we don't know exact values without running the full pipeline.
    //
    // Let's use a simple case: w=1.0, h=0.5 (typical panel aspect).
    double w = 1.0, h = 0.5;

    // OBT_MARGIN outer insets (emBorder.cpp:659-669):
    double s_pre = (w < h ? w : h) * 1.0; // bordScaling = 1.0
    double d_margin = s_pre * 0.057;
    hex("d_margin", d_margin);

    double rndX = d_margin;
    double rndY = d_margin;
    double rndW = w - 2 * d_margin;
    double rndH = h - 2 * d_margin;
    double rndR = 0.0; // Margin has no round rect

    // No HowTo shift for this test.
    // No label_in_border.
    // ms = s_pre * minSpaceFactor. For Margin: minSpaceFactor = 0.023 (C++ line 693).
    double ms = s_pre * 0.023;
    hex("ms", ms);

    // No-label path: symmetric minSpace
    rndX += ms; rndY += ms; rndW -= 2*ms; rndH -= 2*ms;
    rndR -= ms;

    // rndR <= 0, so rec = rnd
    double recX = rndX, recY = rndY, recW = rndW, recH = rndH;

    hex("recX", recX);
    hex("recY", recY);
    hex("recW", recW);
    hex("recH", recH);

    // No inner border for checkbox (InnerBorderType::None).

    // Step 3: box_label_geometry (emButton.cpp:237-260)
    double lw = 1.0;
    double lh = tallness;
    double bw = lh;
    double d_box = bw * 0.1;
    double f = (recW/(bw+d_box+lw) < recH/lh) ? recW/(bw+d_box+lw) : recH/lh;
    hex("f_scale", f);

    bw *= f;
    d_box *= f;
    lw = recW - bw - d_box;
    lh = bw;

    hex("bw", bw);
    hex("bx", recX);
    hex("by", recY + (recH - bw)*0.5);

    // Image inset
    double d_img = bw * 0.13;
    double bx = recX + d_img;
    double by = recY + (recH - bw)*0.5 + d_img;
    bw -= 2*d_img;
    double bh = bw;

    hex("face_bx", bx);
    hex("face_by", by);
    hex("face_bw", bw);

    // Face inset
    double d2 = bw * 30.0/380.0;
    double fx = bx + d2;
    double fy = by + d2;
    double fw = bw - 2*d2;
    double fh = bh - 2*d2;

    hex("fx", fx);
    hex("fy", fy);
    hex("fw", fw);

    // Checkmark vertices
    hex("check_x0", fx+fw*0.2);
    hex("check_y0", fy+fh*0.6);
    hex("check_x1", fx+fw*0.4);
    hex("check_y1", fy+fh*0.8);
    hex("check_x2", fx+fw*0.8);
    hex("check_y2", fy+fh*0.2);

    return 0;
}
