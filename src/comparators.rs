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

#[derive(Debug)]
pub struct ByGeneratedLocation;

impl ComparatorFunction<Mapping> for ByGeneratedLocation {
    #[inline]
    fn compare(a: &Mapping, b: &Mapping) -> Ordering {
        a.generated_line
            .cmp(&b.generated_line)
            .then(a.generated_column.cmp(&b.generated_column))
            .then_with(|| ByOriginalLocation::compare(&a.original, &b.original))
    }
}

#[derive(Debug)]
pub struct ByOriginalLocation;

impl ComparatorFunction<Mapping> for ByOriginalLocation {
    #[inline]
    fn compare(a: &Mapping, b: &Mapping) -> Ordering {
        ByOriginalLocation::compare(&a.original, &b.original).then(
            a.generated_line
                .cmp(&b.generated_line)
                .then(a.generated_column.cmp(&b.generated_column)),
        )
    }
}

impl ComparatorFunction<OriginalLocation> for ByOriginalLocation {
    #[inline]
    fn compare(a: &OriginalLocation, b: &OriginalLocation) -> Ordering {
        a.source
            .cmp(&b.source)
            .then(a.original_line.cmp(&b.original_line))
            .then(a.original_column.cmp(&b.original_column))
            .then(a.name.cmp(&b.name))
    }
}
