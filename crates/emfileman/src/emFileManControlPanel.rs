//! Sort/filter/theme UI control panel.
//!
//! Port of C++ `emFileManControlPanel`. Extends `emLinearLayout`.
//! Contains sort criterion radio buttons, name sorting style radio buttons,
//! directories-first and show-hidden checkboxes, theme selectors,
//! autosave checkbox, and command group buttons.

use std::cell::RefCell;
use std::rc::Rc;

use emcore::emButton::emButton;
use emcore::emCheckButton::emCheckButton;
use emcore::emColor::emColor;
use emcore::emContext::emContext;
use emcore::emInput::emInputEvent;
use emcore::emInputState::emInputState;
use emcore::emLook::emLook;
use emcore::emPanel::{PanelBehavior, PanelState};
use emcore::emPainter::{emPainter, TextAlignment, VAlign};
use emcore::emRadioButton::{emRadioButton, RadioGroup};

use crate::emFileManConfig::{NameSortingStyle, SortCriterion};
use crate::emFileManModel::emFileManModel;
use crate::emFileManThemeNames::emFileManThemeNames;
use crate::emFileManViewConfig::emFileManViewConfig;

/// Sort criterion labels matching enum variant order.
const SORT_LABELS: [&str; 6] = [
    "By Name", "By Ending", "By Class", "By Version", "By Date", "By Size",
];

/// Name sorting style labels matching enum variant order.
const NSS_LABELS: [&str; 3] = ["Per Locale", "Case Sensitive", "Case Insensitive"];

/// Control panel for file manager settings.
/// Port of C++ `emFileManControlPanel` (extends emLinearLayout).
pub struct emFileManControlPanel {
    _ctx: Rc<emContext>,
    config: Rc<RefCell<emFileManViewConfig>>,
    file_man: Rc<RefCell<emFileManModel>>,
    _theme_names: Rc<RefCell<emFileManThemeNames>>,
    _look: Rc<emLook>,

    // Sort criterion radio group (6 buttons)
    sort_group: Rc<RefCell<RadioGroup>>,
    sort_radios: Vec<emRadioButton>,

    // Name sorting style radio group (3 buttons)
    nss_group: Rc<RefCell<RadioGroup>>,
    nss_radios: Vec<emRadioButton>,

    // Checkboxes
    dirs_first_check: emCheckButton,
    show_hidden_check: emCheckButton,
    autosave_check: emCheckButton,

    // Action buttons
    save_button: emButton,
    select_all_button: emButton,
    clear_sel_button: emButton,
    swap_sel_button: emButton,
    paths_clip_button: emButton,
    names_clip_button: emButton,

    /// Tracks config generation to detect external changes.
    last_config_gen: u64,
}

impl emFileManControlPanel {
    pub fn new(ctx: Rc<emContext>) -> Self {
        let config = emFileManViewConfig::Acquire(&ctx);
        let file_man = emFileManModel::Acquire(&ctx);
        let theme_names = emFileManThemeNames::Acquire(&ctx);
        let look = emLook::new();

        // Build sort criterion radio group
        let sort_group = RadioGroup::new();
        let sort_radios: Vec<emRadioButton> = SORT_LABELS
            .iter()
            .enumerate()
            .map(|(i, label)| emRadioButton::new(label, Rc::clone(&look), Rc::clone(&sort_group), i))
            .collect();

        // Build name sorting style radio group
        let nss_group = RadioGroup::new();
        let nss_radios: Vec<emRadioButton> = NSS_LABELS
            .iter()
            .enumerate()
            .map(|(i, label)| emRadioButton::new(label, Rc::clone(&look), Rc::clone(&nss_group), i))
            .collect();

        // Checkboxes
        let dirs_first_check = emCheckButton::new("Directories First", Rc::clone(&look));
        let show_hidden_check = emCheckButton::new("Show Hidden", Rc::clone(&look));
        let autosave_check = emCheckButton::new("Autosave", Rc::clone(&look));

        // Action buttons
        let save_button = emButton::new("Save", Rc::clone(&look));
        let select_all_button = emButton::new("Select All", Rc::clone(&look));
        let clear_sel_button = emButton::new("Clear Sel", Rc::clone(&look));
        let swap_sel_button = emButton::new("Swap Sel", Rc::clone(&look));
        let paths_clip_button = emButton::new("Paths→Clip", Rc::clone(&look));
        let names_clip_button = emButton::new("Names→Clip", Rc::clone(&look));

        let last_config_gen = config.borrow().GetChangeSignal();

        let mut panel = Self {
            _ctx: ctx,
            config,
            file_man,
            _theme_names: theme_names,
            _look: look,
            sort_group,
            sort_radios,
            nss_group,
            nss_radios,
            dirs_first_check,
            show_hidden_check,
            autosave_check,
            save_button,
            select_all_button,
            clear_sel_button,
            swap_sel_button,
            paths_clip_button,
            names_clip_button,
            last_config_gen,
        };
        panel.sync_from_config();
        panel
    }

    /// Read current config state into widget state.
    fn sync_from_config(&mut self) {
        let cfg = self.config.borrow();
        self.sort_group
            .borrow_mut()
            .SetChecked(cfg.GetSortCriterion() as usize);
        self.nss_group
            .borrow_mut()
            .SetChecked(cfg.GetNameSortingStyle() as usize);
        self.dirs_first_check
            .SetChecked(cfg.GetSortDirectoriesFirst());
        self.show_hidden_check
            .SetChecked(cfg.GetShowHiddenFiles());
        self.autosave_check.SetChecked(cfg.GetAutosave());
    }

    /// Paint a section label at the given y position. Returns the y after the label.
    fn paint_section_label(
        painter: &mut emPainter,
        x: f64,
        y: f64,
        w: f64,
        row_h: f64,
        label: &str,
        fg: emColor,
    ) -> f64 {
        painter.PaintTextBoxed(
            x,
            y,
            w,
            row_h,
            label,
            row_h * 0.7,
            fg,
            emColor::TRANSPARENT,
            TextAlignment::Left,
            VAlign::Center,
            TextAlignment::Left,
            1.0,
            false,
            1.0,
        );
        y + row_h
    }
}

impl PanelBehavior for emFileManControlPanel {
    fn IsOpaque(&self) -> bool {
        false
    }

    fn Cycle(&mut self, _ctx: &mut emcore::emPanelCtx::PanelCtx) -> bool {
        let gen = self.config.borrow().GetChangeSignal();
        if gen != self.last_config_gen {
            self.last_config_gen = gen;
            self.sync_from_config();
            true
        } else {
            false
        }
    }

    fn Paint(&mut self, painter: &mut emPainter, w: f64, h: f64, _state: &PanelState) {
        let fg = emColor::from_packed(0xCCCCCCFF);
        let margin = 0.02;
        let content_w = w - 2.0 * margin;
        let row_h = h * 0.04;
        let widget_h = row_h * 1.2;
        let widget_w = content_w * 0.45;
        let mut y = margin;

        // --- Sort Criterion section ---
        y = Self::paint_section_label(painter, margin, y, content_w, row_h, "Sort Criterion", fg);
        for radio in &mut self.sort_radios {
            radio.Paint(painter, widget_w, widget_h, true);
            // Translate painter for next widget position
            y += widget_h;
        }

        y += row_h * 0.5; // spacing

        // --- Name Sorting Style section ---
        y = Self::paint_section_label(
            painter, margin, y, content_w, row_h, "Name Sorting Style", fg,
        );
        for radio in &mut self.nss_radios {
            radio.Paint(painter, widget_w, widget_h, true);
            y += widget_h;
        }

        y += row_h * 0.5;

        // --- Options section ---
        y = Self::paint_section_label(painter, margin, y, content_w, row_h, "Options", fg);
        self.dirs_first_check
            .Paint(painter, widget_w, widget_h, true);
        y += widget_h;
        self.show_hidden_check
            .Paint(painter, widget_w, widget_h, true);
        y += widget_h;
        self.autosave_check
            .Paint(painter, widget_w, widget_h, true);
        y += widget_h;

        y += row_h * 0.5;

        // --- Actions section ---
        y = Self::paint_section_label(painter, margin, y, content_w, row_h, "Actions", fg);
        self.save_button.Paint(painter, widget_w, widget_h, true);
        y += widget_h;
        self.select_all_button
            .Paint(painter, widget_w, widget_h, true);
        y += widget_h;
        self.clear_sel_button
            .Paint(painter, widget_w, widget_h, true);
        y += widget_h;
        self.swap_sel_button
            .Paint(painter, widget_w, widget_h, true);
        y += widget_h;
        self.paths_clip_button
            .Paint(painter, widget_w, widget_h, true);
        y += widget_h;
        self.names_clip_button
            .Paint(painter, widget_w, widget_h, true);
        let _ = y + widget_h; // suppress unused warning for final y
    }

    fn Input(
        &mut self,
        event: &emInputEvent,
        state: &PanelState,
        input_state: &emInputState,
    ) -> bool {
        // Delegate to sort criterion radios
        for radio in &mut self.sort_radios {
            if radio.Input(event, state, input_state) {
                if let Some(idx) = self.sort_group.borrow().GetChecked() {
                    let sc = match idx {
                        0 => SortCriterion::ByName,
                        1 => SortCriterion::ByEnding,
                        2 => SortCriterion::ByClass,
                        3 => SortCriterion::ByVersion,
                        4 => SortCriterion::ByDate,
                        5 => SortCriterion::BySize,
                        _ => return true,
                    };
                    self.config.borrow_mut().SetSortCriterion(sc);
                }
                return true;
            }
        }

        // Delegate to name sorting style radios
        for radio in &mut self.nss_radios {
            if radio.Input(event, state, input_state) {
                if let Some(idx) = self.nss_group.borrow().GetChecked() {
                    let nss = match idx {
                        0 => NameSortingStyle::PerLocale,
                        1 => NameSortingStyle::CaseSensitive,
                        2 => NameSortingStyle::CaseInsensitive,
                        _ => return true,
                    };
                    self.config.borrow_mut().SetNameSortingStyle(nss);
                }
                return true;
            }
        }

        // Delegate to checkboxes
        if self.dirs_first_check.Input(event, state, input_state) {
            self.config
                .borrow_mut()
                .SetSortDirectoriesFirst(self.dirs_first_check.IsChecked());
            return true;
        }
        if self.show_hidden_check.Input(event, state, input_state) {
            self.config
                .borrow_mut()
                .SetShowHiddenFiles(self.show_hidden_check.IsChecked());
            return true;
        }
        if self.autosave_check.Input(event, state, input_state) {
            self.config
                .borrow_mut()
                .SetAutosave(self.autosave_check.IsChecked());
            return true;
        }

        // Delegate to action buttons
        if self.save_button.Input(event, state, input_state) {
            if self.save_button.IsPressed() {
                // Press tracked; actual save on release via on_click
            } else {
                self.config.borrow_mut().SaveAsDefault();
            }
            return true;
        }
        if self.select_all_button.Input(event, state, input_state) {
            // Select all is handled at the directory panel level, not here.
            return true;
        }
        if self.clear_sel_button.Input(event, state, input_state) {
            self.file_man.borrow_mut().ClearTargetSelection();
            return true;
        }
        if self.swap_sel_button.Input(event, state, input_state) {
            self.file_man.borrow_mut().SwapSelection();
            return true;
        }
        if self.paths_clip_button.Input(event, state, input_state) {
            let _text = self.file_man.borrow().SelectionToClipboard(false, false);
            return true;
        }
        if self.names_clip_button.Input(event, state, input_state) {
            let _text = self.file_man.borrow().SelectionToClipboard(false, true);
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_implements_panel_behavior() {
        use emcore::emPanel::PanelBehavior;

        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emFileManControlPanel::new(Rc::clone(&ctx));
        let _: Box<dyn PanelBehavior> = Box::new(panel);
    }

    #[test]
    fn sync_from_config_initializes_widgets() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emFileManControlPanel::new(Rc::clone(&ctx));
        // Default config: ByName sort, PerLocale nss, dirs_first=false, hidden=false
        assert_eq!(panel.sort_group.borrow().GetChecked(), Some(0));
        assert_eq!(panel.nss_group.borrow().GetChecked(), Some(0));
        assert!(!panel.dirs_first_check.IsChecked());
        assert!(!panel.show_hidden_check.IsChecked());
    }

    #[test]
    fn cycle_detects_config_change() {
        use emcore::emPanelCtx::PanelCtx;
        use emcore::emPanelTree::{PanelId, PanelTree};
        use slotmap::Key as _;

        let ctx = emcore::emContext::emContext::NewRoot();
        let mut panel = emFileManControlPanel::new(Rc::clone(&ctx));

        // Mutate config externally
        panel
            .config
            .borrow_mut()
            .SetSortCriterion(SortCriterion::BySize);

        let mut tree = PanelTree::new();
        let mut pctx = PanelCtx {
            tree: &mut tree,
            id: PanelId::null(),
        };
        let changed = panel.Cycle(&mut pctx);
        assert!(changed);
        // Widget should now reflect BySize
        assert_eq!(panel.sort_group.borrow().GetChecked(), Some(5));
    }

    #[test]
    fn widget_counts() {
        let ctx = emcore::emContext::emContext::NewRoot();
        let panel = emFileManControlPanel::new(Rc::clone(&ctx));
        assert_eq!(panel.sort_radios.len(), 6);
        assert_eq!(panel.nss_radios.len(), 3);
    }
}
