#[cfg(target_arch = "wasm32")]
pub fn info(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&msg.into());
    #[cfg(not(target_arch = "wasm32"))]
    println!("{}", msg);
}

pub fn warn(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::warn_1(&msg.into());
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("WARN: {}", msg);
}

pub fn error(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::error_1(&msg.into());
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("ERROR: {}", msg);
}
