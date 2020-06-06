#[cfg(target_arch = "wasm32")]
include!("main.rs");

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::panic;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn run() {
        panic::set_hook(Box::new(console_error_panic_hook::hook));
        super::main();
    }
}
