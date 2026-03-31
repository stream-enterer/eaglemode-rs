// Port of C++ emStocksFilePanel.h / emStocksFilePanel.cpp

use emcore::emColor::emColor;
use emcore::emInput::{InputKey, InputVariant};
use emcore::emInput::emInputEvent;
use emcore::emInputState::emInputState;
use emcore::emPainter::emPainter;
use emcore::emPanel::{PanelBehavior, PanelState};

use super::emStocksConfig::{emStocksConfig, Sorting};
use super::emStocksListBox::emStocksListBox;
use super::emStocksRec::{emStocksRec, Interest};

/// Port of C++ emStocksFilePanel.
pub struct emStocksFilePanel {
    pub(crate) bg_color: emColor,
    pub(crate) config: emStocksConfig,
    pub(crate) list_box: Option<emStocksListBox>,
    pub(crate) rec: emStocksRec,
    /// Whether the virtual file state is good (data loaded).
    /// DIVERGED: C++ uses IsVFSGood() from emFilePanel base class; Rust uses
    /// a simple bool since the file state machine is not yet integrated.
    pub(crate) vfs_good: bool,
}

impl PanelBehavior for emStocksFilePanel {
    fn Paint(&mut self, painter: &mut emPainter, _w: f64, _h: f64, _state: &PanelState) {
        if self.vfs_good {
            painter.Clear(self.bg_color);
        }
        // C++: if (!IsVFSGood()) emFilePanel::Paint(painter,canvasColor);
        // Base class paint for non-good state deferred until emFilePanel integration.
    }

    fn IsOpaque(&self) -> bool {
        // C++: if (!IsVFSGood()) return emFilePanel::IsOpaque(); else return false;
        false
    }

    fn Input(
        &mut self,
        event: &emInputEvent,
        _state: &PanelState,
        input_state: &emInputState,
    ) -> bool {
        if !self.vfs_good || self.list_box.is_none() {
            return false;
        }
        if event.IsEmpty() || event.variant != InputVariant::Press {
            return false;
        }

        // ── Shift+Alt shortcuts: interest filter and sorting ──
        if input_state.IsShiftAltMod() {
            match event.key {
                // Interest filter
                InputKey::Key('H') => {
                    self.config.min_visible_interest = Interest::High;
                    return true;
                }
                InputKey::Key('M') => {
                    self.config.min_visible_interest = Interest::Medium;
                    return true;
                }
                InputKey::Key('L') => {
                    self.config.min_visible_interest = Interest::Low;
                    return true;
                }
                // Sorting
                InputKey::Key('N') => {
                    self.config.sorting = Sorting::ByName;
                    return true;
                }
                InputKey::Key('T') => {
                    self.config.sorting = Sorting::ByTradeDate;
                    return true;
                }
                InputKey::Key('I') => {
                    self.config.sorting = Sorting::ByInquiryDate;
                    return true;
                }
                InputKey::Key('A') => {
                    self.config.sorting = Sorting::ByAchievement;
                    return true;
                }
                InputKey::Key('1') => {
                    self.config.sorting = Sorting::ByOneWeekRise;
                    return true;
                }
                InputKey::Key('3') => {
                    self.config.sorting = Sorting::ByThreeWeekRise;
                    return true;
                }
                InputKey::Key('9') => {
                    self.config.sorting = Sorting::ByNineWeekRise;
                    return true;
                }
                InputKey::Key('D') => {
                    self.config.sorting = Sorting::ByDividend;
                    return true;
                }
                InputKey::Key('P') => {
                    self.config.sorting = Sorting::ByPurchaseValue;
                    return true;
                }
                InputKey::Key('V') => {
                    self.config.sorting = Sorting::ByValue;
                    return true;
                }
                InputKey::Key('F') => {
                    self.config.sorting = Sorting::ByDifference;
                    return true;
                }
                // OwnedSharesFirst toggle
                InputKey::Key('O') => {
                    self.config.owned_shares_first = !self.config.owned_shares_first;
                    return true;
                }
                _ => {}
            }
        }

        // ── Ctrl shortcuts: ListBox operations ──
        if input_state.IsCtrlMod() {
            let list_box = self.list_box.as_mut().unwrap();
            match event.key {
                InputKey::Key('J') => {
                    list_box.GoBackInHistory(&self.rec);
                    return true;
                }
                InputKey::Key('K') => {
                    list_box.GoForwardInHistory(&self.rec);
                    return true;
                }
                InputKey::Key('N') => {
                    // C++: ListBox->NewStock()
                    // TODO: delegate to ListBox once NewStock is implemented
                    return true;
                }
                InputKey::Key('X') => {
                    // C++: ListBox->CutStocks()
                    // TODO: delegate to ListBox once CutStocks is implemented
                    return true;
                }
                InputKey::Key('C') => {
                    // C++: ListBox->CopyStocks()
                    // TODO: delegate to ListBox once CopyStocks is implemented
                    return true;
                }
                InputKey::Key('V') => {
                    // C++: ListBox->PasteStocks()
                    // TODO: delegate to ListBox once PasteStocks is implemented
                    return true;
                }
                InputKey::Key('P') => {
                    // C++: ListBox->StartToFetchSharePrices()
                    // TODO: delegate to ListBox once StartToFetchSharePrices is implemented
                    return true;
                }
                InputKey::Key('W') => {
                    // C++: ListBox->ShowFirstWebPages()
                    // TODO: delegate to ListBox once ShowFirstWebPages is implemented
                    return true;
                }
                InputKey::Key('H') => {
                    // C++: ListBox->FindSelected()
                    // TODO: delegate to ListBox once FindSelected is implemented
                    return true;
                }
                InputKey::Key('G') => {
                    // C++: ListBox->FindNext()
                    // TODO: delegate to ListBox once FindNext is implemented
                    return true;
                }
                _ => {}
            }
        }

        // ── Shift+Ctrl shortcuts ──
        if input_state.IsShiftCtrlMod() {
            match event.key {
                InputKey::Key('W') => {
                    // C++: ListBox->ShowAllWebPages()
                    // TODO: delegate to ListBox once ShowAllWebPages is implemented
                    return true;
                }
                InputKey::Key('G') => {
                    // C++: ListBox->FindPrevious()
                    // TODO: delegate to ListBox once FindPrevious is implemented
                    return true;
                }
                _ => {}
            }
        }

        // ── No-modifier shortcuts ──
        if input_state.IsNoMod() && event.key == InputKey::Delete {
            // C++: ListBox->DeleteStocks()
            // TODO: delegate to ListBox once DeleteStocks is implemented
            return true;
        }

        // ── Alt shortcuts: set interest on selected stocks ──
        if input_state.IsAltMod() {
            match event.key {
                InputKey::Key('H') => {
                    // C++: ListBox->SetInterest(HIGH_INTEREST)
                    // TODO: delegate to ListBox once SetInterest is implemented
                    return true;
                }
                InputKey::Key('M') => {
                    // C++: ListBox->SetInterest(MEDIUM_INTEREST)
                    // TODO: delegate to ListBox once SetInterest is implemented
                    return true;
                }
                InputKey::Key('L') => {
                    // C++: ListBox->SetInterest(LOW_INTEREST)
                    // TODO: delegate to ListBox once SetInterest is implemented
                    return true;
                }
                _ => {}
            }
        }

        false
    }

    fn GetIconFileName(&self) -> Option<String> {
        Some("documents.tga".to_string())
    }
}

impl emStocksFilePanel {
    pub(crate) fn new() -> Self {
        Self {
            bg_color: emColor::from_packed(0x131520FF),
            config: emStocksConfig::default(),
            list_box: None,
            rec: emStocksRec::default(),
            vfs_good: false,
        }
    }
}

impl Default for emStocksFilePanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_panel_new() {
        let panel = emStocksFilePanel::new();
        assert_eq!(panel.bg_color, emColor::from_packed(0x131520FF));
    }

    #[test]
    fn file_panel_icon() {
        let panel = emStocksFilePanel::new();
        assert_eq!(panel.GetIconFileName(), Some("documents.tga".to_string()));
    }

    #[test]
    fn is_opaque_returns_false() {
        let panel = emStocksFilePanel::new();
        assert!(!panel.IsOpaque());
    }

    #[test]
    fn input_returns_false_when_vfs_not_good() {
        let mut panel = emStocksFilePanel::new();
        panel.vfs_good = false;
        panel.list_box = Some(emStocksListBox::new());
        let mut input_state = emInputState::new();
        input_state.press(InputKey::Shift);
        input_state.press(InputKey::Alt);
        let event = emInputEvent::press(InputKey::Key('H'));
        let state = PanelState::default_for_test();
        assert!(!panel.Input(&event, &state, &input_state));
    }

    #[test]
    fn input_returns_false_when_no_listbox() {
        let mut panel = emStocksFilePanel::new();
        panel.vfs_good = true;
        panel.list_box = None;
        let mut input_state = emInputState::new();
        input_state.press(InputKey::Shift);
        input_state.press(InputKey::Alt);
        let event = emInputEvent::press(InputKey::Key('H'));
        let state = PanelState::default_for_test();
        assert!(!panel.Input(&event, &state, &input_state));
    }

    fn make_active_panel() -> emStocksFilePanel {
        let mut panel = emStocksFilePanel::new();
        panel.vfs_good = true;
        panel.list_box = Some(emStocksListBox::new());
        panel
    }

    #[test]
    fn shift_alt_h_sets_high_interest_filter() {
        let mut panel = make_active_panel();
        let mut input_state = emInputState::new();
        input_state.press(InputKey::Shift);
        input_state.press(InputKey::Alt);
        let event = emInputEvent::press(InputKey::Key('H'));
        let state = PanelState::default_for_test();
        assert!(panel.Input(&event, &state, &input_state));
        assert_eq!(panel.config.min_visible_interest, Interest::High);
    }

    #[test]
    fn shift_alt_m_sets_medium_interest_filter() {
        let mut panel = make_active_panel();
        let mut input_state = emInputState::new();
        input_state.press(InputKey::Shift);
        input_state.press(InputKey::Alt);
        let event = emInputEvent::press(InputKey::Key('M'));
        let state = PanelState::default_for_test();
        assert!(panel.Input(&event, &state, &input_state));
        assert_eq!(panel.config.min_visible_interest, Interest::Medium);
    }

    #[test]
    fn shift_alt_l_sets_low_interest_filter() {
        let mut panel = make_active_panel();
        let mut input_state = emInputState::new();
        input_state.press(InputKey::Shift);
        input_state.press(InputKey::Alt);
        let event = emInputEvent::press(InputKey::Key('L'));
        let state = PanelState::default_for_test();
        assert!(panel.Input(&event, &state, &input_state));
        assert_eq!(panel.config.min_visible_interest, Interest::Low);
    }

    #[test]
    fn shift_alt_n_sets_sort_by_name() {
        let mut panel = make_active_panel();
        panel.config.sorting = Sorting::ByValue; // set non-default
        let mut input_state = emInputState::new();
        input_state.press(InputKey::Shift);
        input_state.press(InputKey::Alt);
        let event = emInputEvent::press(InputKey::Key('N'));
        let state = PanelState::default_for_test();
        assert!(panel.Input(&event, &state, &input_state));
        assert_eq!(panel.config.sorting, Sorting::ByName);
    }

    #[test]
    fn shift_alt_sorting_keys() {
        let cases: Vec<(char, Sorting)> = vec![
            ('T', Sorting::ByTradeDate),
            ('I', Sorting::ByInquiryDate),
            ('A', Sorting::ByAchievement),
            ('1', Sorting::ByOneWeekRise),
            ('3', Sorting::ByThreeWeekRise),
            ('9', Sorting::ByNineWeekRise),
            ('D', Sorting::ByDividend),
            ('P', Sorting::ByPurchaseValue),
            ('V', Sorting::ByValue),
            ('F', Sorting::ByDifference),
        ];
        for (key, expected_sorting) in cases {
            let mut panel = make_active_panel();
            let mut input_state = emInputState::new();
            input_state.press(InputKey::Shift);
            input_state.press(InputKey::Alt);
            let event = emInputEvent::press(InputKey::Key(key));
            let state = PanelState::default_for_test();
            assert!(
                panel.Input(&event, &state, &input_state),
                "Shift+Alt+{key} should consume"
            );
            assert_eq!(
                panel.config.sorting, expected_sorting,
                "Shift+Alt+{key} should set {expected_sorting:?}"
            );
        }
    }

    #[test]
    fn shift_alt_o_toggles_owned_shares_first() {
        let mut panel = make_active_panel();
        assert!(!panel.config.owned_shares_first);
        let mut input_state = emInputState::new();
        input_state.press(InputKey::Shift);
        input_state.press(InputKey::Alt);
        let event = emInputEvent::press(InputKey::Key('O'));
        let state = PanelState::default_for_test();
        assert!(panel.Input(&event, &state, &input_state));
        assert!(panel.config.owned_shares_first);
        // Toggle back
        assert!(panel.Input(&event, &state, &input_state));
        assert!(!panel.config.owned_shares_first);
    }

    #[test]
    fn ctrl_j_goes_back_in_history() {
        let mut panel = make_active_panel();
        // Set up rec with dates so GoBackInHistory works
        let mut stock = super::super::emStocksRec::StockRec::default();
        stock.AddPrice("2024-06-14", "100");
        stock.AddPrice("2024-06-15", "101");
        panel.rec.stocks.push(stock);
        panel.list_box.as_mut().unwrap().SetSelectedDate("2024-06-15");

        let mut input_state = emInputState::new();
        input_state.press(InputKey::Ctrl);
        let event = emInputEvent::press(InputKey::Key('J'));
        let state = PanelState::default_for_test();
        assert!(panel.Input(&event, &state, &input_state));
        assert_eq!(
            panel.list_box.as_ref().unwrap().GetSelectedDate(),
            "2024-06-14"
        );
    }

    #[test]
    fn ctrl_k_goes_forward_in_history() {
        let mut panel = make_active_panel();
        let mut stock = super::super::emStocksRec::StockRec::default();
        stock.AddPrice("2024-06-14", "100");
        stock.AddPrice("2024-06-15", "101");
        panel.rec.stocks.push(stock);
        panel.list_box.as_mut().unwrap().SetSelectedDate("2024-06-14");

        let mut input_state = emInputState::new();
        input_state.press(InputKey::Ctrl);
        let event = emInputEvent::press(InputKey::Key('K'));
        let state = PanelState::default_for_test();
        assert!(panel.Input(&event, &state, &input_state));
        assert_eq!(
            panel.list_box.as_ref().unwrap().GetSelectedDate(),
            "2024-06-15"
        );
    }

    #[test]
    fn ctrl_shortcuts_consume_events() {
        let ctrl_keys = ['N', 'X', 'C', 'V', 'P', 'W', 'H', 'G'];
        for key in ctrl_keys {
            let mut panel = make_active_panel();
            let mut input_state = emInputState::new();
            input_state.press(InputKey::Ctrl);
            let event = emInputEvent::press(InputKey::Key(key));
            let state = PanelState::default_for_test();
            assert!(
                panel.Input(&event, &state, &input_state),
                "Ctrl+{key} should consume"
            );
        }
    }

    #[test]
    fn shift_ctrl_shortcuts_consume_events() {
        let keys = ['W', 'G'];
        for key in keys {
            let mut panel = make_active_panel();
            let mut input_state = emInputState::new();
            input_state.press(InputKey::Shift);
            input_state.press(InputKey::Ctrl);
            let event = emInputEvent::press(InputKey::Key(key));
            let state = PanelState::default_for_test();
            assert!(
                panel.Input(&event, &state, &input_state),
                "Shift+Ctrl+{key} should consume"
            );
        }
    }

    #[test]
    fn delete_consumes_event() {
        let mut panel = make_active_panel();
        let input_state = emInputState::new();
        let event = emInputEvent::press(InputKey::Delete);
        let state = PanelState::default_for_test();
        assert!(panel.Input(&event, &state, &input_state));
    }

    #[test]
    fn alt_interest_shortcuts_consume_events() {
        let keys = ['H', 'M', 'L'];
        for key in keys {
            let mut panel = make_active_panel();
            let mut input_state = emInputState::new();
            input_state.press(InputKey::Alt);
            let event = emInputEvent::press(InputKey::Key(key));
            let state = PanelState::default_for_test();
            assert!(
                panel.Input(&event, &state, &input_state),
                "Alt+{key} should consume"
            );
        }
    }

    #[test]
    fn unrecognized_key_not_consumed() {
        let mut panel = make_active_panel();
        let input_state = emInputState::new();
        let event = emInputEvent::press(InputKey::Key('Z'));
        let state = PanelState::default_for_test();
        assert!(!panel.Input(&event, &state, &input_state));
    }

    #[test]
    fn release_events_not_consumed() {
        let mut panel = make_active_panel();
        let mut input_state = emInputState::new();
        input_state.press(InputKey::Shift);
        input_state.press(InputKey::Alt);
        let event = emInputEvent::release(InputKey::Key('H'));
        let state = PanelState::default_for_test();
        assert!(!panel.Input(&event, &state, &input_state));
    }
}
