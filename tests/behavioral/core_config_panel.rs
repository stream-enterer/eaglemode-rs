use std::cell::RefCell;
use std::rc::Rc;

use eaglemode_rs::emCore::emConfigModel::emConfigModel;
use eaglemode_rs::emCore::emCoreConfig::emCoreConfig;
use eaglemode_rs::emCore::emCoreConfigPanel::emCoreConfigPanel;
use eaglemode_rs::emCore::emLook::emLook;

#[test]
fn smoke_new() {
    let config = Rc::new(RefCell::new(emConfigModel::new(
        emCoreConfig::default(),
        std::path::PathBuf::from("/tmp/test_core_config.rec"),
        slotmap::KeyData::from_ffi(u64::MAX).into(),
    )));
    let look = emLook::new();
    let _panel = emCoreConfigPanel::new(config, look);
}
