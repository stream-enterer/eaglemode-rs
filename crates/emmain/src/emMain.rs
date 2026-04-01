// Port of C++ emMain (IPC server + window factory).
//
// DIVERGED: C++ emMain is an emEngine with IPC server polling via emMiniIpc.
// Rust uses a simplified struct that computes the server name for
// single-instance coordination.  IPC server/client is stubbed until
// emMiniIpc is ported.

/// Compute the IPC server name for single-instance coordination.
///
/// Port of C++ `emMain::CalcServerName`.
/// Derives a unique name from the hostname and DISPLAY environment variable.
pub fn CalcServerName() -> String {
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
    let hostname = get_hostname();
    format!(
        "eaglemode_on_{}_{}",
        hostname,
        display.replace([':', '.'], "_")
    )
}

/// Read the system hostname via /etc/hostname, falling back to "localhost".
fn get_hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "localhost".to_string())
}

/// Try to send a command to an already-running instance via IPC.
///
/// DIVERGED: C++ uses emMiniIpc::TrySend for single-instance coordination.
/// Stubbed until emMiniIpc is ported. Always returns false (no existing instance).
pub fn try_ipc_client(_server_name: &str, _visit: Option<&str>) -> bool {
    // TODO: wire emMiniIpc when available
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_server_name() {
        let name = CalcServerName();
        assert!(name.starts_with("eaglemode_on_"));
        // Should contain no colons or dots (they're replaced)
        let suffix = &name["eaglemode_on_".len()..];
        assert!(!suffix.contains(':'));
        assert!(!suffix.contains('.'));
    }

    #[test]
    fn test_get_hostname_non_empty() {
        let h = get_hostname();
        assert!(!h.is_empty());
    }

    #[test]
    fn test_try_ipc_client_stub() {
        assert!(!try_ipc_client("test_server", None));
        assert!(!try_ipc_client("test_server", Some("/home")));
    }
}
