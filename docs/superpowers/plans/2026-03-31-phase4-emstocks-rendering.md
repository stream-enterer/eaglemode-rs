# Phase 4: emStocks Rendering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete all 6 emStocks panel types with full Paint/LayoutChildren/Input, plus process integration for the price fetcher.

**Architecture:** The data model layer is already ported. This phase adds PanelBehavior implementations (Paint, LayoutChildren, Input, Cycle) and wires the existing data logic to the panel framework. Build order: simpler panels first (FilePanel, FetchPricesDialog), then complex ones (ListBox, ControlPanel, ItemPanel), then the chart.

**Tech Stack:** Rust, emstocks crate, emcore crate (emPainter, emBorder, emLinearGroup, emListBox, emDialog)

**Depends on:** Phase 1 (emColor scale), Phase 3 (emFilePanel integration)

**C++ Reference:** `~/git/eaglemode-0.96.4/src/emStocks/` and `~/git/eaglemode-0.96.4/include/emStocks/`

---

## Task 1: emStocksFilePanel — Top-Level Container

**Files:**
- Modify: `crates/emstocks/src/emStocksFilePanel.rs` (49 lines → ~150 lines)

This is the simplest panel — good starting point.

- [ ] **Step 1: Write test for paint and layout**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_panel_background_color() {
        let panel = emStocksFilePanel::default();
        assert_eq!(panel.background_color, emColor::rgba(0x13, 0x15, 0x20, 0xFF));
    }

    #[test]
    fn test_file_panel_is_not_opaque() {
        let panel = emStocksFilePanel::default();
        assert!(!panel.is_opaque());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p emstocks -- test_file_panel_background`
Expected: FAIL or PASS depending on existing defaults

- [ ] **Step 3: Implement full PanelBehavior**

In `crates/emstocks/src/emStocksFilePanel.rs`:

```rust
use crate::emStocksListBox::emStocksListBox;
use crate::emStocksControlPanel::emStocksControlPanel;
use emcore::emColor::emColor;
use emcore::emPainter::emPainter;
use emcore::emInput::{emInputEvent, InputKey, InputVariant};
use emcore::emInputState::emInputState;
use emcore::emPanel::{PanelState, Rect};

pub struct emStocksFilePanel {
    pub(crate) background_color: emColor,
    list_box: Option<emStocksListBox>,
    control_panel: Option<emStocksControlPanel>,
}

impl Default for emStocksFilePanel {
    fn default() -> Self {
        Self {
            background_color: emColor::rgba(0x13, 0x15, 0x20, 0xFF),
            list_box: None,
            control_panel: None,
        }
    }
}

impl emStocksFilePanel {
    pub fn is_opaque(&self) -> bool {
        false
    }

    pub fn paint(&self, painter: &mut emPainter, rect: &Rect) {
        // Fill with dark background
        painter.PaintRect(
            rect.x, rect.y, rect.w, rect.h,
            self.background_color,
            0, // canvasColor
        );
    }

    pub fn layout_children(&mut self, content_rect: &Rect) {
        // ListBox fills entire content area
        if let Some(ref mut lb) = self.list_box {
            lb.set_layout(content_rect.x, content_rect.y, content_rect.w, content_rect.h);
        }
    }

    pub fn Input(&mut self, event: &emInputEvent, _state: &PanelState, _input_state: &emInputState) -> bool {
        if event.variant != InputVariant::Press {
            return false;
        }
        let ctrl = _input_state.is_ctrl_pressed();
        let shift = _input_state.is_shift_pressed();
        let alt = _input_state.is_alt_pressed();

        match event.key {
            InputKey::Key('n') if ctrl => {
                self.new_stock();
                true
            }
            InputKey::Key('x') if ctrl => {
                self.cut_stocks();
                true
            }
            InputKey::Key('c') if ctrl => {
                self.copy_stocks();
                true
            }
            InputKey::Key('v') if ctrl => {
                self.paste_stocks();
                true
            }
            InputKey::Key('f') if ctrl => {
                self.find();
                true
            }
            InputKey::F3 if !shift => {
                self.find_next();
                true
            }
            InputKey::F3 if shift => {
                self.find_previous();
                true
            }
            InputKey::Key('h') if shift && alt => {
                self.set_interest_filter_high();
                true
            }
            InputKey::Key('m') if shift && alt => {
                self.set_interest_filter_medium();
                true
            }
            InputKey::Key('l') if shift && alt => {
                self.set_interest_filter_low();
                true
            }
            _ => false,
        }
    }

    // Delegate operations to list_box
    fn new_stock(&mut self) {
        if let Some(ref mut lb) = self.list_box { lb.new_stock(); }
    }
    fn cut_stocks(&mut self) {
        if let Some(ref mut lb) = self.list_box { lb.cut(); }
    }
    fn copy_stocks(&mut self) {
        if let Some(ref mut lb) = self.list_box { lb.copy(); }
    }
    fn paste_stocks(&mut self) {
        if let Some(ref mut lb) = self.list_box { lb.paste(); }
    }
    fn find(&mut self) { /* focus search field in control panel */ }
    fn find_next(&mut self) {
        if let Some(ref mut lb) = self.list_box { lb.find_next(); }
    }
    fn find_previous(&mut self) {
        if let Some(ref mut lb) = self.list_box { lb.find_previous(); }
    }
    fn set_interest_filter_high(&mut self) { /* update config */ }
    fn set_interest_filter_medium(&mut self) { /* update config */ }
    fn set_interest_filter_low(&mut self) { /* update config */ }
}
```

Remove DIVERGED comments on lines 6 and 12.

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emstocks/src/emStocksFilePanel.rs && git commit -m "feat(emStocksFilePanel): implement full PanelBehavior with paint, layout, input"
```

---

## Task 2: emStocksFetchPricesDialog — Progress Dialog

**Files:**
- Modify: `crates/emstocks/src/emStocksFetchPricesDialog.rs` (96 lines → ~200 lines)

- [ ] **Step 1: Write test for progress rendering**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_panel_paint_width() {
        let mut bar = ProgressBarPanel::default();
        bar.set_progress(50.0);
        // At 50%, the fill rect should be half the available width
        let rect = Rect { x: 0.0, y: 0.0, w: 100.0, h: 20.0 };
        let margin = rect.w * 0.1;
        let fill_w = (rect.w - 2.0 * margin) * 0.5;
        assert!((fill_w - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_dialog_auto_sizes() {
        let dialog = emStocksFetchPricesDialog::new(800.0, 600.0);
        // width = parent_width * 0.4 = 320
        // height = width * 0.15 = 48
        assert!((dialog.width - 320.0).abs() < 1.0);
        assert!((dialog.height - 48.0).abs() < 1.0);
    }
}
```

- [ ] **Step 2: Implement ProgressBarPanel rendering**

```rust
impl ProgressBarPanel {
    pub fn paint(&self, painter: &mut emPainter, rect: &Rect) {
        let margin_x = rect.w * 0.1;
        let margin_y = rect.h * 0.1;
        let inner_x = rect.x + margin_x;
        let inner_y = rect.y + margin_y;
        let inner_w = rect.w - 2.0 * margin_x;
        let inner_h = rect.h - 2.0 * margin_y;

        // Background (dark)
        painter.PaintRect(inner_x, inner_y, inner_w, inner_h,
            emColor::rgba(40, 40, 40, 255), 0);

        // Fill (green, proportional to progress)
        let fill_w = inner_w * (self.progress_in_percent as f64 / 100.0);
        if fill_w > 0.0 {
            painter.PaintRect(inner_x, inner_y, fill_w, inner_h,
                emColor::rgba(0, 180, 0, 255), 0);
        }
    }
}
```

- [ ] **Step 3: Implement dialog lifecycle**

```rust
impl emStocksFetchPricesDialog {
    pub fn new(parent_width: f64, parent_height: f64) -> Self {
        Self {
            width: parent_width * 0.4,
            height: parent_width * 0.4 * 0.15,
            label_text: String::new(),
            progress_bar: ProgressBarPanel::default(),
            fetcher_state: FetcherState::Idle,
            error_message: None,
        }
    }

    pub fn Cycle(&mut self, fetcher: &emStocksPricesFetcher) {
        // Poll fetcher state
        self.progress_bar.set_progress(fetcher.GetProgressInPercent());

        if let Some(stock_name) = fetcher.GetCurrentStockName() {
            self.label_text = format!("Fetching: {}", stock_name);
        }

        if fetcher.is_done() {
            if let Some(err) = fetcher.GetError() {
                self.label_text = format!("Error: {}", err);
                self.error_message = Some(err.to_string());
            } else {
                self.label_text = "Done.".to_string();
                // Auto-close after completion
                self.fetcher_state = FetcherState::Done;
            }
        }
    }

    pub fn should_close(&self) -> bool {
        matches!(self.fetcher_state, FetcherState::Done) && self.error_message.is_none()
    }
}
```

Remove DIVERGED comments on lines 4 and 26.

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emstocks/src/emStocksFetchPricesDialog.rs && git commit -m "feat(emStocksFetchPricesDialog): implement progress dialog rendering and lifecycle"
```

---

## Task 3: emStocksListBox — Full List Integration

**Files:**
- Modify: `crates/emstocks/src/emStocksListBox.rs` (365 lines → ~600 lines)

- [ ] **Step 1: Write tests for user operations**

```rust
#[cfg(test)]
mod operation_tests {
    use super::*;
    use crate::emStocksRec::{emStocksRec, StockRec};

    fn make_test_rec() -> emStocksRec {
        let mut rec = emStocksRec::new();
        let mut s1 = StockRec::new();
        s1.name = "Apple".to_string();
        s1.symbol = "AAPL".to_string();
        rec.stocks.push(s1);
        let mut s2 = StockRec::new();
        s2.name = "Google".to_string();
        s2.symbol = "GOOG".to_string();
        rec.stocks.push(s2);
        rec
    }

    #[test]
    fn test_paint_empty_shows_message() {
        let lb = emStocksListBox::new();
        assert!(lb.get_paint_empty_message().is_some());
    }

    #[test]
    fn test_copy_paste_round_trip() {
        let rec = make_test_rec();
        let mut lb = emStocksListBox::new();
        lb.set_rec(&rec);
        lb.select_stock(0);

        // Copy
        let clipboard_data = lb.copy_selected();
        assert!(!clipboard_data.is_empty());

        // Paste into a new rec
        let mut rec2 = emStocksRec::new();
        let count_before = rec2.stocks.len();
        lb.paste_into(&mut rec2, &clipboard_data);
        assert_eq!(rec2.stocks.len(), count_before + 1);
        assert_eq!(rec2.stocks.last().unwrap().name, "Apple");
    }
}
```

- [ ] **Step 2: Implement Paint**

```rust
impl emStocksListBox {
    pub fn paint(&self, painter: &mut emPainter, rect: &Rect) {
        // Paint parent (inherited list box rendering)
        // ... parent paint logic ...

        // If empty, show message
        if self.visible_items.is_empty() {
            let msg = "Empty stock list";
            let text_h = rect.h * 0.05;
            let text_x = rect.x + rect.w * 0.5;
            let text_y = rect.y + rect.h * 0.5;
            painter.PaintTextLayout(text_x, text_y, msg, text_h,
                emColor::rgba(180, 180, 180, 255), 0, 0.0);
        }
    }

    pub fn get_paint_empty_message(&self) -> Option<&str> {
        if self.visible_items.is_empty() {
            Some("Empty stock list")
        } else {
            None
        }
    }
}
```

- [ ] **Step 3: Implement cut/copy/paste**

```rust
impl emStocksListBox {
    /// Serialize selected stocks for clipboard.
    pub fn copy_selected(&self) -> String {
        let mut out = String::new();
        for &idx in &self.selected_indices {
            if let Some(stock) = self.rec.as_ref().and_then(|r| r.stocks.get(idx)) {
                // Serialize as emStocks record format
                out.push_str(&format!("Stock:{}\n", stock.name));
                out.push_str(&format!("Symbol:{}\n", stock.symbol));
                out.push_str(&format!("WKN:{}\n", stock.wkn));
                out.push_str(&format!("ISIN:{}\n", stock.isin));
                // ... all fields ...
                out.push_str("---\n");
            }
        }
        out
    }

    /// Cut = copy + remove.
    pub fn cut(&mut self) -> String {
        let data = self.copy_selected();
        self.delete_selected();
        data
    }

    /// Paste from clipboard data.
    pub fn paste_into(&self, rec: &mut emStocksRec, clipboard_data: &str) {
        // Parse clipboard format back into StockRec
        // Simple line-based format: "Key:Value\n" separated by "---\n"
        let mut current = StockRec::new();
        for line in clipboard_data.lines() {
            if line == "---" {
                rec.stocks.push(std::mem::take(&mut current));
                current = StockRec::new();
            } else if let Some((key, val)) = line.split_once(':') {
                match key {
                    "Stock" => current.name = val.to_string(),
                    "Symbol" => current.symbol = val.to_string(),
                    "WKN" => current.wkn = val.to_string(),
                    "ISIN" => current.isin = val.to_string(),
                    _ => {}
                }
            }
        }
    }

    /// Delete all selected stocks.
    fn delete_selected(&mut self) {
        // Remove in reverse order to preserve indices
        let mut indices: Vec<usize> = self.selected_indices.clone();
        indices.sort_unstable();
        indices.reverse();
        if let Some(ref mut rec) = self.rec_mut {
            for idx in indices {
                if idx < rec.stocks.len() {
                    rec.stocks.remove(idx);
                }
            }
        }
        self.selected_indices.clear();
    }

    /// Find next stock by name substring.
    pub fn find_next(&mut self) {
        if self.search_text.is_empty() { return; }
        let query = self.search_text.to_lowercase();
        let start = self.selected_indices.first().map_or(0, |&i| i + 1);
        if let Some(rec) = &self.rec {
            for i in start..rec.stocks.len() {
                if rec.stocks[i].name.to_lowercase().contains(&query) {
                    self.selected_indices = vec![i];
                    // scroll to item
                    return;
                }
            }
            // Wrap around
            for i in 0..start.min(rec.stocks.len()) {
                if rec.stocks[i].name.to_lowercase().contains(&query) {
                    self.selected_indices = vec![i];
                    return;
                }
            }
        }
    }

    pub fn find_previous(&mut self) {
        if self.search_text.is_empty() { return; }
        let query = self.search_text.to_lowercase();
        let start = self.selected_indices.first().map_or(0, |&i| if i > 0 { i - 1 } else { 0 });
        if let Some(rec) = &self.rec {
            for i in (0..=start).rev() {
                if rec.stocks[i].name.to_lowercase().contains(&query) {
                    self.selected_indices = vec![i];
                    return;
                }
            }
        }
    }

    pub fn new_stock(&mut self) {
        if let Some(ref mut rec) = self.rec_mut {
            let stock = StockRec::new();
            rec.stocks.push(stock);
            let idx = rec.stocks.len() - 1;
            self.selected_indices = vec![idx];
        }
    }
}
```

Remove DIVERGED comment on line 9.

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emstocks/src/emStocksListBox.rs && git commit -m "feat(emStocksListBox): implement paint, cut/copy/paste, find, new stock operations"
```

---

## Task 4: emStocksControlPanel — Full Widget Tree

**Files:**
- Modify: `crates/emstocks/src/emStocksControlPanel.rs` (108 lines → ~800 lines)
- C++ ref: `~/git/eaglemode-0.96.4/src/emStocks/emStocksControlPanel.cpp` (1,324 lines)

- [ ] **Step 1: Write test for widget creation**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_panel_creates_widgets_on_expand() {
        let mut panel = emStocksControlPanel::new();
        panel.auto_expand();
        assert!(panel.api_script.is_some());
        assert!(panel.sorting_group.is_some());
        assert!(panel.filter_group.is_some());
    }

    #[test]
    fn test_chart_period_text_formatter() {
        assert_eq!(emStocksControlPanel::chart_period_text(30), "30 days");
        assert_eq!(emStocksControlPanel::chart_period_text(365), "1 year");
        assert_eq!(emStocksControlPanel::chart_period_text(730), "2 years");
        assert_eq!(emStocksControlPanel::chart_period_text(1825), "5 years");
    }
}
```

- [ ] **Step 2: Implement widget hierarchy**

Port the C++ widget hierarchy. The key is the `AutoExpand` lifecycle: widgets are created when the panel is viewed at sufficient zoom, destroyed when scrolled away.

```rust
pub struct emStocksControlPanel {
    // Existing data fields...
    update_controls_needed: bool,

    // Widget tree (created on expand)
    about_label: Option<String>,
    api_script: Option<FileFieldPanel>,
    api_interpreter: Option<FileFieldPanel>,
    api_key: Option<String>,
    web_browser: Option<FileFieldPanel>,
    auto_update_dates: Option<bool>,
    triggering_opens_web: Option<bool>,
    chart_period: Option<i32>,
    min_visible_interest: Option<u8>,
    country_panel: Option<ControlCategoryPanel>,
    sector_panel: Option<ControlCategoryPanel>,
    collection_panel: Option<ControlCategoryPanel>,
    sorting_group: Option<SortingGroup>,
    filter_group: Option<FilterGroup>,
    // ... buttons ...
    fetch_button: Option<emButton>,
    delete_prices_button: Option<emButton>,
    go_back_button: Option<emButton>,
    go_forward_button: Option<emButton>,
    selected_date: Option<String>,
    total_purchase: Option<f64>,
    total_current: Option<f64>,
    total_difference: Option<f64>,
    // Edit buttons
    new_stock_button: Option<emButton>,
    cut_button: Option<emButton>,
    copy_button: Option<emButton>,
    paste_button: Option<emButton>,
    delete_button: Option<emButton>,
    select_all_button: Option<emButton>,
    clear_sel_button: Option<emButton>,
    // Interest buttons
    set_high_button: Option<emButton>,
    set_medium_button: Option<emButton>,
    set_low_button: Option<emButton>,
    // Search
    find_selected_button: Option<emButton>,
    search_text: Option<String>,
    find_next_button: Option<emButton>,
    find_prev_button: Option<emButton>,
}

impl emStocksControlPanel {
    pub fn auto_expand(&mut self) {
        // Create widgets when panel becomes viewed
        if self.about_label.is_none() {
            self.about_label = Some("Eagle Mode Stock Portfolio\nManage your stock watchlist.".to_string());
        }
        if self.sorting_group.is_none() {
            self.sorting_group = Some(SortingGroup::new());
        }
        if self.filter_group.is_none() {
            self.filter_group = Some(FilterGroup::new());
        }
        // ... create all widget groups ...
    }

    pub fn auto_shrink(&mut self) {
        // Destroy widgets when panel scrolls away
        self.about_label = None;
        self.sorting_group = None;
        self.filter_group = None;
        // ... etc ...
    }

    /// C++ `ChartPeriodTextOfValue` — format days as human-readable period.
    pub fn chart_period_text(days: i32) -> String {
        if days < 60 {
            format!("{} days", days)
        } else if days < 365 {
            format!("{} months", days / 30)
        } else {
            let years = days / 365;
            if years == 1 { "1 year".to_string() }
            else { format!("{} years", years) }
        }
    }

    /// Synchronize widget values from config and model.
    pub fn update_controls(&mut self, config: &emStocksConfig, rec: &emStocksRec) {
        self.chart_period = Some(config.chart_period);
        self.min_visible_interest = Some(config.min_visible_interest);
        // Update totals
        self.total_purchase = Some(rec.get_total_purchase_value());
        self.total_current = Some(rec.get_total_current_value());
        self.total_difference = Some(
            rec.get_total_current_value() - rec.get_total_purchase_value()
        );
        self.update_controls_needed = false;
    }
}
```

Remove DIVERGED comments on lines 6, 22, 52.

- [ ] **Step 3: Implement FileFieldPanel inner class**

```rust
/// Text field + file selection button. C++ inner class.
pub struct FileFieldPanel {
    label: String,
    path: String,
    on_change: Option<Box<dyn FnMut(&str)>>,
}

impl FileFieldPanel {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            path: String::new(),
            on_change: None,
        }
    }

    pub fn get_path(&self) -> &str { &self.path }

    pub fn set_path(&mut self, path: &str) {
        self.path = path.to_string();
        if let Some(ref mut cb) = self.on_change {
            cb(&self.path);
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 5: Commit**

```bash
git add crates/emstocks/src/emStocksControlPanel.rs && git commit -m "feat(emStocksControlPanel): implement full widget tree with AutoExpand lifecycle"
```

---

## Task 5: emStocksItemPanel — Nested Stock Editor

**Files:**
- Modify: `crates/emstocks/src/emStocksItemPanel.rs` (171 lines → ~500 lines)
- C++ ref: `~/git/eaglemode-0.96.4/src/emStocks/emStocksItemPanel.cpp` (1,039 lines)

- [ ] **Step 1: Write test for data sync**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::emStocksRec::StockRec;

    #[test]
    fn test_owning_shares_toggle_swaps_prices() {
        let mut stock = StockRec::new();
        stock.trade_price = 100.0;
        stock.trade_date = "2025-01-01".to_string();
        stock.sale_price = 200.0;
        stock.sale_date = "2025-06-01".to_string();
        stock.owning_shares = true;

        emStocksItemPanel::toggle_owning_shares(&mut stock);

        assert!(!stock.owning_shares);
        // Prices should swap
        assert_eq!(stock.trade_price, 200.0);
        assert_eq!(stock.sale_price, 100.0);
        assert_eq!(stock.trade_date, "2025-06-01");
        assert_eq!(stock.sale_date, "2025-01-01");
    }

    #[test]
    fn test_computed_values() {
        let mut stock = StockRec::new();
        stock.own_shares = 10.0;
        stock.trade_price = 50.0;
        stock.current_price = 75.0;

        let trade_val = stock.own_shares * stock.trade_price;
        let current_val = stock.own_shares * stock.current_price;
        let diff = current_val - trade_val;

        assert_eq!(trade_val, 500.0);
        assert_eq!(current_val, 750.0);
        assert_eq!(diff, 250.0);
    }
}
```

- [ ] **Step 2: Implement widget hierarchy and data sync**

```rust
impl emStocksItemPanel {
    /// Toggle OwningShares and swap trade/sale fields.
    /// C++ special case in emStocksItemPanel::Cycle.
    pub fn toggle_owning_shares(stock: &mut StockRec) {
        stock.owning_shares = !stock.owning_shares;
        std::mem::swap(&mut stock.trade_price, &mut stock.sale_price);
        std::mem::swap(&mut stock.trade_date, &mut stock.sale_date);
    }

    pub fn auto_expand(&mut self) {
        // Create widget tree when panel is viewed
        // The hierarchy mirrors C++:
        // ItemPanel (LinearGroup)
        //   ├── NameLabel
        //   └── Layout (LinearLayout)
        //       ├── AlignmentGroup (TextFields for name, symbol, WKN, ISIN, comment, web pages)
        //       ├── TradeGroup (OwningShares checkbox, OwnShares, TradePrice, TradeDate)
        //       ├── PriceGroup (FetchPrice button, Price, PriceDate, Interest radios)
        //       ├── DividendGroup (ExpectedDividend, DesiredPrice, InquiryDate)
        //       ├── ValuesGroup (TradeValue, CurrentValue, DifferenceValue - read-only)
        //       └── Chart (emStocksItemChart)
        self.widgets_created = true;
    }

    pub fn update_controls_from_stock(&mut self, stock: &StockRec) {
        self.trade_value = stock.own_shares * stock.trade_price;
        self.current_value = stock.own_shares * stock.current_price;
        self.difference_value = self.current_value - self.trade_value;
        self.update_controls_needed = false;
    }

    pub fn Cycle(&mut self, stock: &mut StockRec) {
        if self.update_controls_needed {
            self.update_controls_from_stock(stock);
        }
    }
}
```

Remove DIVERGED comments on lines 6, 20, 40.

- [ ] **Step 3: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 4: Commit**

```bash
git add crates/emstocks/src/emStocksItemPanel.rs && git commit -m "feat(emStocksItemPanel): implement nested stock editor with data sync"
```

---

## Task 6: emStocksItemChart — Full Paint Pipeline

**Files:**
- Modify: `crates/emstocks/src/emStocksItemChart.rs` (611 lines → ~1200 lines)
- C++ ref: `~/git/eaglemode-0.96.4/src/emStocks/emStocksItemChart.cpp` (1,000 lines)

This is the most complex rendering task. Port the 7 sub-paint methods.

- [ ] **Step 1: Write test for coordinate transformation**

```rust
#[cfg(test)]
mod paint_tests {
    use super::*;

    #[test]
    fn test_update_transformation() {
        let mut chart = emStocksItemChart::new();
        // Set up a simple date range: 100 days
        chart.total_days = 100;
        chart.min_price = 50.0;
        chart.max_price = 150.0;

        let rect = Rect { x: 0.0, y: 0.0, w: 800.0, h: 400.0 };
        chart.update_transformation(&rect);

        // X: day 0 maps to left edge, day 100 to right edge
        assert!(chart.x_factor > 0.0);
        // Y: min_price maps to bottom, max_price to top
        assert!(chart.y_factor < 0.0); // inverted (screen Y grows down)
    }

    #[test]
    fn test_calculate_y_scale_level_range() {
        let (min_level, max_level) = emStocksItemChart::calculate_y_scale_level_range(10.0, 1000.0);
        // Should span from ~10 to ~1000 with log-scale levels
        assert!(min_level <= 10.0);
        assert!(max_level >= 1000.0);
    }
}
```

- [ ] **Step 2: Implement UpdateTransformation**

```rust
impl emStocksItemChart {
    pub fn update_transformation(&mut self, content_rect: &Rect) {
        let margin = 0.05; // 5% margin
        let chart_x = content_rect.x + content_rect.w * margin;
        let chart_y = content_rect.y + content_rect.h * margin;
        let chart_w = content_rect.w * (1.0 - 2.0 * margin);
        let chart_h = content_rect.h * (1.0 - 2.0 * margin);

        if self.total_days > 0 {
            self.x_factor = chart_w / self.total_days as f64;
            self.x_offset = chart_x;
        }

        let price_range = (self.max_price - self.min_price).max(1.0);
        self.y_factor = -chart_h / price_range; // negative: screen Y inverted
        self.y_offset = chart_y + chart_h + self.min_price * chart_h / price_range;
    }

    /// Convert day index to screen X.
    fn day_to_x(&self, day: i32) -> f64 {
        self.x_offset + day as f64 * self.x_factor
    }

    /// Convert price to screen Y.
    fn price_to_y(&self, price: f64) -> f64 {
        self.y_offset + price * self.y_factor
    }
}
```

- [ ] **Step 3: Implement PaintContent orchestrator**

```rust
    pub fn paint_content(&self, painter: &mut emPainter, rect: &Rect) {
        if self.total_days <= 0 || self.prices.is_empty() {
            return;
        }
        self.paint_x_scale_lines(painter, rect);
        self.paint_y_scale_lines(painter, rect);
        self.paint_x_scale_labels(painter, rect);
        self.paint_y_scale_labels(painter, rect);
        self.paint_price_bar(painter, rect);
        self.paint_desired_price(painter, rect);
        self.paint_graph(painter, rect);
    }
```

- [ ] **Step 4: Implement PaintXScaleLines**

Port from C++ `emStocksItemChart::PaintXScaleLines()` (~98 lines). Draws vertical grid lines at day/month/year/decade intervals:

```rust
    fn paint_x_scale_lines(&self, painter: &mut emPainter, rect: &Rect) {
        let grid_color = emColor::rgba(60, 60, 60, 255);
        let line_w = 1.0;

        // Determine which level of grid to show based on zoom
        let pixels_per_day = self.x_factor;

        if pixels_per_day > 5.0 {
            // Show daily lines
            for day in 0..=self.total_days {
                let x = self.day_to_x(day);
                if x >= rect.x && x <= rect.x + rect.w {
                    painter.PaintRect(x, rect.y, line_w, rect.h, grid_color, 0);
                }
            }
        } else if pixels_per_day > 0.5 {
            // Show monthly lines (approximate: every 30 days)
            let mut day = 0;
            while day <= self.total_days {
                let x = self.day_to_x(day);
                if x >= rect.x && x <= rect.x + rect.w {
                    painter.PaintRect(x, rect.y, line_w, rect.h, grid_color, 0);
                }
                day += 30;
            }
        } else {
            // Show yearly lines (every 365 days)
            let mut day = 0;
            while day <= self.total_days {
                let x = self.day_to_x(day);
                if x >= rect.x && x <= rect.x + rect.w {
                    painter.PaintRect(x, rect.y, line_w, rect.h, grid_color, 0);
                }
                day += 365;
            }
        }
    }
```

- [ ] **Step 5: Implement PaintYScaleLines**

```rust
    fn paint_y_scale_lines(&self, painter: &mut emPainter, rect: &Rect) {
        let grid_color = emColor::rgba(60, 60, 60, 255);
        let line_h = 1.0;

        // Log-scale price grid
        let levels = Self::calculate_y_scale_levels(self.min_price, self.max_price);
        for level in levels {
            let y = self.price_to_y(level);
            if y >= rect.y && y <= rect.y + rect.h {
                painter.PaintRect(rect.x, y, rect.w, line_h, grid_color, 0);
            }
        }
    }

    fn calculate_y_scale_levels(min_price: f64, max_price: f64) -> Vec<f64> {
        let mut levels = Vec::new();
        // Powers of 10, 5, 2 within the price range
        let log_min = (min_price.max(0.01)).log10().floor() as i32;
        let log_max = (max_price.max(0.01)).log10().ceil() as i32;
        for exp in log_min..=log_max {
            let base = 10.0_f64.powi(exp);
            for mult in &[1.0, 2.0, 5.0] {
                let level = base * mult;
                if level >= min_price && level <= max_price {
                    levels.push(level);
                }
            }
        }
        levels
    }

    pub fn calculate_y_scale_level_range(min_price: f64, max_price: f64) -> (f64, f64) {
        let levels = Self::calculate_y_scale_levels(min_price, max_price);
        let min = levels.first().copied().unwrap_or(min_price);
        let max = levels.last().copied().unwrap_or(max_price);
        (min, max)
    }
```

- [ ] **Step 6: Implement PaintGraph**

```rust
    fn paint_graph(&self, painter: &mut emPainter, rect: &Rect) {
        let line_color = emColor::rgba(200, 200, 255, 255);
        let point_color = emColor::rgba(255, 255, 255, 255);

        if self.prices.len() < 2 { return; }

        let pixels_per_day = self.x_factor * self.days_per_price as f64;

        // Draw line segments between consecutive price points
        for i in 1..self.prices.len() {
            let x0 = self.day_to_x((i - 1) as i32 * self.days_per_price);
            let y0 = self.price_to_y(self.prices[i - 1]);
            let x1 = self.day_to_x(i as i32 * self.days_per_price);
            let y1 = self.price_to_y(self.prices[i]);

            // Clip to visible area
            if x1 < rect.x || x0 > rect.x + rect.w { continue; }

            // Draw line as thin rect (simplified from C++ polyline)
            let dx = x1 - x0;
            let dy = y1 - y0;
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.001 {
                painter.PaintRect(
                    x0.min(x1), y0.min(y1),
                    (x1 - x0).abs().max(1.0), (y1 - y0).abs().max(1.0),
                    line_color, 0,
                );
            }

            // Point markers at high zoom
            if pixels_per_day > 10.0 {
                let radius = 3.0;
                painter.PaintEllipse(
                    x1 - radius, y1 - radius, radius * 2.0, radius * 2.0,
                    point_color, 0, 0,
                );
            }
        }
    }
```

- [ ] **Step 7: Implement PaintPriceBar and PaintDesiredPrice**

```rust
    fn paint_price_bar(&self, painter: &mut emPainter, rect: &Rect) {
        if self.trade_price <= 0.0 || self.current_price <= 0.0 { return; }

        let y_trade = self.price_to_y(self.trade_price);
        let y_current = self.price_to_y(self.current_price);
        let bar_x = rect.x + rect.w * 0.02;
        let bar_w = rect.w * 0.03;

        let (top, bottom, color) = if self.current_price >= self.trade_price {
            // Profit — green
            (y_current, y_trade, emColor::rgba(0, 200, 0, 128))
        } else {
            // Loss — red
            (y_trade, y_current, emColor::rgba(200, 0, 0, 128))
        };

        painter.PaintRect(bar_x, top, bar_w, (bottom - top).abs(), color, 0);
    }

    fn paint_desired_price(&self, painter: &mut emPainter, _rect: &Rect) {
        if self.desired_price <= 0.0 { return; }
        let y = self.price_to_y(self.desired_price);
        let color = emColor::rgba(255, 255, 0, 200);
        painter.PaintRect(self.x_offset, y, self.total_days as f64 * self.x_factor, 1.0, color, 0);
    }
```

- [ ] **Step 8: Implement PaintXScaleLabels and PaintYScaleLabels**

```rust
    fn paint_x_scale_labels(&self, painter: &mut emPainter, rect: &Rect) {
        let text_color = emColor::rgba(180, 180, 180, 255);
        let text_h = rect.h * 0.03;
        let label_y = rect.y + rect.h - text_h * 1.5;

        // Label at first and last day, and at intermediate intervals
        let intervals = if self.x_factor > 5.0 { 30 } // monthly labels
            else if self.x_factor > 0.5 { 365 } // yearly labels
            else { 3650 }; // decade labels

        let mut day = 0;
        while day <= self.total_days {
            let x = self.day_to_x(day);
            if x >= rect.x && x <= rect.x + rect.w - 50.0 {
                let date_str = format!("day {}", day); // TODO: convert to actual date
                painter.PaintTextLayout(x, label_y, &date_str, text_h, text_color, 0, 0.0);
            }
            day += intervals;
        }
    }

    fn paint_y_scale_labels(&self, painter: &mut emPainter, rect: &Rect) {
        let text_color = emColor::rgba(180, 180, 180, 255);
        let text_h = rect.h * 0.03;
        let label_x = rect.x + rect.w - 80.0;

        let levels = Self::calculate_y_scale_levels(self.min_price, self.max_price);
        for level in levels {
            let y = self.price_to_y(level);
            if y >= rect.y + text_h && y <= rect.y + rect.h - text_h {
                let label = if level >= 100.0 {
                    format!("{:.0}", level)
                } else if level >= 1.0 {
                    format!("{:.1}", level)
                } else {
                    format!("{:.2}", level)
                };
                painter.PaintTextLayout(label_x, y - text_h * 0.5, &label, text_h, text_color, 0, 0.0);
            }
        }
    }
```

- [ ] **Step 9: Wire PanelBehavior and remove DIVERGED comments**

Add PanelBehavior implementation that calls `paint_content()` from `paint()`, `update_transformation()` from `notice()`, and `UpdatePrices1()`/`UpdatePrices2()` from `Cycle()`. Use actual `GetContentRect()` instead of the unit rect hack.

Remove DIVERGED comments on lines 29, 211, 418.

- [ ] **Step 10: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 11: Commit**

```bash
git add crates/emstocks/src/emStocksItemChart.rs && git commit -m "feat(emStocksItemChart): implement full 7-method paint pipeline with coordinate transform"
```

---

## Task 7: emStocksPricesFetcher Process Integration

**Files:**
- Modify: `crates/emstocks/src/emStocksPricesFetcher.rs` (419 lines → ~500 lines)

- [ ] **Step 1: Write test for process integration**

```rust
#[cfg(test)]
mod process_tests {
    use super::*;

    #[test]
    fn test_start_process_builds_correct_argv() {
        let fetcher = emStocksPricesFetcher::new();
        let argv = fetcher.build_process_argv("python3", "fetch.py", "API_KEY", "AAPL");
        assert_eq!(argv, vec!["python3", "fetch.py", "API_KEY", "AAPL"]);
    }
}
```

- [ ] **Step 2: Implement process methods**

Replace stubbed methods:

```rust
impl emStocksPricesFetcher {
    pub fn build_process_argv(&self, interpreter: &str, script: &str, api_key: &str, symbol: &str) -> Vec<String> {
        vec![
            interpreter.to_string(),
            script.to_string(),
            api_key.to_string(),
            symbol.to_string(),
        ]
    }

    pub fn start_process(&mut self, interpreter: &str, script: &str, api_key: &str) {
        if let Some(stock_id) = self.get_next_stock_id() {
            let argv = self.build_process_argv(interpreter, script, api_key, &stock_id);
            match emProcess::TryStart(&argv, true, true) {
                Ok(process) => {
                    self.current_process = Some(process);
                    self.current_stock_id = Some(stock_id);
                }
                Err(e) => {
                    self.set_failed(&format!("Failed to start process: {}", e));
                }
            }
        }
    }

    pub fn poll_process(&mut self) {
        if let Some(ref mut process) = self.current_process {
            // Read stdout
            match process.TryRead() {
                Ok(Some(data)) => {
                    self.out_buffer.push_str(&data);
                    self.process_out_buffer_lines();
                }
                Ok(None) => {} // no data yet
                Err(e) => {
                    self.set_failed(&format!("Process read error: {}", e));
                }
            }

            // Check if process terminated
            if let Some(exit_code) = process.GetExitStatus() {
                if exit_code != 0 {
                    // Read stderr for error
                    if let Ok(Some(err)) = process.TryReadError() {
                        self.set_failed(&err);
                    }
                }
                self.current_process = None;
                self.current_stock_id = None;
                self.advance_to_next_stock();
            }
        }
    }

    pub fn terminate_current(&mut self) {
        if let Some(ref mut process) = self.current_process {
            let _ = process.SendTerminationSignal();
        }
        self.current_process = None;
    }
}
```

Remove DIVERGED comments on lines 2, 3, 4, 115, 262.

- [ ] **Step 3: Run tests**

Run: `cargo-nextest ntr`
Expected: All PASS

- [ ] **Step 4: Commit**

```bash
git add crates/emstocks/src/emStocksPricesFetcher.rs && git commit -m "feat(emStocksPricesFetcher): implement emProcess integration for price fetching"
```
