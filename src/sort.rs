//! Custom quick sort implementation that is fast for sorting mappings.

use comparators::ComparatorFunction;
use rand::{self, Rng};
use std::cmp::{self, Ordering};
use std::mem;

/// Swap the elements in `slice` at indices `x` and `y`.
///
/// For whatever reason, this ends up being an order of magnitude faster than
/// using the builtin `slice::swap` -- maybe because we can't force that to be
/// inlined?
#[inline(always)]
fn swap<T>(slice: &mut [T], x: usize, y: usize) {
    debug_assert!(x < slice.len(), "(x = {}) < (slice.len() = {})", x, slice.len());
    debug_assert!(y < slice.len(), "(y = {}) < (slice.len() = {})", y, slice.len());

    if x == y {
        return;
    }

    let (x, y) = (cmp::min(x, y), cmp::max(x, y));
    let (low, high) = slice.split_at_mut(y);

    debug_assert!(x < low.len());
    debug_assert!(0 < high.len());

    unsafe {
        mem::swap(low.get_unchecked_mut(x), high.get_unchecked_mut(0));
    }
}

/// Partition the `slice[p..r]` about some pivot element in that range, and
/// return the index of the pivot.
#[inline(always)]
fn partition<R, F, T>(rng: &mut R, slice: &mut [T], p: usize, r: usize) -> usize
where
    R: Rng,
    F: ComparatorFunction<T>
{
    let pivot = rng.gen_range(p, r + 1);
    swap(slice, pivot, r);

    let mut i = (p as isize) - 1;

    for j in p..r {
        if let Ordering::Greater = unsafe {
            debug_assert!(j < slice.len());
            debug_assert!(r < slice.len());
            F::compare(slice.get_unchecked(j), slice.get_unchecked(r))
        } {
            continue;
        }

        i += 1;
        swap(slice, i as usize, j);
    }

    swap(slice, (i + 1) as usize, r);
    return (i + 1) as usize;
}

/// Recursive quick sort implementation with all the extra parameters that we
/// want to hide from callers to give them better ergonomics.
fn do_quick_sort<R, F, T>(rng: &mut R, slice: &mut [T], p: usize, r: usize)
where
    R: Rng,
    F: ComparatorFunction<T>
{
    if p < r {
        let q = partition::<R, F, T>(rng, slice, p, r);
        do_quick_sort::<R, F, T>(rng, slice, p, q.saturating_sub(1));
        do_quick_sort::<R, F, T>(rng, slice, q + 1, r);
    }
}

/// Do a quick sort on the given slice.
pub fn quick_sort<F, T>(slice: &mut [T])
where
    F: ComparatorFunction<T>
{
    if slice.is_empty() {
        return;
    }

    let mut rng = rand::XorShiftRng::new_unseeded();
    let len = slice.len();
    do_quick_sort::<_, F, T>(&mut rng, slice, 0, len - 1);
}
