#![deny(warnings)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]
#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::indexing_slicing)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

//! `GerberView` WASM module — Gerber/Excellon parsing and geometry conversion.

use wasm_bindgen::prelude::*;

/// Smoke-test export. Returns 42.
#[allow(clippy::missing_const_for_fn)]
#[wasm_bindgen]
pub fn ping() -> u32 {
    42
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_returns_42() {
        assert_eq!(ping(), 42);
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn wasm_ping_returns_42() {
        assert_eq!(ping(), 42);
    }
}
