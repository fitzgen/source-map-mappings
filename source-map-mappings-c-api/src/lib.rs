//! The public JS API to the `source-map-mappings` library.
//!
//! Every exported function must be `#[no_mangle]` and `pub extern "C"`.

extern crate source_map_mappings;

#[no_mangle]
pub extern "C" fn hello(x: u32) -> u32 {
    x
}
