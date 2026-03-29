# emStocks Port Design

Date: 2026-03-29

## Objective

Port the C++ emStocks app module to Rust as the first outside-emCore app,
stress-testing the newly ported emCore types (emArray, emAvlTreeMap, emList,
emCrossPtr, emFileStream) and proving emCore completeness. Work is LLM-driven
with human review.

## Rationale

emStocks is the highest-value emCore stress-test among all C++ app modules:
- Uses emAvlTreeMap (ordered map with COW), emList (COW linked list),
  emCrossPtr (explicit invalidation), emArray (COW array) — all ported in
  Phases 2-4
- Uses emConfigModel, emRecFileModel, emProcess, emEngine, emTimer — core
  infrastructure types
- 10 headers, 11 source files — substantial but bounded
- No dependencies on other app modules (fully self-contained with emCore)
- Real data flow: fetch -> parse -> store -> render charts — exercises emCore
  end-to-end

## Scope

- 1 emCore gap to close (emAbsoluteFileModelClient in emFileModel.rs)
- 11 new Rust files in src/emStocks/ (10 type files + mod.rs)
- Test expansion across unit, behavioral, golden, integration, and pipeline
  layers

---

## Section 1: Principles & Constraints

### Port-don't-skip (inherited)

Every emStocks type gets a Rust equivalent. If porting reveals that a .no_rs
type's stdlib replacement doesn't cover a usage pattern, build the Rust
equivalent then. The app port is the forcing function.

### File and Name Correspondence (inherited)

Each C++ header in include/emStocks/ gets a .rs file in src/emStocks/ with the
same name. Class names, method names, and field names in the public API match
C++ unless annotated with DIVERGED.

### emPainter firewall (inherited)

Do not refactor any emPainter*.rs file as blast radius. emStocksItemChart calls
emPainter methods but does not modify emPainter itself.

### No standalone reimplementations (inherited)

If emStocks needs functionality that an emCore type provides, use the emCore
type. Do not reimplement locally.

### Static registration

emStocks is compiled as part of the main crate (src/emStocks/ as a sibling
module to src/emCore/). The plugin system's dynamic loading layer is not yet
wired up. emStocksFpPlugin.rs provides a public registration function that
the main binary calls at startup to register the .emStocks file type handler
with emFpPluginList. No .emFpPlugin config file needed at this stage. When
dynamic loading is implemented later, emStocks moves to a separate crate with
no code changes beyond crate boundaries.

---

## Section 2: emCore Gaps

### Gap 1: emAbsoluteFileModelClient

**File:** emFileModel.rs (where C++ defines it in emFileModel.h)

**What:** Small wrapper that holds `Rc<RefCell<T>>` + `SignalId` for tracking
a file model's validity and change notifications. C++ emStocksPricesFetcher
uses this to hold a validity-tracked reference to emStocksFileModel.

**Design:** Struct with `get()` (returns Option if model still valid),
`IsSignaled()` (checks change signal), and `SetModel()` / `ClearModel()`
for lifecycle management.

**Testing:** Unit tests for validity tracking, behavioral test for signal
integration.

### Gap 2: emEnumRec (no emCore change needed)

C++ InterestRec extends emEnumRec. Rust handles this with plain enums +
`From<&str>` / `Display` impls in emStocksRec.rs. The Record trait's
`from_rec()` / `to_rec()` converts between RecValue::Ident and the enum.
No new emCore type.

### Gap 3: emRecListener (no emCore change needed)

C++ multiple-inheritance pattern (class Foo : emRecListener) translates to
registering callbacks on RecListenerList. Existing API is sufficient. No new
emCore type.

---

## Section 3: Module Structure

### File layout

```
src/emStocks/
  mod.rs                        -- module declaration, pub(crate) re-exports
  emStocksRec.rs                -- emStocksRec, StockRec, Interest enum
  emStocksConfig.rs             -- emStocksConfig, ChartPeriod, Sorting enums
  emStocksFileModel.rs          -- emStocksFileModel
  emStocksPricesFetcher.rs      -- emStocksPricesFetcher
  emStocksListBox.rs            -- emStocksListBox
  emStocksItemPanel.rs          -- emStocksItemPanel, CategoryPanel
  emStocksItemChart.rs          -- emStocksItemChart
  emStocksFetchPricesDialog.rs  -- emStocksFetchPricesDialog, ProgressBarPanel
  emStocksFilePanel.rs          -- emStocksFilePanel
  emStocksControlPanel.rs       -- emStocksControlPanel, FileFieldPanel, CategoryPanel
  emStocksFpPlugin.rs           -- Plugin registration function
```

Nested types (StockRec, InterestRec, CategoryPanel, ProgressBarPanel,
FileFieldPanel) stay in the same file as their parent, matching C++ nested
classes.

### Dependency order (build sequence)

```
emStocksRec (foundation — no emStocks deps)
  |
  +-- emStocksConfig (depends on emStocksRec for Interest type)
  +-- emStocksFileModel (depends on emStocksRec)
        |
        +-- emStocksPricesFetcher (depends on FileModel, ListBox)
        +-- emStocksItemChart (depends on Config, FileModel)
        +-- emStocksItemPanel (depends on Config, FileModel, ItemChart)
              |
              +-- emStocksListBox (depends on ItemPanel)
                    |
                    +-- emStocksControlPanel (depends on ListBox, Config, FileModel)
                    +-- emStocksFetchPricesDialog (depends on PricesFetcher)
                    +-- emStocksFilePanel (depends on ListBox, ControlPanel)
                          |
                          +-- emStocksFpPlugin (depends on FilePanel)
```

---

## Section 4: Type Designs

### emStocksRec

```
Interest enum: High, Medium, Low
  - From<&str> handles "HIGH"/"MEDIUM"/"LOW" plus deprecated identifiers
  - Display outputs canonical names

StockRec struct:
  - 18 fields matching C++ (id, name, symbol, wkn, isin, country, sector,
    collection, comment, owning_shares, own_shares, trade_price, trade_date,
    prices, last_price_date, desired_price, expected_dividend, inquiry_date,
    interest, web_pages)
  - cross_ptr_list: emCrossPtrList for LinkCrossPtr support
  - Methods: GetLatestPricesDate, AddDaysToDate, price query/calculation
    methods matching C++ names
  - Implements Record trait

emStocksRec struct:
  - stocks: Vec<StockRec>
  - listener_list: RecListenerList
  - Implements Record trait
```

DIVERGED: Rust struct fields use snake_case. Method names preserve C++ names.
StockRec fields are pub — change notification happens at emStocksRec level
via RecListenerList, not per-field setters.

### emStocksConfig

```
ChartPeriod enum: Week1, Month1, Month3, Month6, Year1, Year2, Year5,
                  Year10, Year20
Sorting enum: ByName, BySymbol, ByCountry, BySector, ByCollection,
              ByInterest, BySharePrice, ByDifference

emStocksConfig struct:
  - api_script, api_script_interpreter, api_key, web_browser: String
  - auto_update_dates, triggering_opens_web_page, owned_shares_first: bool
  - chart_period: ChartPeriod
  - min_visible_interest: Interest
  - visible_countries, visible_sectors, visible_collections: Vec<String>
  - sorting: Sorting
  - search_text: String
  - Acquired via Acquire(context, name) -> Rc<RefCell<Self>>
  - Implements Record trait
```

### emStocksFileModel

```
emStocksFileModel struct:
  - rec: emStocksRec
  - file_model: emRecFileModel<emStocksRec>
  - save_timer: emTimer (15-second delayed save)
  - prices_fetching_dialog: emCrossPtr<emStocksFetchPricesDialog>
  - OnRecChanged() starts save timer
  - Cycle() checks save timer signal, triggers Save(true)
  - Acquired via Acquire(context, path) -> Rc<RefCell<Self>>
```

### emStocksPricesFetcher

```
emStocksPricesFetcher struct:
  - file_model: Rc<RefCell<emStocksFileModel>>
  - list_boxes: emList<emCrossPtr<emStocksListBox>>
  - stock_ids: emArray<String>
  - stock_recs_map: emAvlTreeMap<String, emCrossPtr<StockRec>>
  - current_process: Option<emProcess>
  - current_stock_id: String
  - out_buffer: emArray<u8>
  - err_buffer: emArray<u8>
  - Implements emEngine trait (Cycle method)
  - Methods: AddListBox, AddStockIds, GetCurrentStockId,
    GetCurrentStockRec, StartProcess, PollProcess
```

Key emCore stress points:
  - emList<emCrossPtr<T>>: COW list containing cross-pointers
  - emAvlTreeMap<String, emCrossPtr<T>>: ordered map with cross-pointer values
  - emProcess: spawns external script, polls stdout/stderr incrementally
  - emEngine::Cycle(): cooperative scheduling with process I/O

### UI Panels (emStocksItemChart through emStocksFilePanel)

All UI panels follow the same pattern: extend an emCore widget base class,
implement PaintContent/LayoutChildren/Input/Cycle as needed, and hold
references to the data layer via Rc<RefCell<T>> or emCrossPtr<T>.

- emStocksItemChart: extends emBorder, custom PaintContent with emPainter
  (lines, rects, text for price chart), emRecListener for reactive updates
- emStocksItemPanel: extends emLinearGroup, contains toolkit widgets
  (TextField, CheckButton, RadioButton) + ItemChart child, nested
  CategoryPanel
- emStocksListBox: extends emListBox, creates ItemPanel instances as items,
  sorting/filtering/selection/clipboard
- emStocksControlPanel: extends emLinearGroup, settings widgets, nested
  FileFieldPanel and CategoryPanel
- emStocksFetchPricesDialog: extends emDialog, wraps PricesFetcher,
  nested ProgressBarPanel with custom painting
- emStocksFilePanel: extends emFilePanel, top-level container creating
  ListBox + ControlPanel
- emStocksFpPlugin: static registration function

---

## Section 5: Phase Structure

### Phase 1 -- emCore Gap + Data Layer

**Goal:** Close emAbsoluteFileModelClient gap. Port emStocksRec with full
Record round-trip. Port emStocksConfig and emStocksFileModel.

Work items:
1. emAbsoluteFileModelClient in emFileModel.rs
2. emStocksRec.rs (Interest, StockRec, emStocksRec, Record impls, date math)
3. emStocksConfig.rs (ChartPeriod, Sorting, Record impl, Acquire)
4. emStocksFileModel.rs (save timer, emCrossPtr dialog field, Acquire)
5. src/emStocks/mod.rs module setup

Testing:
- Unit tests for Interest/ChartPeriod/Sorting enum parsing
- Unit tests for StockRec/emStocksRec/emStocksConfig Record round-trip
- Unit tests for date arithmetic methods
- Behavioral tests for emAbsoluteFileModelClient validity tracking
- Behavioral tests for emStocksFileModel save timer + emCrossPtr dialog
- cargo clippy + cargo-nextest pass

Gate: All data layer tests pass. A .emStocks file can be loaded into
emStocksRec, round-tripped through Record, and saved identically.

### Phase 2 -- Engine Layer

**Goal:** Port emStocksPricesFetcher. This is the primary emCore stress-test.

Work items:
1. emStocksPricesFetcher.rs (full Cycle implementation, process management)

Testing:
- Unit tests for output parsing logic
- Behavioral tests for emList<emCrossPtr> invalidation
- Behavioral tests for emAvlTreeMap<String, emCrossPtr> with invalid entries
- Integration test: Cycle() with a simple echo script
- Verify emCore COW types handle the composition patterns correctly

Gate: PricesFetcher can spawn a process, read output, parse prices, and
update StockRec entries. emCrossPtr invalidation works correctly when
ListBoxes or StockRecs are dropped.

### Phase 3 -- UI Panels

**Goal:** Port all 6 panel types + plugin registration.

Work items (bottom-up):
1. emStocksItemChart.rs
2. emStocksItemPanel.rs (+ CategoryPanel)
3. emStocksListBox.rs
4. emStocksControlPanel.rs (+ FileFieldPanel, CategoryPanel)
5. emStocksFetchPricesDialog.rs (+ ProgressBarPanel)
6. emStocksFilePanel.rs
7. emStocksFpPlugin.rs

Testing:
- Golden tests for emStocksItemChart price chart rendering
- Pipeline tests for widget composition (ItemPanel, ControlPanel)
- Pipeline tests for dialog lifecycle (FetchPricesDialog)
- Integration test: load .emStocks file -> verify full panel tree
- Unit tests for ListBox sorting/filtering logic

Gate: emStocks panel tree renders correctly from a .emStocks file. All
widget interactions (buttons, text fields, checkboxes) work. Chart
renders price data.

### Phase 4 -- Integration & Polish

**Goal:** End-to-end verification, fix any emCore gaps discovered.

Work items:
1. End-to-end test: load file -> display -> edit -> save -> reload
2. Fix any emCore gaps discovered during Phases 1-3
3. Update docs/CORRESPONDENCE.md with emStocks port status
4. Final test audit across all phases

Gate: emStocks is fully functional. All tests pass. CORRESPONDENCE.md
updated. Any emCore gaps found are closed with tests.

---

## Section 6: Testing Strategy

### Layer coverage

| Component | Unit | Behavioral | Golden | Integration | Pipeline |
|---|---|---|---|---|---|
| emAbsoluteFileModelClient | x | x | | | |
| Interest / enums | x | | | | |
| StockRec | x | | | | |
| emStocksRec | x | x (listeners) | | | |
| emStocksConfig | x | x (Acquire) | | | |
| emStocksFileModel | x | x (timer, emCrossPtr) | | | |
| emStocksPricesFetcher | x | x (COW+emCrossPtr) | | x (Cycle+process) | |
| emStocksItemChart | x | | x (chart rendering) | | |
| emStocksListBox | x (sort/filter) | | | | x (items) |
| emStocksControlPanel | | | | | x (widgets) |
| emStocksFetchPricesDialog | | | | | x (dialog) |
| emStocksFilePanel | | | | x (load -> panel tree) | |

### Key behavioral tests for emCore stress-testing

1. **COW + emCrossPtr interaction:** Clone an emList containing
   cross-pointers, invalidate one, verify both copies handle it correctly
   (original sees invalid, clone sees invalid independently).

2. **emAvlTreeMap ordered iteration with invalid entries:** Insert
   cross-pointer values, invalidate some, iterate in order, verify
   invalid entries are detectable without crashing.

3. **emEngine Cycle() with emProcess I/O:** Verify cooperative scheduling
   correctly interleaves process polling with other engine work.

4. **emRecFileModel state machine under I/O errors:** Verify error states
   are reachable and recoverable.

5. **emTimer signal integration:** Verify delayed save fires at correct
   time, re-fires on subsequent changes.

### Golden test approach for ItemChart

Generate C++ reference images if feasible (build emStocks C++ and capture
chart output). If not feasible, establish Rust baselines with visual
verification, then lock as golden references. Document provenance.

### No Kani proofs

emStocks has no fixed-point arithmetic or integer math feeding rendering.
Price chart uses f64 geometry covered by golden tests.

### Test audit protocol (inherited)

Before relying on any existing test as a regression gate:
1. Check golden reference data provenance
2. Check behavioral assertions against C++ contracts
3. Check that the test exercises the code path being changed
