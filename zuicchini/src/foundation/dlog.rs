//! Debug log toggle and macro.
//!
//! Port of C++ `emEnableDLog` / `emIsDLogEnabled` / `EM_DLOG`. A global
//! `AtomicBool` controls whether debug log output is enabled. The `dlog!`
//! macro checks the flag and outputs to stderr with a module prefix.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static DLOG_ENABLED: AtomicBool = AtomicBool::new(false);

/// Optional capture buffer for testing. When `Some`, dlog! appends here
/// instead of (in addition to) stderr.
static DLOG_CAPTURE: Mutex<Option<Vec<String>>> = Mutex::new(None);

/// Check whether debug logging is enabled.
pub fn is_dlog_enabled() -> bool {
    DLOG_ENABLED.load(Ordering::Relaxed)
}

/// Enable or disable debug logging.
pub fn set_dlog_enabled(enable: bool) {
    DLOG_ENABLED.store(enable, Ordering::Relaxed);
}

/// Start capturing dlog output into a buffer (for testing).
pub fn start_capture() {
    *DLOG_CAPTURE.lock().unwrap() = Some(Vec::new());
}

/// Stop capturing and return all captured lines.
pub fn stop_capture() -> Vec<String> {
    DLOG_CAPTURE
        .lock()
        .unwrap()
        .take()
        .unwrap_or_default()
}

/// Push a line to the capture buffer if active.
#[doc(hidden)]
pub fn _capture_line(line: &str) {
    if let Ok(mut guard) = DLOG_CAPTURE.lock() {
        if let Some(ref mut buf) = *guard {
            buf.push(line.to_string());
        }
    }
}

/// Debug log macro. Checks `is_dlog_enabled()` and outputs to stderr with
/// a module prefix derived from `module_path!()`.
///
/// Usage: `dlog!("message {}", value);`
#[macro_export]
macro_rules! dlog {
    ($($arg:tt)*) => {
        if $crate::foundation::is_dlog_enabled() {
            let path = module_path!();
            // Strip crate prefix for readability
            let short = path.strip_prefix("zuicchini::").unwrap_or(path);
            let msg = format!("[{}] {}", short, format_args!($($arg)*));
            eprintln!("{}", msg);
            $crate::foundation::dlog::_capture_line(&msg);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_dlog() {
        assert!(!is_dlog_enabled());
        set_dlog_enabled(true);
        assert!(is_dlog_enabled());
        set_dlog_enabled(false);
        assert!(!is_dlog_enabled());
    }

    #[test]
    fn dlog_macro_fires_when_enabled() {
        set_dlog_enabled(true);
        // Should output to stderr without panicking
        dlog!("test message: {}", 42);
        set_dlog_enabled(false);
        // Should be a no-op when disabled
        dlog!("this should not appear");
    }

    #[test]
    fn dlog_capture_stderr_output() {
        set_dlog_enabled(true);
        start_capture();

        dlog!("captured line {}", 1);
        dlog!("captured line {}", 2);

        let lines = stop_capture();
        set_dlog_enabled(false);

        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("captured line 1"));
        assert!(lines[1].contains("captured line 2"));
    }
}
