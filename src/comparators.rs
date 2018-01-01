use super::{Mapping, OriginalLocation};
use std::cmp::Ordering;
use std::fmt;

pub trait ComparatorFunction<T>: fmt::Debug {
    fn compare(&T, &T) -> Ordering;
}

impl<T, F> ComparatorFunction<Option<T>> for F
where
    F: ComparatorFunction<T>,
{
    #[inline]
    fn compare(a: &Option<T>, b: &Option<T>) -> Ordering {
        match (a, b) {
            (&None, &None) => Ordering::Equal,
            (&Some(_), &None) => Ordering::Less,
            (&None, &Some(_)) => Ordering::Greater,
            (&Some(ref a), &Some(ref b)) => F::compare(a, b),
        }
    }
}

// Yes, using this style of comparison instead of `cmp.then(cmp2).then(cmp3)` is
// actually a big performance win in practice:
//
// ```
// $ cargo benchcmp control variable
// name                                     control ns/iter  variable ns/iter  diff ns/iter   diff %  speedup
// bench_parse_part_of_scala_js_source_map  2,029,981        1,290,716         -739,265       -36.42% x 1.57
// ```
//
// This doesn't seem right, but you can't argue with those numbers...
macro_rules! compare {
    ($a:expr, $b:expr) => {
        let cmp = ($a as i64) - ($b as i64);
        if cmp < 0 {
            return Ordering::Less;
        } else if cmp > 0 {
            return Ordering::Greater;
        }
    }
}

#[derive(Debug)]
pub struct ByGeneratedLocation;

impl ComparatorFunction<Mapping> for ByGeneratedLocation {
    #[inline]
    fn compare(a: &Mapping, b: &Mapping) -> Ordering {
        compare!(a.generated_line, b.generated_line);
        compare!(a.generated_column, b.generated_column);
        ByOriginalLocation::compare(&a.original, &b.original)
    }
}

#[derive(Debug)]
pub struct ByOriginalLocation;

impl ComparatorFunction<Mapping> for ByOriginalLocation {
    #[inline]
    fn compare(a: &Mapping, b: &Mapping) -> Ordering {
        let c = ByOriginalLocation::compare(&a.original, &b.original);
        match c {
            Ordering::Less | Ordering::Greater => c,
            Ordering::Equal => {
                compare!(a.generated_line, b.generated_line);
                compare!(a.generated_column, b.generated_column);
                Ordering::Equal
            }
        }
    }
}

impl ComparatorFunction<OriginalLocation> for ByOriginalLocation {
    #[inline]
    fn compare(a: &OriginalLocation, b: &OriginalLocation) -> Ordering {
        compare!(a.source, b.source);
        compare!(a.original_line, b.original_line);
        compare!(a.original_column, b.original_column);
        a.name.cmp(&b.name)
    }
}
