// Standalone C++ rasterizer trace — extracted from emPainter.cpp PaintPolygon.
// Feeds the same arrow pentagon as the Rust trace_rasterizer_polygon test.
// Outputs per-scanline spans in the same format for diff.
//
// Compile: g++ -O2 -o trace_raster trace_raster.cpp -lm
//          (no Eagle Mode dependency — rasterizer logic is inlined)

#include <cstdio>
#include <cmath>
#include <cstdlib>
#include <cstring>
#include <climits>
#include <algorithm>
#include <vector>

// ── Rasterizer extracted from emPainter.cpp:401-716 ──────────────────
// Adapted to work standalone: no ScanlineTool, no texture, just coverage output.

struct ScanEntry {
    double A0, A1, A2;
    ScanEntry* Next;
    int X;
};

struct Span {
    int x_start, x_end;
    int opacity_beg, opacity_mid, opacity_end;
};

static ScanEntry seTerminator = {0.0, 0.0, 0.0, nullptr, INT_MAX};

// `scanlines` is already offset: scanlines[sy] is valid for sy in [sly1, sly2).
static void pp_add_scan_entry(ScanEntry** scanlines, int x, int y,
                               double a0, double a1, double a2,
                               ScanEntry*& freeScanEntries,
                               ScanEntry*& freeScanEntriesEnd,
                               std::vector<std::vector<ScanEntry>>& chunks) {
    ScanEntry** ppse = &scanlines[y];
    ScanEntry* pse = *ppse;
    while (pse->X < x) {
        ppse = &pse->Next;
        pse = *ppse;
    }
    if (pse->X == x) {
        pse->A0 += a0;
        pse->A1 += a1;
        pse->A2 += a2;
    } else {
        if (freeScanEntries >= freeScanEntriesEnd) {
            chunks.emplace_back(2048);
            freeScanEntries = chunks.back().data();
            freeScanEntriesEnd = freeScanEntries + 2048;
        }
        pse = freeScanEntries++;
        pse->Next = *ppse;
        *ppse = pse;
        pse->A0 = a0;
        pse->A1 = a1;
        pse->A2 = a2;
        pse->X = x;
    }
}

static std::vector<std::pair<int, std::vector<Span>>> rasterize(
    const double* xy, int n,
    double ClipX1, double ClipY1, double ClipX2, double ClipY2,
    double ScaleX, double ScaleY, double OriginX, double OriginY)
{
    if (n < 3) return {};

    double minX, maxX, minY, maxY;
    minX = maxX = xy[0];
    minY = maxY = xy[1];
    const double* pxy = xy + n * 2 - 2;
    do {
        if (maxX < pxy[0]) maxX = pxy[0];
        else if (minX > pxy[0]) minX = pxy[0];
        if (maxY < pxy[1]) maxY = pxy[1];
        else if (minY > pxy[1]) minY = pxy[1];
        pxy -= 2;
    } while (pxy > xy);

    minY = minY * ScaleY + OriginY;
    if (minY < ClipY1) minY = ClipY1;
    maxY = maxY * ScaleY + OriginY;
    if (maxY > ClipY2) maxY = ClipY2;
    if (minY >= maxY) return {};
    minX = minX * ScaleX + OriginX;
    if (minX < ClipX1) minX = ClipX1;
    maxX = maxX * ScaleX + OriginX;
    if (maxX > ClipX2 - 0.0001) maxX = ClipX2 - 0.0001;
    if (minX >= maxX) return {};

    int sly1 = (int)minY;
    int sly2 = (int)ceil(maxY);

    // Allocate scanline heads.
    int slCount = sly2 - sly1 + 2;
    auto slmem = std::vector<ScanEntry*>(slCount, &seTerminator);
    ScanEntry** scanlines = slmem.data() - sly1 + 1;

    ScanEntry autoEntries[1024];
    ScanEntry* freeScanEntries = autoEntries;
    ScanEntry* freeScanEntriesEnd = freeScanEntries + 1024;
    std::vector<std::vector<ScanEntry>> chunks;

    double x0 = xy[0] * ScaleX + OriginX;
    double y0 = xy[1] * ScaleY + OriginY;

    for (pxy = xy + n * 2 - 2; pxy >= xy; pxy -= 2) {
        double y1p = y0;
        y0 = pxy[1] * ScaleY + OriginY;
        double x1, y1, x2, y2, va;
        if (y1p > y0) {
            y2 = y1p; y1 = y0;
            x2 = x0;
            x1 = x0 = pxy[0] * ScaleX + OriginX;
            va = 0x1000;
        } else {
            y2 = y0; y1 = y1p;
            x1 = x0;
            x2 = x0 = pxy[0] * ScaleX + OriginX;
            va = -0x1000;
        }
        if (y1 >= maxY || y2 <= minY) continue;
        if (y1 < minY) {
            if (y2 - y1 >= 0.0001) x1 += (minY - y1) * (x2 - x1) / (y2 - y1);
            y1 = minY;
        }
        if (y2 > maxY) {
            if (y2 - y1 >= 0.0001) x2 += (maxY - y2) * (x2 - x1) / (y2 - y1);
            y2 = maxY;
        }

        int i = 0;
        double ex1[2], ey1[2], ex2[2], ey2[2];

        if (x1 < x2) {
            if (x1 < minX) {
                if (x2 > minX && x2 - x1 >= 0.0001) {
                    ey1[0] = y1;
                    y1 += (minX - x1) * (y2 - y1) / (x2 - x1);
                    ey2[0] = y1;
                    ex1[0] = ex2[0] = x1 = minX;
                    i = 1;
                } else { x1 = x2 = minX; }
            }
            if (x2 > maxX) {
                if (x1 < maxX && x2 - x1 >= 0.0001) {
                    ey2[i] = y2;
                    y2 += (maxX - x2) * (y2 - y1) / (x2 - x1);
                    ey1[i] = y2;
                    ex1[i] = ex2[i] = x2 = maxX;
                    i++;
                } else { x1 = x2 = maxX; }
            }
        } else {
            if (x1 > maxX) {
                if (x2 < maxX && x2 - x1 <= -0.0001) {
                    ey1[0] = y1;
                    y1 += (maxX - x1) * (y2 - y1) / (x2 - x1);
                    ey2[0] = y1;
                    ex1[0] = ex2[0] = x1 = maxX;
                    i = 1;
                } else { x1 = x2 = maxX; }
            }
            if (x2 < minX) {
                if (x1 > minX && x2 - x1 <= -0.0001) {
                    ey2[i] = y2;
                    y2 += (minX - x2) * (y2 - y1) / (x2 - x1);
                    ey1[i] = y2;
                    ex1[i] = ex2[i] = x2 = minX;
                    i++;
                } else { x1 = x2 = minX; }
            }
        }

        for (;;) {
            double dy = y2 - y1;
            if (dy >= 0.0001) {
                int sy = (int)y1;
                int sy2 = ((int)ceil(y2)) - 1;
                double ax = floor(x1);
                int sx = (int)ax;
                double t = ax + 1.0 - x1;
                double dx = x2 - x1;

                if (dx >= 0.0001 || dx <= -0.0001) {
                    double a2 = va * dy / dx;
                    double a0 = t * t * 0.5 * a2;
                    double a1 = (t + 0.5) * a2;
                    dx /= dy;
                    x1 += (sy + 1 - y1) * dx;
                    for (;;) {
                        if (sy >= sy2) {
                            if (sy > sy2) break;
                            x1 = x2;
                        }
                        pp_add_scan_entry(scanlines, sx, sy, a0, a1, a2,
                                          freeScanEntries, freeScanEntriesEnd, chunks);
                        ax = floor(x1);
                        sx = (int)ax;
                        t = ax + 1.0 - x1;
                        a0 = t * t * 0.5 * a2;
                        a1 = (t + 0.5) * a2;
                        pp_add_scan_entry(scanlines, sx, sy, -a0, -a1, -a2,
                                          freeScanEntries, freeScanEntriesEnd, chunks);
                        x1 += dx;
                        sy++;
                    }
                } else {
                    double a1 = va * (sy + 1 - y1);
                    for (;;) {
                        if (sy >= sy2) {
                            if (sy > sy2) break;
                            a1 -= va * (sy2 + 1 - y2);
                        }
                        double a0 = t * a1;
                        pp_add_scan_entry(scanlines, sx, sy, a0, a1, 0.0,
                                          freeScanEntries, freeScanEntriesEnd, chunks);
                        a1 = va;
                        sy++;
                    }
                }
            }
            if (!i) break;
            i--;
            x1 = ex1[i]; y1 = ey1[i]; x2 = ex2[i]; y2 = ey2[i];
        }
    }

    // Coverage walk — extracted from emPainter.cpp:637-716
    std::vector<std::pair<int, std::vector<Span>>> result;

    for (int sy = sly1; sy < sly2; sy++) {
        ScanEntry* pse = scanlines[sy];
        if (pse == &seTerminator) continue;

        std::vector<Span> spans;
        double a1 = 0, a2 = 0;
        int sx = pse->X;

        do {
            double a0 = a1;
            a1 += a2;
            if (pse->X == sx) {
                a0 += pse->A0;
                a1 += pse->A1;
                a2 += pse->A2;
                pse = pse->Next;
            }
            int sx0 = sx;
            sx++;
            int alpha = (int)(a0 >= 0 ? 0.5 + a0 : 0.5 - a0);
            if (!alpha) {
                if (pse->X > sx && pse != &seTerminator) {
                    double t = a1 + a2 * (pse->X - 1 - sx);
                    int ta = (int)(t >= 0 ? 0.5 + t : 0.5 - t);
                    if (alpha == ta) {
                        a1 = t + a2;
                        sx = pse->X;
                    }
                }
                continue;
            }
            if (pse == &seTerminator) {
                spans.push_back({sx0, sx0 + 1, alpha, alpha, alpha});
                break;
            }
            // Second pixel
            a0 = a1; a1 += a2;
            if (pse->X == sx) {
                a0 += pse->A0; a1 += pse->A1; a2 += pse->A2;
                pse = pse->Next;
            }
            sx++;
            int alpha2 = (int)(a0 >= 0 ? 0.5 + a0 : 0.5 - a0);
            if (!alpha2) {
                spans.push_back({sx0, sx0 + 1, alpha, alpha, alpha});
                continue;
            }
            if (pse == &seTerminator) {
                spans.push_back({sx0, sx0 + 2, alpha, alpha, alpha2});
                break;
            }
            // Skip optimization
            if (pse->X > sx) {
                double t = a1 + a2 * (pse->X - 1 - sx);
                int ta = (int)(t >= 0 ? 0.5 + t : 0.5 - t);
                if (alpha2 == ta) {
                    a1 = t + a2;
                    sx = pse->X;
                }
            }
            // Third pixel
            a0 = a1; a1 += a2;
            if (pse->X == sx) {
                a0 += pse->A0; a1 += pse->A1; a2 += pse->A2;
                pse = pse->Next;
            }
            sx++;
            int alpha3 = (int)(a0 >= 0 ? 0.5 + a0 : 0.5 - a0);
            if (!alpha3) {
                spans.push_back({sx0, sx - 1, alpha, alpha2, alpha2});
            } else {
                spans.push_back({sx0, sx, alpha, alpha2, alpha3});
            }
        } while (pse != &seTerminator);

        if (!spans.empty()) {
            result.push_back({sy, std::move(spans)});
        }
    }

    return result;
}

int main() {
    // Same arrow pentagon as Rust test, in pixel space (identity transform).
    double tx = 20.0, e = 3.5, ry = 5.0, ay = 10.0, ah = 45.0;
    double xy[] = {
        tx - e, ry,
        tx + e, ry,
        tx + e, ay + ah - e,
        tx,     ay + ah,
        tx - e, ay + ah - e,
    };
    int n = 5;

    auto rows = rasterize(xy, n, 0.0, 0.0, 40.0, 60.0, 1.0, 1.0, 0.0, 0.0);
    for (auto& [y, spans] : rows) {
        for (auto& s : spans) {
            printf("SPAN y=%d x=%d..%d beg=%d mid=%d end=%d\n",
                   y, s.x_start, s.x_end, s.opacity_beg, s.opacity_mid, s.opacity_end);
        }
    }
    return 0;
}
