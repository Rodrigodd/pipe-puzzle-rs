
#[cfg(target_arch = "wasm32")]
include!("main.rs");

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;
    use std::panic;

    #[wasm_bindgen(start)]
    pub fn run() {
        panic::set_hook(Box::new(console_error_panic_hook::hook));
        super::main();
    }
}