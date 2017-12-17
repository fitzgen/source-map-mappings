//! The public JS API to the `source-map-mappings` library.
//!
//! Every exported function must be `#[no_mangle]` and `pub extern "C"`.

extern crate source_map_mappings;

use source_map_mappings::{Bias, Error, Mapping, Mappings};
use std::cell::Cell;
use std::mem;
use std::ptr;
use std::slice;

thread_local! {
    static LAST_ERROR: Cell<Option<Error>> = Cell::new(None);
}

#[no_mangle]
pub extern "C" fn get_last_error() -> u32 {
    match LAST_ERROR.with(|last_error| last_error.get()) {
        None => 0,
        Some(e) => e as u32,
    }
}

#[inline]
fn assert_pointer_is_word_aligned(p: *mut u8) {
    assert_eq!(p as usize & (mem::size_of::<usize>() - 1), 0);
}

// TODO: factor out allocation into its own wasm-allocator crate.

#[no_mangle]
pub extern "C" fn allocate_mappings(size: usize) -> *mut u8 {
    // Make sure that we don't lose any bytes from size in the remainder.
    let size_in_units_of_usize = (size + mem::size_of::<usize>() - 1) / mem::size_of::<usize>();

    // Make room for two additional `usize`s: we'll stuff capacity and length in
    // there.
    let mut vec: Vec<usize> = Vec::with_capacity(size_in_units_of_usize + 2);

    // And do the stuffing.
    let capacity = vec.capacity();
    vec.push(capacity);
    vec.push(size);

    // Leak the vec's elements and get a pointer to them.
    let ptr = vec.as_mut_ptr();
    assert!(!ptr.is_null());
    mem::forget(vec);

    // Advance the pointer past our stuffed data and return it to JS, so that JS
    // can write the mappings string into it.
    let ptr = ptr.wrapping_offset(2) as *mut u8;
    assert_pointer_is_word_aligned(ptr);
    ptr
}

#[inline]
fn constrain<'a, T>(_scope: &'a (), reference: &'a T) -> &'a T
where
    T: ?Sized
{
    reference
}

#[no_mangle]
pub extern "C" fn parse_mappings(mappings: *mut u8) -> *mut Mappings {
    assert_pointer_is_word_aligned(mappings);
    let mappings = mappings as *mut usize;

    // Unstuff the data we put just before the pointer to the mappings
    // string.
    let capacity_ptr = mappings.wrapping_offset(-2);
    assert!(!capacity_ptr.is_null());
    let capacity = unsafe { *capacity_ptr };

    let size_ptr = mappings.wrapping_offset(-1);
    assert!(!size_ptr.is_null());
    let size = unsafe { *size_ptr };

    // Construct the input slice from the pointer and parse the mappings.
    let result = unsafe {
        let input = slice::from_raw_parts(mappings as *const u8, size);
        let this_scope = ();
        let input = constrain(&this_scope, input);
        source_map_mappings::parse_mappings(input)
    };

    // Deallocate the mappings string and its two prefix words.
    unsafe {
        Vec::<usize>::from_raw_parts(capacity_ptr, size, capacity);
    }

    // Return the result, saving any errors on the side for later inspection by
    // JS if required.
    match result {
        Ok(mappings) => Box::into_raw(Box::new(mappings)),
        Err(e) => {
            LAST_ERROR.with(|last_error| last_error.set(Some(e)));
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn free_mappings(mappings: *mut Mappings) {
    unsafe {
        Box::from_raw(mappings);
    }
}

#[inline]
unsafe fn mappings_mut<'a>(_scope: &'a (), mappings: *mut Mappings) -> &'a mut Mappings {
    mappings.as_mut().unwrap()
}

#[inline]
fn mapping_to_parts(mapping: &Mapping) -> (u32, u32, bool, u32, bool, u32, u32, u32, bool, u32) {
    let generated_line = mapping.generated_line();
    let generated_column = mapping.generated_column();

    let (
        has_last_generated_column,
        last_generated_column,
    ) = if let Some(last_generated_column) = mapping.last_generated_column() {
        (true, last_generated_column)
    } else {
        (false, 0)
    };

    let (
        has_original,
        source,
        original_line,
        original_column,
        has_name,
        name,
    ) = if let Some(original) = mapping.original() {
        let (
            has_name,
            name,
        ) = if let Some(name) = original.name() {
            (true, name)
        } else {
            (false, 0)
        };

        (
            true,
            original.source(),
            original.original_line(),
            original.original_column(),
            has_name,
            name,
        )
    } else {
        (
            false,
            0,
            0,
            0,
            false,
            0,
        )
    };

    (
        generated_line,
        generated_column,
        has_last_generated_column,
        last_generated_column,
        has_original,
        source,
        original_line,
        original_column,
        has_name,
        name,
    )
}

extern "C" {
    fn mapping_callback(
        // These two parameters are always valid.
        generated_line: u32,
        generated_column: u32,

        // The `last_generated_column` parameter is only valid if
        // `has_last_generated_column` is `true`.
        has_last_generated_column: bool,
        last_generated_column: u32,

        // The `source`, `original_line`, and `original_column` parameters are
        // only valid if `has_original` is `true`.
        has_original: bool,
        source: u32,
        original_line: u32,
        original_column: u32,

        // The `name` parameter is only valid if `has_name` is `true`.
        has_name: bool,
        name: u32,
    );
}

#[inline]
unsafe fn invoke_mapping_callback(mapping: &Mapping) {
    let (
        generated_line,
        generated_column,
        has_last_generated_column,
        last_generated_column,
        has_original,
        source,
        original_line,
        original_column,
        has_name,
        name,
    ) = mapping_to_parts(mapping);

    mapping_callback(
        generated_line,
        generated_column,
        has_last_generated_column,
        last_generated_column,
        has_original,
        source,
        original_line,
        original_column,
        has_name,
        name,
    );
}

#[no_mangle]
pub extern "C" fn by_generated_location(mappings: *mut Mappings) {
    let this_scope = ();
    let mappings = unsafe { mappings_mut(&this_scope, mappings) };

    mappings.by_generated_location().iter().for_each(|m| unsafe {
        invoke_mapping_callback(m);
    });
}

#[no_mangle]
pub extern "C" fn compute_column_spans(mappings: *mut Mappings) {
    let this_scope = ();
    let mappings = unsafe { mappings_mut(&this_scope, mappings) };

    mappings.compute_column_spans();
}

#[no_mangle]
pub extern "C" fn by_original_location(mappings: *mut Mappings) {
    let this_scope = ();
    let mappings = unsafe { mappings_mut(&this_scope, mappings) };

    mappings.by_original_location().iter().for_each(|m| unsafe {
        invoke_mapping_callback(m);
    });
}

#[inline]
fn byte_to_bias(bias: u8) -> Bias {
    match bias {
        1 => Bias::GreatestLowerBound,
        2 => Bias::LeastUpperBound,
        otherwise => panic!(
            "Invalid `Bias = {}`; must be `Bias::GreatestLowerBound = {}` or \
             `Bias::LeastUpperBound = {}`",
            otherwise,
            Bias::GreatestLowerBound as u8,
            Bias::LeastUpperBound as u8,
        ),
    }
}

#[no_mangle]
pub extern "C" fn original_location_for(
    mappings: *mut Mappings,
    generated_line: u32,
    generated_column: u32,
    bias: u8,
) {
    let this_scope = ();
    let mappings = unsafe { mappings_mut(&this_scope, mappings) };
    let bias = byte_to_bias(bias);

    if let Some(m) = mappings.original_location_for(generated_line, generated_column, bias) {
        unsafe {
            invoke_mapping_callback(m);
        }
    }
}

#[no_mangle]
pub extern "C" fn generated_location_for(
    mappings: *mut Mappings,
    source: u32,
    original_line: u32,
    original_column: u32,
    bias: u8,
) {
    let this_scope = ();
    let mappings = unsafe { mappings_mut(&this_scope, mappings) };
    let bias = byte_to_bias(bias);

    if let Some(m) = mappings.generated_location_for(source, original_line, original_column, bias) {
        unsafe {
            invoke_mapping_callback(m);
        }
    }
}

#[no_mangle]
pub extern "C" fn all_generated_locations_for(
    mappings: *mut Mappings,
    source: u32,
    original_line: u32,
    has_original_column: bool,
    original_column: u32,
) {
    let this_scope = ();
    let mappings = unsafe { mappings_mut(&this_scope, mappings) };

    let original_column = if has_original_column {
        Some(original_column)
    } else {
        None
    };

    for m in mappings.all_generated_locations_for(source, original_line, original_column) {
        unsafe {
            invoke_mapping_callback(m);
        }
    }
}
