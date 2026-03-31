use emcore::emFpPlugin::emFpPluginFunc;

/// Static plugin function registry for statically linked plugins.
/// Falls back to this when dynamic symbol resolution fails.
pub fn resolve_static_plugin(function_name: &str) -> Option<emFpPluginFunc> {
    match function_name {
        "emDirFpPluginFunc" => Some(emFileMan::emDirFpPlugin::emDirFpPluginFunc),
        "emStocksFpPluginFunc" => Some(emStocks::emStocksFpPlugin::emStocksFpPluginFunc),
        "emDirStatFpPluginFunc" => Some(emFileMan::emDirStatFpPlugin::emDirStatFpPluginFunc),
        "emFileLinkFpPluginFunc" => Some(emFileMan::emFileLinkFpPlugin::emFileLinkFpPluginFunc),
        _ => None,
    }
}
