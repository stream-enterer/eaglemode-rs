#!/usr/bin/env python3
"""Generate file_mapping.json: maps every C++ emCore header to Rust target(s).

Reads:
  - ~/git/eaglemode-0.96.4/include/emCore/*.h  (the 90 C++ headers)
  - stale/state/run_002/feature_list.json       (672 features, 1533 provenance items)
  - src/**/*.rs                                  (the Rust source tree)

Writes:
  - scripts/file_mapping.json
"""

import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

ZUICCHINI = Path(__file__).resolve().parent.parent
EAGLEMODE = Path.home() / "git" / "eaglemode-0.96.4"
HEADERS_DIR = EAGLEMODE / "include" / "emCore"
FEATURE_LIST = ZUICCHINI / "stale" / "state" / "run_002" / "feature_list.json"
SRC_DIR = ZUICCHINI / "src"
OUTPUT = ZUICCHINI / "scripts" / "file_mapping.json"


def load_headers():
    """Return sorted list of .h filenames in include/emCore/."""
    return sorted(p.name for p in HEADERS_DIR.glob("*.h"))


def load_rust_files():
    """Return sorted list of .rs files relative to src/, excluding mod.rs and lib.rs."""
    files = []
    for p in SRC_DIR.rglob("*.rs"):
        rel = p.relative_to(SRC_DIR)
        name = rel.name
        if name in ("mod.rs", "lib.rs"):
            continue
        files.append(str(rel))
    return sorted(files)


def load_features():
    with open(FEATURE_LIST) as f:
        return json.load(f)["features"]


def normalize_rust_target(rt):
    """Extract just the file path from a rust_target string.

    Examples:
      "foundation/em_rec.rs :: RecStruct" -> "foundation/em_rec.rs"
      "render/painter.rs:Painter::clear" -> "render/painter.rs"
      "foundation/em_rec.rs (RecValue enum) + model/record.rs (Record trait)"
        -> ["foundation/em_rec.rs", "model/record.rs"]
    """
    if not rt or not rt.strip():
        return []

    # Handle "file1 + file2" pattern
    if " + " in rt:
        parts = rt.split(" + ")
        result = []
        for part in parts:
            result.extend(normalize_rust_target(part.strip()))
        return result

    # Remove :: scope notation
    rt = re.sub(r"\s*::.*", "", rt)
    # Remove :Method notation
    rt = re.sub(r":(?!/).*", "", rt)
    # Remove parenthetical notes
    rt = re.sub(r"\s*\(.*?\)", "", rt)
    rt = rt.strip()

    if rt.endswith(".rs"):
        return [rt]
    return []


def extract_class_prefix(cpp_symbol):
    """Extract the top-level class name from a C++ symbol.

    "emClipRects::GetX1" -> "emClipRects"
    "emColor::BLACK" -> "emColor"
    "emRec" -> "emRec"
    "EM_KEY_0 through EM_KEY_9..." -> None (not a class)
    """
    sym = cpp_symbol.strip()

    # Skip descriptive multi-word entries that aren't real class names
    if " " in sym and "::" not in sym:
        return None

    # Get the first part before ::
    base = sym.split("::")[0].strip()

    # Must start with "em" and be a valid identifier
    if re.match(r"^em[A-Z]\w*$", base):
        return base
    return None


def build_feature_mapping(features):
    """Build header -> set of rust files from feature_list.json."""
    # Maps emFoo -> set of rust file paths
    class_to_rust = defaultdict(set)

    for feat in features:
        rust_targets = normalize_rust_target(feat.get("rust_target", ""))
        for prov in feat.get("cpp_provenance", []):
            prefix = extract_class_prefix(prov["cpp_symbol"])
            if prefix:
                for rt in rust_targets:
                    class_to_rust[prefix].add(rt)

    # Convert class names to header names
    header_to_rust = defaultdict(set)
    for cls, rust_files in class_to_rust.items():
        header = cls + ".h"
        header_to_rust[header].update(rust_files)

    return header_to_rust


# ── Hardcoded knowledge ────────────────────────────────────────────────
# These mappings cannot be derived from feature_list.json alone because
# either the header has no features, or the Rust file has no provenance,
# or the naming diverges.

NO_RUST_EQUIVALENT = {
    "emAnything.h": "Rust uses Box<dyn Any> / std::any::Any",
    "emArray.h": "Rust uses Vec<T>",
    "emAvlTree.h": "Rust uses BTreeMap / HashMap",
    "emAvlTreeMap.h": "Rust uses HashMap / BTreeMap",
    "emAvlTreeSet.h": "Rust uses HashSet / BTreeSet",
    "emCrossPtr.h": "Rust uses Weak<T> references",
    "emFileStream.h": "Rust uses std::fs / std::io",
    "emList.h": "Rust uses Vec / VecDeque",
    "emOwnPtr.h": "Rust uses Box<T>",
    "emOwnPtrArray.h": "Rust uses Vec<Box<T>>",
    "emRef.h": "Rust uses Rc<T> / Arc<T>",
    "emString.h": "Rust uses String / &str",
    "emThread.h": "Rust uses std::thread; single-threaded UI tree",
    "emTmpFile.h": "Rust uses tempfile crate / std::env::temp_dir",
    "emToolkit.h": "Umbrella header — just #includes other widget headers",
}

# Headers whose symbols are scattered or have special Rust mapping
MANUAL_OVERRIDES = {
    # emStd1.h contains emDLog, emFatalError, emSetFatalErrorGraphical + primitives/macros
    "emStd1.h": {
        "source_files": [
            "foundation/dlog.rs",
            "foundation/mod.rs (set_fatal_error_graphical, is_fatal_error_graphical)",
        ],
        "pattern": "split",
        "note": "Most of emStd1.h is Rust std prelude; emDLog→dlog.rs, emFatalError/emSetFatalErrorGraphical→mod.rs functions",
    },
    # emStd2.h contains emCalcAdler32/CRC32/CRC64/HashCode/HashName + file/process/time utilities
    "emStd2.h": {
        "source_files": [
            "foundation/checksum.rs",
        ],
        "pattern": "split",
        "note": "emCalcAdler32 etc. in checksum.rs; most emStd2.h file/process utilities covered by Rust std",
    },
    # emImageFile.h contains both emImageFileModel and emImageFilePanel
    "emImageFile.h": {
        "source_files": [
            "model/image_file_model.rs",
            "widget/image_file_panel.rs",
        ],
        "pattern": "split",
        "note": "C++ header has both model and panel; Rust splits them",
    },
    # emPanel.h is a large class split into multiple Rust files
    "emPanel.h": {
        "source_files": [
            "panel/behavior.rs",
            "panel/ctx.rs",
            "panel/tree.rs",
        ],
        "pattern": "split",
        "note": "Large class split: behavior (trait), ctx (panel context), tree (panel tree)",
    },
    # emView.h maps to view.rs
    "emView.h": {
        "source_files": ["panel/view.rs"],
        "pattern": "rename",
    },
    # emViewAnimator.h maps to animator.rs
    "emViewAnimator.h": {
        "source_files": ["panel/animator.rs"],
        "pattern": "rename",
    },
    # emViewInputFilter.h maps to input_filter.rs
    "emViewInputFilter.h": {
        "source_files": ["panel/input_filter.rs"],
        "pattern": "rename",
    },
    # emSubViewPanel.h maps to sub_view_panel.rs
    "emSubViewPanel.h": {
        "source_files": ["panel/sub_view_panel.rs"],
        "pattern": "rename",
    },
    # emGUIFramework.h -> App in window/app.rs
    "emGUIFramework.h": {
        "source_files": ["window/app.rs"],
        "pattern": "rename",
        "note": "emGUIFramework class → App struct",
    },
    # emWindow.h -> zui_window.rs + platform.rs
    "emWindow.h": {
        "source_files": ["window/zui_window.rs", "window/platform.rs"],
        "pattern": "split",
        "note": "emWindow class in zui_window.rs; platform beep/screensaver inhibit in platform.rs",
    },
    # emWindowStateSaver.h -> state_saver.rs
    "emWindowStateSaver.h": {
        "source_files": ["window/state_saver.rs"],
        "pattern": "rename",
    },
    # emScreen.h -> screen.rs
    "emScreen.h": {
        "source_files": ["window/screen.rs"],
        "pattern": "rename",
    },
    # emInput.h -> input/event.rs + input/state.rs
    "emInput.h": {
        "source_files": ["input/event.rs", "input/state.rs"],
        "pattern": "split",
        "note": "emInput class split into InputEvent/InputKey and InputState",
    },
    # emCursor.h -> input/cursor.rs
    "emCursor.h": {
        "source_files": ["input/cursor.rs"],
        "pattern": "rename",
    },
    # emInputHotkey is not in emInput.h — it's a separate header
    # emScheduler.h -> scheduler/core.rs
    "emScheduler.h": {
        "source_files": ["scheduler/core.rs"],
        "pattern": "rename",
    },
    # emEngine.h -> scheduler/engine.rs
    "emEngine.h": {
        "source_files": ["scheduler/engine.rs"],
        "pattern": "rename",
    },
    # emJob.h -> scheduler/job.rs
    "emJob.h": {
        "source_files": ["scheduler/job.rs"],
        "pattern": "rename",
    },
    # emSignal.h -> scheduler/signal.rs
    "emSignal.h": {
        "source_files": ["scheduler/signal.rs"],
        "pattern": "rename",
    },
    # emTimer.h -> scheduler/timer.rs
    "emTimer.h": {
        "source_files": ["scheduler/timer.rs"],
        "pattern": "rename",
    },
    # emPriSchedAgent.h -> scheduler/pri_sched_agent.rs
    "emPriSchedAgent.h": {
        "source_files": ["scheduler/pri_sched_agent.rs"],
        "pattern": "rename",
    },
    # emContext.h -> model/context.rs
    "emContext.h": {
        "source_files": ["model/context.rs"],
        "pattern": "rename",
    },
    # emModel.h -> own file; code currently in model/context.rs, needs extraction
    "emModel.h": {
        "source_files": ["model/context.rs"],
        "pattern": "extract",
        "note": "emModel base class currently in context.rs; extract to own file",
    },
    # emConfigModel.h -> model/config_model.rs
    "emConfigModel.h": {
        "source_files": ["model/config_model.rs"],
        "pattern": "rename",
    },
    # emFileModel.h -> model/file_model.rs
    "emFileModel.h": {
        "source_files": ["model/file_model.rs"],
        "pattern": "rename",
    },
    # emRecFileModel.h -> model/rec_file_model.rs
    "emRecFileModel.h": {
        "source_files": ["model/rec_file_model.rs"],
        "pattern": "rename",
    },
    # emCoreConfig.h -> model/core_config.rs
    "emCoreConfig.h": {
        "source_files": ["model/core_config.rs"],
        "pattern": "rename",
    },
    # emClipboard.h -> model/clipboard.rs
    "emClipboard.h": {
        "source_files": ["model/clipboard.rs"],
        "pattern": "rename",
    },
    # emFpPlugin.h -> model/fp_plugin.rs
    "emFpPlugin.h": {
        "source_files": ["model/fp_plugin.rs"],
        "pattern": "rename",
    },
    # emVarModel.h -> model/watched_var.rs
    "emVarModel.h": {
        "source_files": ["model/watched_var.rs"],
        "pattern": "rename",
        "note": "emVarModel<T> → WatchedVar<T>",
    },
    # emVarSigModel.h -> own file; code currently in model/watched_var.rs, needs extraction
    "emVarSigModel.h": {
        "source_files": ["model/watched_var.rs"],
        "pattern": "extract",
        "note": "emVarSigModel currently in watched_var.rs; extract to own file",
    },
    # emSigModel.h -> own file; code currently in model/watched_var.rs, needs extraction
    "emSigModel.h": {
        "source_files": ["model/watched_var.rs"],
        "pattern": "extract",
        "note": "emSigModel currently in watched_var.rs; extract to own file",
    },
    # emRec.h -> foundation/em_rec.rs + model/rec_types.rs + model/record.rs
    "emRec.h": {
        "source_files": [
            "foundation/em_rec.rs",
            "model/rec_types.rs",
            "model/record.rs",
        ],
        "pattern": "split",
        "note": "emRec class hierarchy split: parsing in em_rec.rs, rec types in rec_types.rs, Record trait in record.rs",
    },
    # emRes.h -> model/resource_cache.rs + foundation/tga.rs
    "emRes.h": {
        "source_files": ["model/resource_cache.rs", "foundation/tga.rs"],
        "pattern": "split",
        "note": "emResModelBase → ResourceCache; TGA loading (emGetResImage uses TGA) → tga.rs",
    },
    # emColor.h -> foundation/color.rs + foundation/x11_colors.rs
    "emColor.h": {
        "source_files": ["foundation/color.rs", "foundation/x11_colors.rs"],
        "pattern": "split",
        "note": "emColor class in color.rs; X11 color name table (for FromName) in x11_colors.rs",
    },
    # emImage.h -> foundation/image.rs
    "emImage.h": {
        "source_files": ["foundation/image.rs"],
        "pattern": "rename",
    },
    # emClipRects.h -> foundation/clip_rects.rs
    "emClipRects.h": {
        "source_files": ["foundation/clip_rects.rs"],
        "pattern": "rename",
    },
    # emATMatrix.h -> foundation/at_matrix.rs
    "emATMatrix.h": {
        "source_files": ["foundation/at_matrix.rs"],
        "pattern": "rename",
    },
    # emInstallInfo.h -> foundation/install_info.rs
    "emInstallInfo.h": {
        "source_files": ["foundation/install_info.rs"],
        "pattern": "rename",
    },
    # emMiniIpc.h -> foundation/mini_ipc.rs
    "emMiniIpc.h": {
        "source_files": ["foundation/mini_ipc.rs"],
        "pattern": "rename",
    },
    # emProcess.h -> foundation/process.rs
    "emProcess.h": {
        "source_files": ["foundation/process.rs"],
        "pattern": "rename",
    },
    # emPainter.h -> render/painter.rs + render/interpolation.rs + render/scanline*.rs + render/draw_list.rs
    "emPainter.h": {
        "source_files": [
            "render/painter.rs",
            "render/interpolation.rs",
            "render/scanline.rs",
            "render/scanline_avx2.rs",
            "render/scanline_tool.rs",
            "render/draw_list.rs",
        ],
        "pattern": "split",
        "note": "emPainter split: main API in painter.rs, interpolation/scanline/draw_list for parallel rendering",
    },
    # emStroke.h -> render/stroke.rs
    "emStroke.h": {
        "source_files": ["render/stroke.rs"],
        "pattern": "rename",
    },
    # emStrokeEnd.h -> own file; code currently in render/stroke.rs, needs extraction
    "emStrokeEnd.h": {
        "source_files": ["render/stroke.rs"],
        "pattern": "extract",
        "note": "emStrokeEnd currently in stroke.rs; extract to own file",
    },
    # emTexture.h -> render/texture.rs
    "emTexture.h": {
        "source_files": ["render/texture.rs"],
        "pattern": "rename",
    },
    # emFontCache.h -> render/em_font.rs + render/bitmap_font.rs
    "emFontCache.h": {
        "source_files": ["render/em_font.rs", "render/bitmap_font.rs"],
        "pattern": "split",
        "note": "emFontCache split: font loading/caching in em_font.rs, text measurement in bitmap_font.rs",
    },
    # emRenderThreadPool.h -> render/thread_pool.rs
    "emRenderThreadPool.h": {
        "source_files": ["render/thread_pool.rs"],
        "pattern": "rename",
    },
    # emViewRenderer.h -> render/software_compositor.rs + render/compositor.rs + render/tile_cache.rs
    "emViewRenderer.h": {
        "source_files": [
            "render/software_compositor.rs",
            "render/compositor.rs",
            "render/tile_cache.rs",
        ],
        "pattern": "split",
        "note": "emViewRenderer split: software path, WGPU path, and tile cache",
    },
    # emBorder.h -> widget/border.rs + foundation/alignment.rs
    "emBorder.h": {
        "source_files": ["widget/border.rs", "foundation/alignment.rs"],
        "pattern": "split",
        "note": "emBorder class in border.rs; ContentAlignment (emAlignment) in alignment.rs",
    },
    # emButton.h -> widget/button.rs
    "emButton.h": {
        "source_files": ["widget/button.rs"],
        "pattern": "rename",
    },
    # emCheckBox.h -> widget/check_box.rs
    "emCheckBox.h": {
        "source_files": ["widget/check_box.rs"],
        "pattern": "rename",
    },
    # emCheckButton.h -> widget/check_button.rs
    "emCheckButton.h": {
        "source_files": ["widget/check_button.rs"],
        "pattern": "rename",
    },
    # emColorField.h -> widget/color_field.rs + widget/field_panel.rs
    "emColorField.h": {
        "source_files": ["widget/color_field.rs", "widget/field_panel.rs"],
        "pattern": "split",
        "note": "emColorField in color_field.rs; ScalarFieldPanel wrapper (used by ColorField expansion) in field_panel.rs",
    },
    # emCoreConfigPanel.h -> widget/core_config_panel.rs
    "emCoreConfigPanel.h": {
        "source_files": ["widget/core_config_panel.rs"],
        "pattern": "rename",
    },
    # emDialog.h -> widget/dialog.rs
    "emDialog.h": {
        "source_files": ["widget/dialog.rs"],
        "pattern": "rename",
    },
    # emErrorPanel.h -> widget/error_panel.rs
    "emErrorPanel.h": {
        "source_files": ["widget/error_panel.rs"],
        "pattern": "rename",
    },
    # emFileDialog.h -> widget/file_dialog.rs
    "emFileDialog.h": {
        "source_files": ["widget/file_dialog.rs"],
        "pattern": "rename",
    },
    # emFilePanel.h -> widget/file_panel.rs
    "emFilePanel.h": {
        "source_files": ["widget/file_panel.rs"],
        "pattern": "rename",
    },
    # emFileSelectionBox.h -> widget/file_selection_box.rs
    "emFileSelectionBox.h": {
        "source_files": ["widget/file_selection_box.rs"],
        "pattern": "rename",
    },
    # emLabel.h -> widget/label.rs
    "emLabel.h": {
        "source_files": ["widget/label.rs"],
        "pattern": "rename",
    },
    # emListBox.h -> widget/list_box.rs
    "emListBox.h": {
        "source_files": ["widget/list_box.rs"],
        "pattern": "rename",
    },
    # emLook.h -> widget/look.rs
    "emLook.h": {
        "source_files": ["widget/look.rs"],
        "pattern": "rename",
    },
    # emRadioBox.h -> widget/radio_box.rs
    "emRadioBox.h": {
        "source_files": ["widget/radio_box.rs"],
        "pattern": "rename",
    },
    # emRadioButton.h -> widget/radio_button.rs
    "emRadioButton.h": {
        "source_files": ["widget/radio_button.rs"],
        "pattern": "rename",
    },
    # emScalarField.h -> widget/scalar_field.rs
    "emScalarField.h": {
        "source_files": ["widget/scalar_field.rs"],
        "pattern": "rename",
    },
    # emSplitter.h -> widget/splitter.rs
    "emSplitter.h": {
        "source_files": ["widget/splitter.rs"],
        "pattern": "rename",
    },
    # emTextField.h -> widget/text_field.rs
    "emTextField.h": {
        "source_files": ["widget/text_field.rs"],
        "pattern": "rename",
    },
    # emTunnel.h -> widget/tunnel.rs
    "emTunnel.h": {
        "source_files": ["widget/tunnel.rs"],
        "pattern": "rename",
    },
    # emTiling.h -> part of layout system, types in layout/mod.rs
    "emTiling.h": {
        "source_files": [
            "layout/mod.rs (Orientation, Alignment, Spacing, ChildConstraint)",
        ],
        "pattern": "rename",
        "note": "emTiling layout types moved to layout module-level code",
    },
    # emLinearLayout.h -> layout/linear.rs
    "emLinearLayout.h": {
        "source_files": ["layout/linear.rs"],
        "pattern": "rename",
    },
    # emLinearGroup.h -> own file; code currently in layout/linear.rs, needs extraction
    "emLinearGroup.h": {
        "source_files": ["layout/linear.rs"],
        "pattern": "extract",
        "note": "emLinearGroup currently in linear.rs; extract to own file",
    },
    # emPackLayout.h -> layout/pack.rs
    "emPackLayout.h": {
        "source_files": ["layout/pack.rs"],
        "pattern": "rename",
    },
    # emPackGroup.h -> own file; code currently in layout/pack.rs, needs extraction
    "emPackGroup.h": {
        "source_files": ["layout/pack.rs"],
        "pattern": "extract",
        "note": "emPackGroup currently in pack.rs; extract to own file",
    },
    # emRasterLayout.h -> layout/raster.rs
    "emRasterLayout.h": {
        "source_files": ["layout/raster.rs"],
        "pattern": "rename",
    },
    # emRasterGroup.h -> own file; code currently in layout/raster.rs, needs extraction
    "emRasterGroup.h": {
        "source_files": ["layout/raster.rs"],
        "pattern": "extract",
        "note": "emRasterGroup currently in raster.rs; extract to own file",
    },
    # emGroup.h -> own file; deprecated in C++, code currently in layout/mod.rs
    "emGroup.h": {
        "source_files": [
            "layout/mod.rs (emGroup extends emTiling — deprecated thin wrapper)",
        ],
        "pattern": "extract",
        "note": "Deprecated C++ class; extract to own file",
    },
    # emInputHotkey -> input/hotkey.rs (custom name, different from class name)
    # Actually: the class is emInputHotkey, header is emInput.h? No — there's a separate header.
    # Let me check... there's no emInputHotkey.h in the list. But there's emInput.h.
    # Actually wait — looking at the headers list, there IS no emInputHotkey.h.
    # The feature_list maps emInputHotkey to input/hotkey.rs.
    # emInput.h likely contains emInputEvent, emInputKey, emInputState, emInputHotkey.
    # Let me adjust the emInput.h mapping to include hotkey.rs too.
}

# Fix: emInput.h should include hotkey.rs
MANUAL_OVERRIDES["emInput.h"] = {
    "source_files": ["input/event.rs", "input/state.rs", "input/hotkey.rs"],
    "pattern": "split",
    "note": "emInput.h contains emInputEvent, emInputKey, emInputState; emInputHotkey in separate but related header",
}


def derive_target_name(header):
    """Derive the target emCore/*.rs filename from a header name.

    emFoo.h -> emFoo.rs
    """
    return header.replace(".h", ".rs")


def determine_pattern(header, source_files):
    """Determine the mapping pattern."""
    if len(source_files) == 0:
        return "no-rust-equivalent"
    elif len(source_files) == 1:
        return "rename"
    else:
        return "split"


def build_mapping():
    headers = load_headers()
    rust_files = load_rust_files()
    features = load_features()

    # Build feature-based mapping
    feature_map = build_feature_mapping(features)

    # Collect all cpp_symbols per header from features
    header_symbols = defaultdict(set)
    for feat in features:
        for prov in feat.get("cpp_provenance", []):
            prefix = extract_class_prefix(prov["cpp_symbol"])
            if prefix:
                header_symbols[prefix + ".h"].add(prov["cpp_symbol"])

    mapping = {}
    accounted_rust_files = set()

    for header in headers:
        if header in NO_RUST_EQUIVALENT:
            marker = "src/emCore/" + header.replace(".h", ".no_rust_equivalent")
            mapping[header] = {
                "target_rs": None,
                "marker_file": marker,
                "source_files": [],
                "mod_rs_code": None,
                "pattern": "no-rust-equivalent",
                "reason": NO_RUST_EQUIVALENT[header],
                "cpp_symbols": sorted(header_symbols.get(header, [])),
            }
            continue

        if header in MANUAL_OVERRIDES:
            override = MANUAL_OVERRIDES[header]
            source_files = override["source_files"]
            pattern = override.get("pattern", determine_pattern(header, source_files))

            target_rs = "src/emCore/" + derive_target_name(header)

            # Separate mod.rs code references from regular files
            real_sources = []
            mod_rs_code = None
            for sf in source_files:
                if "mod.rs" in sf:
                    mod_rs_code = sf
                else:
                    real_sources.append("src/" + sf if not sf.startswith("src/") else sf)
                    # Track for accounting
                    clean = sf.replace("src/", "") if sf.startswith("src/") else sf
                    accounted_rust_files.add(clean)

            mapping[header] = {
                "target_rs": target_rs,
                "source_files": real_sources,
                "mod_rs_code": mod_rs_code,
                "pattern": pattern,
                "note": override.get("note"),
                "cpp_symbols": sorted(header_symbols.get(header, [])),
            }
            continue

        # Try feature-based mapping
        feature_sources = feature_map.get(header, set())
        if feature_sources:
            source_files = sorted("src/" + f for f in feature_sources)
            for f in feature_sources:
                accounted_rust_files.add(f)
            pattern = determine_pattern(header, source_files)
            target_rs = "src/emCore/" + derive_target_name(header)
            mapping[header] = {
                "target_rs": target_rs,
                "source_files": source_files,
                "mod_rs_code": None,
                "pattern": pattern,
                "cpp_symbols": sorted(header_symbols.get(header, [])),
            }
            continue

        # No mapping found — mark as unmapped for manual review
        mapping[header] = {
            "target_rs": "src/emCore/" + derive_target_name(header),
            "source_files": [],
            "mod_rs_code": None,
            "pattern": "no-rust-equivalent",
            "reason": "UNMAPPED — no features found, needs manual review",
            "cpp_symbols": sorted(header_symbols.get(header, [])),
        }

    # Rust-only files (no C++ header equivalent)
    # target_rs is the Phase 3 destination; None means file stays in place
    # marker_file is the .rust_only marker created alongside the .rs file
    RUST_ONLY = {
        "debug/input_trace.rs": {
            "reason": "Rust-only debug utility for input event tracing",
            "target_rs": None,  # stays in src/debug/ (exempt from flatten)
            "marker_file": None,  # no marker for debug-only test file
        },
        "foundation/rect.rs": {
            "reason": "Rust-specific Rect/PixelRect newtypes; C++ passes loose (x,y,w,h) doubles",
            "target_rs": "src/emCore/rect.rs",
            "marker_file": "src/emCore/rect.rust_only",
        },
        "foundation/fixed.rs": {
            "reason": "Rust-specific Fixed12 newtype for sub-pixel math; C++ uses inline integer arithmetic",
            "target_rs": "src/emCore/fixed.rs",
            "marker_file": "src/emCore/fixed.rust_only",
        },
        "widget/toolkit_images.rs": {
            "reason": "Rust-specific TGA resource loader; C++ loads toolkit images via emGetInsResImage at point of use",
            "target_rs": "src/emCore/toolkit_images.rs",
            "marker_file": "src/emCore/toolkit_images.rust_only",
        },
    }
    for rf in RUST_ONLY:
        accounted_rust_files.add(rf)

    # Remove mod.rs entries from accounting (they're not in rust_files)
    accounted_rust_files = {f for f in accounted_rust_files if not f.endswith("mod.rs")}

    # Check which Rust files are NOT accounted for
    unaccounted = []
    for rf in rust_files:
        if rf not in accounted_rust_files:
            unaccounted.append(rf)

    # Summary
    total_headers = len(headers)
    mapped_headers = sum(1 for h, m in mapping.items() if m["pattern"] != "no-rust-equivalent")
    no_equiv = sum(1 for h, m in mapping.items() if m["pattern"] == "no-rust-equivalent" and "UNMAPPED" not in m.get("reason", ""))
    unmapped = sum(1 for h, m in mapping.items() if "UNMAPPED" in m.get("reason", ""))

    result = {
        "_meta": {
            "total_headers": total_headers,
            "mapped_headers": mapped_headers,
            "no_rust_equivalent": no_equiv,
            "unmapped_needs_review": unmapped,
            "total_rust_files": len(rust_files),
            "accounted_rust_files": len(accounted_rust_files),
            "unaccounted_rust_files": unaccounted,
        },
        "rust_only": RUST_ONLY,
        "mappings": mapping,
    }

    return result


def main():
    result = build_mapping()
    with open(OUTPUT, "w") as f:
        json.dump(result, f, indent=2)

    meta = result["_meta"]
    print(f"Generated {OUTPUT}")
    print(f"  Headers: {meta['total_headers']}")
    print(f"  Mapped: {meta['mapped_headers']}")
    print(f"  No Rust equivalent: {meta['no_rust_equivalent']}")
    print(f"  Unmapped (needs review): {meta['unmapped_needs_review']}")
    print(f"  Rust files total: {meta['total_rust_files']}")
    print(f"  Rust files accounted: {meta['accounted_rust_files']}")
    if meta["unaccounted_rust_files"]:
        print(f"  Unaccounted Rust files:")
        for f in meta["unaccounted_rust_files"]:
            print(f"    - {f}")


if __name__ == "__main__":
    main()
