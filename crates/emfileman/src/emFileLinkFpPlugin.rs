use std::rc::Rc;

use emcore::emFpPlugin::{emFpPlugin, PanelParentArg};
use emcore::emPanel::PanelBehavior;

use crate::emFileLinkPanel::emFileLinkPanel;

/// Entry point for the file link panel plugin.
/// Loaded via `emFileLink.emFpPlugin` config file.
#[no_mangle]
pub fn emFileLinkFpPluginFunc(
    parent: &PanelParentArg,
    _name: &str,
    _path: &str,
    plugin: &emFpPlugin,
    error_buf: &mut String,
) -> Option<Box<dyn PanelBehavior>> {
    if !plugin.properties.is_empty() {
        *error_buf = "emFileLinkFpPlugin: No properties allowed.".to_string();
        return None;
    }
    // Border depends on parent panel type — for now default to true
    // (correct determination requires checking parent's PanelBehavior type)
    Some(Box::new(emFileLinkPanel::new(
        Rc::clone(parent.root_context()),
        true,
    )))
}
