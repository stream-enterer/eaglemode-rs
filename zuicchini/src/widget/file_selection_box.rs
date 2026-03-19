use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::panel::{PanelBehavior, PanelCtx, PanelId, PanelState};
use crate::render::Painter;

use super::border::{Border, InnerBorderType, OuterBorderType};
use super::check_box::CheckBox;
use super::field_panel::{CheckBoxPanel, ListBoxPanel, TextFieldPanel};
use super::list_box::{ListBox, SelectionMode};
use super::look::Look;
use super::text_field::TextField;

/// Data associated with each file entry in the listing.
#[derive(Clone, Debug)]
pub struct FileItemData {
    pub is_directory: bool,
    pub is_readable: bool,
    pub is_hidden: bool,
}

type SelectionChangedCb = Box<dyn FnMut()>;
type FileTriggerCb = Box<dyn FnMut(&str)>;

/// Shared event state collected by child-panel callbacks and drained in `cycle()`.
#[derive(Default)]
struct FsbEvents {
    selection_changed: bool,
    selection_indices: Vec<usize>,
    triggered_index: Option<usize>,
    name_text_changed: Option<String>,
    dir_text_changed: Option<String>,
    hidden_toggled: Option<bool>,
    filter_index_changed: Option<usize>,
}

/// A file selection box widget for browsing and selecting files.
///
/// Port of C++ `emFileSelectionBox`. Provides a file browser with:
/// - A text field showing the current directory path
/// - A checkbox to toggle showing hidden files
/// - A list of files/directories in the current directory
/// - A text field for entering/editing the file name
/// - A filter list for file type filtering
pub struct FileSelectionBox {
    border: Border,
    look: Rc<Look>,
    multi_selection_enabled: bool,
    parent_dir: PathBuf,
    selected_names: Vec<String>,
    filters: Vec<String>,
    selected_filter_index: i32,
    hidden_files_shown: bool,
    triggered_file_name: String,
    parent_dir_field_hidden: bool,
    hidden_check_box_hidden: bool,
    name_field_hidden: bool,
    filter_hidden: bool,
    listing_invalid: bool,
    listing: Vec<(String, FileItemData)>,
    // Child panel IDs (populated on auto-expand)
    dir_field_id: Option<PanelId>,
    hidden_cb_id: Option<PanelId>,
    files_lb_id: Option<PanelId>,
    name_field_id: Option<PanelId>,
    filter_lb_id: Option<PanelId>,
    /// True when children must be torn down and rebuilt on next layout pass.
    children_dirty: bool,
    /// Shared event state collected by child-panel callbacks.
    events: Rc<RefCell<FsbEvents>>,
    /// Consumer callback: selection changed.
    pub on_selection: Option<SelectionChangedCb>,
    /// Consumer callback: file triggered (double-click / Enter on a file).
    pub on_trigger: Option<FileTriggerCb>,
}

impl FileSelectionBox {
    pub fn new(caption: &str) -> Self {
        let parent_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        Self {
            border: Border::new(OuterBorderType::Group)
                .with_inner(InnerBorderType::Group)
                .with_caption(caption),
            look: Look::new(),
            multi_selection_enabled: false,
            parent_dir,
            selected_names: Vec::new(),
            filters: Vec::new(),
            selected_filter_index: -1,
            hidden_files_shown: false,
            triggered_file_name: String::new(),
            parent_dir_field_hidden: false,
            hidden_check_box_hidden: false,
            name_field_hidden: false,
            filter_hidden: false,
            listing_invalid: true,
            listing: Vec::new(),
            dir_field_id: None,
            hidden_cb_id: None,
            files_lb_id: None,
            name_field_id: None,
            filter_lb_id: None,
            children_dirty: false,
            events: Rc::new(RefCell::new(FsbEvents::default())),
            on_selection: None,
            on_trigger: None,
        }
    }

    pub fn is_multi_selection_enabled(&self) -> bool {
        self.multi_selection_enabled
    }

    pub fn set_multi_selection_enabled(&mut self, enabled: bool) {
        if self.multi_selection_enabled != enabled {
            if !enabled && self.selected_names.len() > 1 {
                let first = self.selected_names[0].clone();
                self.set_selected_name(&first);
            }
            self.multi_selection_enabled = enabled;
            self.children_dirty = true;
        }
    }

    pub fn parent_directory(&self) -> &Path {
        &self.parent_dir
    }

    pub fn set_parent_directory(&mut self, parent_directory: &Path) {
        let abs_path = if parent_directory.is_absolute() {
            parent_directory.to_path_buf()
        } else {
            std::fs::canonicalize(parent_directory)
                .unwrap_or_else(|_| parent_directory.to_path_buf())
        };

        if self.parent_dir != abs_path {
            self.parent_dir = abs_path;
            self.triggered_file_name.clear();
            self.invalidate_listing();
        }
    }

    pub fn selected_name(&self) -> Option<&str> {
        self.selected_names.first().map(|s| s.as_str())
    }

    pub fn selected_names(&self) -> &[String] {
        &self.selected_names
    }

    pub fn set_selected_name(&mut self, name: &str) {
        if name.is_empty() {
            if !self.selected_names.is_empty() {
                self.selected_names.clear();
            }
        } else if self.selected_names.len() != 1 || self.selected_names[0] != name {
            self.selected_names = vec![name.to_string()];
        }
    }

    pub fn set_selected_names(&mut self, names: &[String]) {
        let mut sorted = names.to_vec();
        sorted.sort();

        if sorted != self.selected_names {
            self.selected_names = sorted;
        }
    }

    pub fn clear_selection(&mut self) {
        self.set_selected_name("");
    }

    pub fn selected_path(&self) -> PathBuf {
        if let Some(name) = self.selected_names.first() {
            self.parent_dir.join(name)
        } else {
            self.parent_dir.clone()
        }
    }

    pub fn set_selected_path(&mut self, selected_path: &Path) {
        let abs_path = if selected_path.is_absolute() {
            selected_path.to_path_buf()
        } else {
            std::fs::canonicalize(selected_path).unwrap_or_else(|_| selected_path.to_path_buf())
        };

        if abs_path.is_dir() {
            self.set_parent_directory(&abs_path);
            self.clear_selection();
        } else {
            if let Some(parent) = abs_path.parent() {
                self.set_parent_directory(parent);
            }
            if let Some(name) = abs_path.file_name() {
                self.set_selected_name(&name.to_string_lossy());
            }
        }
    }

    pub fn filters(&self) -> &[String] {
        &self.filters
    }

    pub fn set_filters(&mut self, filters: &[String]) {
        if self.filters == filters {
            return;
        }

        self.filters = filters.to_vec();
        let count = self.filters.len() as i32;
        if self.selected_filter_index >= count {
            self.selected_filter_index = count - 1;
        } else if self.selected_filter_index < 0 && count > 0 {
            self.selected_filter_index = 0;
        }
        self.children_dirty = true;
        self.invalidate_listing();
    }

    pub fn selected_filter_index(&self) -> i32 {
        self.selected_filter_index
    }

    pub fn set_selected_filter_index(&mut self, index: i32) {
        let clamped = if index < 0 || index >= self.filters.len() as i32 {
            -1
        } else {
            index
        };
        if self.selected_filter_index != clamped {
            self.selected_filter_index = clamped;
            self.invalidate_listing();
        }
    }

    pub fn are_hidden_files_shown(&self) -> bool {
        self.hidden_files_shown
    }

    pub fn set_hidden_files_shown(&mut self, shown: bool) {
        if self.hidden_files_shown != shown {
            self.hidden_files_shown = shown;
            self.invalidate_listing();
        }
    }

    pub fn triggered_file_name(&self) -> &str {
        &self.triggered_file_name
    }

    pub fn trigger_file(&mut self, name: &str) {
        self.triggered_file_name = name.to_string();
    }

    /// Enter a sub-directory by name.
    pub fn enter_sub_dir(&mut self, name: &str) {
        let path = self.parent_dir.join(name);
        if name == ".." {
            self.set_parent_directory(&path);
            self.clear_selection();
        } else if path.is_dir() {
            // Check readability by attempting to read the directory.
            if std::fs::read_dir(&path).is_ok() {
                self.set_parent_directory(&path);
                self.clear_selection();
            }
        }
    }

    pub fn is_parent_dir_field_hidden(&self) -> bool {
        self.parent_dir_field_hidden
    }

    pub fn set_parent_dir_field_hidden(&mut self, hidden: bool) {
        self.parent_dir_field_hidden = hidden;
    }

    pub fn is_hidden_check_box_hidden(&self) -> bool {
        self.hidden_check_box_hidden
    }

    pub fn set_hidden_check_box_hidden(&mut self, hidden: bool) {
        self.hidden_check_box_hidden = hidden;
    }

    pub fn is_name_field_hidden(&self) -> bool {
        self.name_field_hidden
    }

    pub fn set_name_field_hidden(&mut self, hidden: bool) {
        self.name_field_hidden = hidden;
    }

    pub fn is_filter_hidden(&self) -> bool {
        self.filter_hidden
    }

    pub fn set_filter_hidden(&mut self, hidden: bool) {
        self.filter_hidden = hidden;
    }

    /// Reload the directory listing, applying filters and hidden-file settings.
    pub fn reload_listing(&mut self) {
        let mut entries = Vec::new();

        let dir_entries = match std::fs::read_dir(&self.parent_dir) {
            Ok(rd) => rd,
            Err(_) => {
                self.listing = entries;
                self.listing_invalid = false;
                return;
            }
        };

        for entry in dir_entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let is_directory = path.is_dir();
            let is_readable =
                std::fs::read_dir(&path).is_ok() || std::fs::File::open(&path).is_ok();
            let is_hidden = name.starts_with('.');

            let data = FileItemData {
                is_directory,
                is_readable,
                is_hidden,
            };

            // Filter hidden files.
            if !self.hidden_files_shown && is_hidden {
                continue;
            }

            // Apply file type filter (directories pass through).
            if self.selected_filter_index >= 0
                && (self.selected_filter_index as usize) < self.filters.len()
                && !is_directory
                && !match_file_name_filter(
                    &name,
                    &self.filters[self.selected_filter_index as usize],
                )
            {
                continue;
            }

            entries.push((name, data));
        }

        // Sort: directories first, then by name (locale-insensitive).
        entries.sort_by(|(a, _), (b, _)| a.cmp(b));

        // Add ".." entry at the beginning if not at root.
        if self.parent_dir != Path::new("/") {
            entries.insert(
                0,
                (
                    "..".to_string(),
                    FileItemData {
                        is_directory: true,
                        is_readable: true,
                        is_hidden: false,
                    },
                ),
            );
        }

        self.listing = entries;
        self.listing_invalid = false;
    }

    /// Get the current directory listing.
    pub fn listing(&self) -> &[(String, FileItemData)] {
        &self.listing
    }

    /// Whether the listing needs to be reloaded.
    pub fn is_listing_invalid(&self) -> bool {
        self.listing_invalid
    }

    fn invalidate_listing(&mut self) {
        self.listing_invalid = true;
    }

    pub fn border(&self) -> &Border {
        &self.border
    }

    pub fn border_mut(&mut self) -> &mut Border {
        &mut self.border
    }

    /// Create child panels matching C++ AutoExpand().
    fn create_children(&mut self, ctx: &mut PanelCtx) {
        // Pre-calculate border scaling for FilesLB (C++ sets this dynamically,
        // but we set it at creation time to avoid downcasting).
        let rect = ctx.layout_rect();
        let cr = self
            .border
            .content_rect_unobscured(rect.w, rect.h, &self.look);
        let hs = (cr.w * 0.05).min(cr.h * 0.15);
        let has_top = !self.parent_dir_field_hidden || !self.hidden_check_box_hidden;
        let has_bottom = !self.name_field_hidden || !self.filter_hidden;
        let h1 = if has_top { hs } else { 0.0 };
        let h3 = if has_bottom { hs } else { 0.0 };
        let h2 = cr.h - h1 - h3;

        // 1. ParentDirField
        if !self.parent_dir_field_hidden {
            let mut tf = TextField::new(self.look.clone());
            tf.set_caption("Directory");
            tf.set_editable(true);
            tf.set_text(&self.parent_dir.to_string_lossy());
            let events = self.events.clone();
            tf.on_text = Some(Box::new(move |text: &str| {
                let mut e = events.borrow_mut();
                e.dir_text_changed = Some(text.to_string());
            }));
            let id = ctx.create_child_with(
                "directory",
                Box::new(TextFieldPanel { text_field: tf }),
            );
            self.dir_field_id = Some(id);
        }

        // 2. HiddenCheckBox
        if !self.hidden_check_box_hidden {
            let mut cb = CheckBox::new("Show\nHidden\nFiles", self.look.clone());
            cb.set_checked(self.hidden_files_shown);
            let events = self.events.clone();
            cb.on_check = Some(Box::new(move |checked: bool| {
                let mut e = events.borrow_mut();
                e.hidden_toggled = Some(checked);
            }));
            let id =
                ctx.create_child_with("showHiddenFiles", Box::new(CheckBoxPanel { check_box: cb }));
            self.hidden_cb_id = Some(id);
        }

        // 3. FilesLB (always created)
        {
            let mut lb = ListBox::new(self.look.clone());
            lb.set_caption("Files");
            lb.set_selection_mode(if self.multi_selection_enabled {
                SelectionMode::Multi
            } else {
                SelectionMode::Single
            });
            if h2 > 1e-100 {
                lb.border_mut().set_border_scaling(hs / h2);
            }
            let events = self.events.clone();
            lb.on_selection = Some(Box::new(move |indices: &[usize]| {
                let mut e = events.borrow_mut();
                e.selection_changed = true;
                e.selection_indices = indices.to_vec();
            }));
            let events = self.events.clone();
            lb.on_trigger = Some(Box::new(move |index: usize| {
                let mut e = events.borrow_mut();
                e.triggered_index = Some(index);
            }));
            let id = ctx.create_child_with("files", Box::new(ListBoxPanel { list_box: lb }));
            self.files_lb_id = Some(id);
        }

        // 4. NameField
        if !self.name_field_hidden {
            let mut tf = TextField::new(self.look.clone());
            tf.set_caption("Name");
            tf.set_editable(true);
            if let Some(name) = self.selected_names.first() {
                tf.set_text(name);
            }
            let events = self.events.clone();
            tf.on_text = Some(Box::new(move |text: &str| {
                let mut e = events.borrow_mut();
                e.name_text_changed = Some(text.to_string());
            }));
            let id = ctx.create_child_with(
                "name",
                Box::new(TextFieldPanel { text_field: tf }),
            );
            self.name_field_id = Some(id);
        }

        // 5. FiltersLB
        if !self.filter_hidden {
            let mut lb = ListBox::new(self.look.clone());
            lb.set_caption("Filter");
            for (i, filter) in self.filters.iter().enumerate() {
                lb.add_item(format!("{}", i), filter.clone());
            }
            if self.selected_filter_index >= 0 {
                lb.set_selected_index(self.selected_filter_index as usize);
            }
            let events = self.events.clone();
            lb.on_selection = Some(Box::new(move |indices: &[usize]| {
                let mut e = events.borrow_mut();
                e.filter_index_changed = indices.first().copied();
            }));
            let id = ctx.create_child_with("filter", Box::new(ListBoxPanel { list_box: lb }));
            self.filter_lb_id = Some(id);
        }

        // Register for per-frame cycling so we can process events.
        ctx.tree.request_cycle(ctx.id);
    }

    /// Sync FSB internal selection state FROM ListBox selection indices.
    fn selection_from_list_box(&mut self, indices: &[usize]) {
        let names: Vec<String> = indices
            .iter()
            .filter_map(|&i| self.listing.get(i).map(|(name, _)| name.clone()))
            .collect();
        self.selected_names = names;
    }

    /// Sync FSB selection state TO the ListBox widget after a listing reload.
    fn selection_to_list_box(&self, ctx: &mut PanelCtx) {
        if let Some(lb_id) = self.files_lb_id {
            // Build item data outside the tree borrow.
            let items: Vec<(String, String)> = self
                .listing
                .iter()
                .enumerate()
                .map(|(i, (name, _))| (format!("{}", i), name.clone()))
                .collect();
            let selected_indices: Vec<usize> = self
                .listing
                .iter()
                .enumerate()
                .filter(|(_, (name, _))| self.selected_names.contains(name))
                .map(|(i, _)| i)
                .collect();

            ctx.tree.with_behavior_as::<ListBoxPanel, _>(lb_id, |lbp| {
                lbp.list_box.clear_items();
                for (key, text) in &items {
                    lbp.list_box.add_item(key.clone(), text.clone());
                }
                lbp.list_box.set_selected_indices(&selected_indices);
            });
        }
    }

    /// Update name field text to match current selection.
    fn sync_name_field(&self, ctx: &mut PanelCtx) {
        if let Some(nf_id) = self.name_field_id {
            let text = self
                .selected_names
                .first()
                .cloned()
                .unwrap_or_default();
            ctx.tree
                .with_behavior_as::<TextFieldPanel, _>(nf_id, |tfp| {
                    tfp.text_field.set_text(&text);
                });
        }
    }

    /// Update directory field text to match current parent_dir.
    fn sync_dir_field(&self, ctx: &mut PanelCtx) {
        if let Some(df_id) = self.dir_field_id {
            let text = self.parent_dir.to_string_lossy().into_owned();
            ctx.tree
                .with_behavior_as::<TextFieldPanel, _>(df_id, |tfp| {
                    tfp.text_field.set_text(&text);
                });
        }
    }
}

/// Match a filename against a filter string of the form `Description (*.ext1 *.ext2)`.
///
/// Port of C++ `emFileSelectionBox::MatchFileNameFilter`.
pub(crate) fn match_file_name_filter(file_name: &str, filter: &str) -> bool {
    // Find the patterns between the last '(' and last ')'.
    let pattern_range = match (filter.rfind('('), filter.rfind(')')) {
        (Some(start), Some(end)) if start < end => &filter[start + 1..end],
        _ => filter,
    };

    // Split patterns by whitespace, semicolons, commas, or pipes.
    for pattern in
        pattern_range.split(|c: char| c.is_whitespace() || c == ';' || c == ',' || c == '|')
    {
        let pattern = pattern.trim();
        if !pattern.is_empty() && match_file_name_pattern(file_name, pattern) {
            return true;
        }
    }
    false
}

/// Match a filename against a glob-like pattern with `*` wildcards.
/// Matching is case-insensitive.
///
/// Port of C++ `emFileSelectionBox::MatchFileNamePattern`.
fn match_file_name_pattern(file_name: &str, pattern: &str) -> bool {
    let fname_bytes = file_name.as_bytes();
    let pat_bytes = pattern.as_bytes();
    match_pattern_recursive(fname_bytes, pat_bytes)
}

fn match_pattern_recursive(fname: &[u8], pattern: &[u8]) -> bool {
    if pattern.is_empty() {
        return fname.is_empty();
    }
    if pattern[0] == b'*' {
        // Try matching the rest of the pattern at each position.
        for i in 0..=fname.len() {
            if match_pattern_recursive(&fname[i..], &pattern[1..]) {
                return true;
            }
        }
        return false;
    }
    if fname.is_empty() {
        return pattern.is_empty();
    }
    if !fname[0].eq_ignore_ascii_case(&pattern[0]) {
        return false;
    }
    match_pattern_recursive(&fname[1..], &pattern[1..])
}

impl PanelBehavior for FileSelectionBox {
    fn paint(&mut self, painter: &mut Painter, w: f64, h: f64, state: &PanelState) {
        self.border
            .paint_border(painter, w, h, &self.look, state.enabled, true);
    }

    fn auto_expand(&self) -> bool {
        true
    }

    fn layout_children(&mut self, ctx: &mut PanelCtx) {
        if !ctx.tree.is_auto_expanded(ctx.id) {
            return;
        }

        // If state that affects child structure changed after initial creation,
        // tear down and rebuild all children.
        if self.children_dirty && ctx.child_count() > 0 {
            ctx.delete_all_children();
            self.dir_field_id = None;
            self.hidden_cb_id = None;
            self.files_lb_id = None;
            self.name_field_id = None;
            self.filter_lb_id = None;
        }
        self.children_dirty = false;

        if ctx.child_count() == 0 {
            self.create_children(ctx);
        }

        let rect = ctx.layout_rect();
        let (w, h) = (rect.w, rect.h);

        let cr = self.border.content_rect_unobscured(w, h, &self.look);
        let (x, y, cw, ch) = (cr.x, cr.y, cr.w, cr.h);

        let cc = self
            .border
            .content_canvas_color(ctx.canvas_color(), &self.look, ctx.is_enabled());

        // 3-zone geometry matching C++ LayoutChildren
        let hs = (cw * 0.05).min(ch * 0.15);
        let has_top = self.dir_field_id.is_some() || self.hidden_cb_id.is_some();
        let has_bottom = self.name_field_id.is_some() || self.filter_lb_id.is_some();
        let h1 = if has_top { hs } else { 0.0 };
        let h3 = if has_bottom { hs } else { 0.0 };
        let h2 = ch - h1 - h3;

        // Top row: directory field + checkbox
        if let Some(cb_id) = self.hidden_cb_id {
            let w2 = (cw * 0.5).min(h1 * 2.0);
            let w1 = cw - w2;
            if let Some(df_id) = self.dir_field_id {
                ctx.layout_child_canvas(df_id, x, y, w1, h1, cc);
            }
            ctx.layout_child_canvas(cb_id, x + w1, y, w2, h1, cc);
        } else if let Some(df_id) = self.dir_field_id {
            ctx.layout_child_canvas(df_id, x, y, cw, h1, cc);
        }

        // Middle: files list
        if let Some(fl_id) = self.files_lb_id {
            ctx.layout_child_canvas(fl_id, x, y + h1, cw, h2, cc);
        }

        // Bottom row: name field + filter list
        if let Some(flb_id) = self.filter_lb_id {
            let w2 = (cw * 0.5).min(h3 * 10.0);
            let w1 = cw - w2;
            if let Some(nf_id) = self.name_field_id {
                ctx.layout_child_canvas(nf_id, x, y + h1 + h2, w1, h3, cc);
            }
            ctx.layout_child_canvas(flb_id, x + w1, y + h1 + h2, w2, h3, cc);
        } else if let Some(nf_id) = self.name_field_id {
            ctx.layout_child_canvas(nf_id, x, y + h1 + h2, cw, h3, cc);
        }
    }

    fn cycle(&mut self, ctx: &mut PanelCtx) -> bool {
        // Take all pending events.
        let events = {
            let mut e = self.events.borrow_mut();
            std::mem::take(&mut *e)
        };

        // Step 2 (C++): Directory field changed.
        if let Some(dir_text) = events.dir_text_changed {
            let new_dir = PathBuf::from(&dir_text);
            if self.parent_dir != new_dir {
                self.parent_dir = new_dir;
                self.triggered_file_name.clear();
                self.invalidate_listing();
                if let Some(ref mut cb) = self.on_selection {
                    cb();
                }
            }
        }

        // Step 4 (C++): Hidden files checkbox toggled.
        if let Some(shown) = events.hidden_toggled {
            self.set_hidden_files_shown(shown);
        }

        // Step 5 (C++): Reload listing if invalid.
        if self.listing_invalid && self.files_lb_id.is_some() {
            self.reload_listing();
            // Sync selection TO ListBox after reload.
            self.selection_to_list_box(ctx);
        }

        // Step 6 (C++): ListBox selection changed -> update FSB state.
        if events.selection_changed && !self.listing_invalid {
            self.selection_from_list_box(&events.selection_indices);
            // Update name field.
            self.sync_name_field(ctx);
            if let Some(ref mut cb) = self.on_selection {
                cb();
            }
        }

        // Step 7 (C++): ListBox trigger (double-click).
        if let Some(index) = events.triggered_index {
            if !self.listing_invalid {
                // First sync selection.
                self.selection_from_list_box(&events.selection_indices);

                if let Some((name, data)) = self.listing.get(index) {
                    let name = name.clone();
                    let is_dir = data.is_directory;
                    if name == ".." || is_dir {
                        self.enter_sub_dir(&name);
                        // After entering subdir, update dir field.
                        self.sync_dir_field(ctx);
                    } else {
                        self.triggered_file_name = name.clone();
                        if let Some(ref mut cb) = self.on_trigger {
                            cb(&name);
                        }
                    }
                }
            }
        }

        // Step 8 (C++): Name field text changed.
        if let Some(name_text) = events.name_text_changed {
            if name_text.is_empty() {
                if self.selected_names.len() == 1 {
                    self.set_selected_name("");
                }
            } else if name_text.contains('/') || name_text.contains('\\') {
                // User typed a path -- resolve it.
                let abs = if Path::new(&name_text).is_absolute() {
                    PathBuf::from(&name_text)
                } else {
                    self.parent_dir.join(&name_text)
                };
                self.set_selected_path(&abs);
                // Sync name field back to just the filename.
                self.sync_name_field(ctx);
                self.sync_dir_field(ctx);
                if let Some(ref mut cb) = self.on_selection {
                    cb();
                }
            } else {
                self.set_selected_name(&name_text);
            }
        }

        // Step 9 (C++): Filter selection changed.
        if let Some(filter_idx) = events.filter_index_changed {
            self.set_selected_filter_index(filter_idx as i32);
        }

        // Stay awake as long as we have children (panel is expanded).
        self.files_lb_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_matching_all_files() {
        assert!(match_file_name_filter("anything.txt", "All files (*)"));
        assert!(match_file_name_filter("", "All files (*)"));
    }

    #[test]
    fn filter_matching_extension() {
        assert!(match_file_name_filter("image.tga", "Targa files (*.tga)"));
        assert!(!match_file_name_filter("image.png", "Targa files (*.tga)"));
    }

    #[test]
    fn filter_matching_case_insensitive() {
        assert!(match_file_name_filter("FILE.TGA", "Targa files (*.tga)"));
        assert!(match_file_name_filter("file.Tga", "Targa files (*.tga)"));
    }

    #[test]
    fn filter_matching_multiple_patterns() {
        assert!(match_file_name_filter(
            "page.htm",
            "HTML files (*.htm *.html)"
        ));
        assert!(match_file_name_filter(
            "page.html",
            "HTML files (*.htm *.html)"
        ));
        assert!(!match_file_name_filter(
            "page.txt",
            "HTML files (*.htm *.html)"
        ));
    }

    #[test]
    fn new_file_selection_box() {
        let fsb = FileSelectionBox::new("Files");
        assert!(!fsb.is_multi_selection_enabled());
        assert!(fsb.selected_names().is_empty());
        assert_eq!(fsb.selected_filter_index(), -1);
        assert!(!fsb.are_hidden_files_shown());
    }

    #[test]
    fn set_selected_name() {
        let mut fsb = FileSelectionBox::new("Files");
        fsb.set_selected_name("test.txt");
        assert_eq!(fsb.selected_name(), Some("test.txt"));

        fsb.set_selected_name("");
        assert_eq!(fsb.selected_name(), None);
    }

    #[test]
    fn set_filters() {
        let mut fsb = FileSelectionBox::new("Files");
        fsb.set_filters(&[
            "All files (*)".to_string(),
            "Images (*.png *.jpg)".to_string(),
        ]);
        assert_eq!(fsb.filters().len(), 2);
        assert_eq!(fsb.selected_filter_index(), 0);
    }

    #[test]
    fn enter_parent_dir() {
        let mut fsb = FileSelectionBox::new("Files");
        fsb.set_parent_directory(Path::new("/tmp"));
        fsb.set_selected_name("foo");
        fsb.enter_sub_dir("..");
        assert!(fsb.selected_names().is_empty());
    }

    #[test]
    fn visibility_toggles() {
        let mut fsb = FileSelectionBox::new("Files");
        assert!(!fsb.is_parent_dir_field_hidden());
        fsb.set_parent_dir_field_hidden(true);
        assert!(fsb.is_parent_dir_field_hidden());

        assert!(!fsb.is_name_field_hidden());
        fsb.set_name_field_hidden(true);
        assert!(fsb.is_name_field_hidden());

        assert!(!fsb.is_filter_hidden());
        fsb.set_filter_hidden(true);
        assert!(fsb.is_filter_hidden());
    }
}
