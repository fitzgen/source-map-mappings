//! The public JS API to the library.
//!
//! Every exported function must be `#[no_mangle]` and `pub extern "C"`.

#[no_mangle]
pub extern "C" fn hello(x: u32) -> u32 {
    x
}
