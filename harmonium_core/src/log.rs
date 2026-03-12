pub fn info(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&msg.into());
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Disable logs if HARMONIUM_CLI env var is set
        if std::env::var("HARMONIUM_CLI").is_err() {
            eprintln!("{msg}");
        }
    }
}

pub fn warn(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::warn_1(&msg.into());
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Disable logs if HARMONIUM_CLI env var is set
        if std::env::var("HARMONIUM_CLI").is_err() {
            eprintln!("WARN: {msg}");
        }
    }
}

pub fn error(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::error_1(&msg.into());
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Always show errors even in CLI mode
        eprintln!("ERROR: {msg}");
    }
}
