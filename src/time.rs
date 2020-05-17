cfg_if::cfg_if!{
    if #[cfg(target_arch = "wasm32")] {
        pub use wasm_timer::*;
    } else {
        pub use std::time::*;
    }
}
