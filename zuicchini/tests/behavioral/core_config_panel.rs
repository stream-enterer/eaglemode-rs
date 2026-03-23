use std::cell::RefCell;
use std::rc::Rc;

use zuicchini::emCore::emConfigModel::ConfigModel;
use zuicchini::emCore::emCoreConfig::CoreConfig;
use zuicchini::emCore::emCoreConfigPanel::CoreConfigPanel;
use zuicchini::emCore::emLook::Look;

#[test]
fn smoke_new() {
    let config = Rc::new(RefCell::new(ConfigModel::new(
        CoreConfig::default(),
        std::path::PathBuf::from("/tmp/test_core_config.rec"),
        slotmap::KeyData::from_ffi(u64::MAX).into(),
    )));
    let look = Look::new();
    let _panel = CoreConfigPanel::new(config, look);
}
