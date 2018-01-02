/*!

[![](https://docs.rs/source-map-mappings/badge.svg)](https://docs.rs/source-map-mappings/) [![](https://img.shields.io/crates/v/source-map-mappings.svg)](https://crates.io/crates/source-map-mappings) [![](https://img.shields.io/crates/d/source-map-mappings.png)](https://crates.io/crates/source-map-mappings) [![Build Status](https://travis-ci.org/fitzgen/source-map-mappings.png?branch=master)](https://travis-ci.org/fitzgen/source-map-mappings)

Parse the `"mappings"` string from a source map.

This is intended to be compiled to WebAssembly and eventually used from the
[`mozilla/source-map`][source-map] library. This is **not** a general purpose
source maps library.

[source-map]: https://github.com/mozilla/source-map

* [Documentation](#documentation)
* [License](#license)
* [Contributing](#contributing)

## Documentation

[📚 Documentation on `docs.rs` 📚][docs]

[docs]: https://docs.rs/source-map-mappings

## License

Licensed under either of

 * [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)

 * [MIT license](http://opensource.org/licenses/MIT)

at your option.

## Contributing

See
[CONTRIBUTING.md](https://github.com/fitzgen/source-map-mappings/blob/master/CONTRIBUTING.md)
for hacking.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

 */

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

extern crate vlq;

mod comparators;

use std::cmp;
use comparators::ComparatorFunction;
use std::slice;
use std::u32;

/// Errors that can occur during parsing.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
#[repr(u32)]
pub enum Error {
    // NB: 0 is reserved for OK.

    /// The mappings contained a negative line, column, source index, or name
    /// index.
    UnexpectedNegativeNumber = 1,

    /// The mappings contained a number larger than `u32::MAX`.
    UnexpectedlyBigNumber = 2,

    /// Reached EOF while in the middle of parsing a VLQ.
    VlqUnexpectedEof = 3,

    /// Encountered an invalid base 64 character while parsing a VLQ.
    VlqInvalidBase64 = 4,

    /// VLQ encountered a number that, when decoded, would not fit in
    /// an i64.
    VlqOverflow = 5,
}

impl From<vlq::Error> for Error {
    #[inline]
    fn from(e: vlq::Error) -> Error {
        match e {
            vlq::Error::UnexpectedEof => Error::VlqUnexpectedEof,
            vlq::Error::InvalidBase64(_) => Error::VlqInvalidBase64,
            vlq::Error::Overflow => Error::VlqOverflow,
        }
    }
}

/// When doing fuzzy searching, whether to slide the next larger or next smaller
/// mapping from the queried location.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
#[repr(u32)]
pub enum Bias {
    // XXX: make sure these values always match `mozilla/source-map`'s
    // `SourceMapConsumer.{GreatestLower,LeastUpper}Bound` values!

    /// Slide to the next smaller mapping.
    GreatestLowerBound = 1,

    /// Slide to the next larger mapping.
    LeastUpperBound = 2,
}

impl Default for Bias {
    #[inline]
    fn default() -> Bias {
        Bias::GreatestLowerBound
    }
}

/// A trait for defining a set of RAII types that can observe the start and end
/// of various operations and queries we perform in their constructors and
/// destructors.
///
/// This is also implemented for `()` as the "null observer" that doesn't
/// actually do anything.
pub trait Observer: Default {
    /// Observe the parsing of the `"mappings"` string.
    type ParseMappings: Default;

    /// Observe sorting parsed mappings by original location.
    type SortByOriginalLocation: Default;

    /// Observe sorting parsed mappings by generated location.
    type SortByGeneratedLocation: Default;

    /// Observe computing column spans.
    type ComputeColumnSpans: Default;

    /// Observe querying what the original location for some generated location
    /// is.
    type OriginalLocationFor: Default;

    /// Observe querying what the generated location for some original location
    /// is.
    type GeneratedLocationFor: Default;

    /// Observe querying what all generated locations for some original location
    /// is.
    type AllGeneratedLocationsFor: Default;
}

impl Observer for () {
    type ParseMappings = ();
    type SortByOriginalLocation = ();
    type SortByGeneratedLocation = ();
    type ComputeColumnSpans = ();
    type OriginalLocationFor = ();
    type GeneratedLocationFor = ();
    type AllGeneratedLocationsFor = ();
}

/// A parsed set of mappings that can be queried.
///
/// Constructed via `parse_mappings`.
#[derive(Debug)]
pub struct Mappings<O = ()> {
    by_generated: Vec<Mapping>,
    by_original: Option<Vec<Mapping>>,
    computed_column_spans: bool,
    observer: O,
}

impl<O: Observer> Mappings<O> {
    /// Get the full set of mappings, ordered by generated location.
    #[inline]
    pub fn by_generated_location(&self) -> &[Mapping] {
        &self.by_generated
    }

    /// Compute the last generated column of each mapping.
    ///
    /// After this method has been called, any mappings with
    /// `last_generated_column == None` means that the mapping spans to the end
    /// of the line.
    pub fn compute_column_spans(&mut self) {
        if self.computed_column_spans {
            return;
        }

        let _observer = O::ComputeColumnSpans::default();

        let mut by_generated = self.by_generated.iter_mut().peekable();
        while let Some(this_mapping) = by_generated.next() {
            if let Some(next_mapping) = by_generated.peek() {
                if this_mapping.generated_line == next_mapping.generated_line {
                    this_mapping.last_generated_column = Some(next_mapping.generated_column);
                }
            }
        }

        self.computed_column_spans = true;
    }

    /// Get the set of mappings that have original location information, ordered
    /// by original location.
    pub fn by_original_location(&mut self) -> &[Mapping] {
        if let Some(ref by_original) = self.by_original {
            return by_original;
        }

        self.compute_column_spans();

        let _observer = O::SortByOriginalLocation::default();
        let mut by_original: Vec<_> = self.by_generated
            .iter()
            .filter(|m| m.original.is_some())
            .cloned()
            .collect();
        by_original.sort_by(<comparators::ByOriginalLocation as ComparatorFunction<_>>::compare);
        self.by_original = Some(by_original);
        self.by_original.as_ref().unwrap()
    }

    /// Get the mapping closest to the given generated location, if any exists.
    pub fn original_location_for(
        &self,
        generated_line: u32,
        generated_column: u32,
        bias: Bias,
    ) -> Option<&Mapping> {
        let _observer = O::OriginalLocationFor::default();

        let by_generated = self.by_generated_location();

        let position = by_generated.binary_search_by(|m| {
            m.generated_line
                .cmp(&generated_line)
                .then(m.generated_column.cmp(&generated_column))
        });

        match position {
            Ok(idx) => Some(&by_generated[idx]),
            Err(idx) => match bias {
                Bias::LeastUpperBound => by_generated.get(idx),
                Bias::GreatestLowerBound => if idx == 0 {
                    None
                } else {
                    by_generated.get(idx - 1)
                },
            },
        }
    }

    /// Get the mapping closest to the given original location, if any exists.
    pub fn generated_location_for(
        &mut self,
        source: u32,
        original_line: u32,
        original_column: u32,
        bias: Bias,
    ) -> Option<&Mapping> {
        let _observer = O::GeneratedLocationFor::default();

        let by_original = self.by_original_location();

        let position = by_original.binary_search_by(|m| {
            let original = m.original.as_ref().unwrap();
            original
                .source
                .cmp(&source)
                .then(original.original_line.cmp(&original_line))
                .then(original.original_column.cmp(&original_column))
        });

        match position {
            Ok(idx) => Some(&by_original[idx]),
            Err(idx) => match bias {
                Bias::LeastUpperBound => by_original.get(idx),
                Bias::GreatestLowerBound => if idx == 0 {
                    None
                } else {
                    by_original.get(idx - 1)
                },
            },
        }
    }

    /// Get all mappings at the given original location.
    ///
    /// If `original_column` is `None`, get all mappings on the given source and
    /// original line regardless what columns they have. If `original_column` is
    /// `Some`, only return mappings for which all of source, original line, and
    /// original column match.
    pub fn all_generated_locations_for(
        &mut self,
        source: u32,
        original_line: u32,
        original_column: Option<u32>,
    ) -> AllGeneratedLocationsFor {
        let _observer = O::AllGeneratedLocationsFor::default();

        let query_column = original_column.unwrap_or(0);

        let by_original = self.by_original_location();

        let compare = |m: &Mapping| {
            let original: &OriginalLocation = m.original.as_ref().unwrap();
            original
                .source
                .cmp(&source)
                .then(original.original_line.cmp(&original_line))
                .then(original.original_column.cmp(&query_column))
        };

        let idx = by_original.binary_search_by(&compare);
        let mut idx = match idx {
            Ok(idx) | Err(idx) => idx,
        };

        // If there are multiple mappings for this original location, the binary
        // search gives no guarantees that this is the index for the first of
        // them, so back up to the first.
        while idx > 0 && compare(&by_original[idx - 1]) == cmp::Ordering::Equal {
            idx -= 1;
        }

        let (mappings, original_line, original_column) = if idx < by_original.len() {
            let orig = by_original[idx].original.as_ref().unwrap();
            let mappings = by_original[idx..].iter();

            // Fuzzy line matching only happens when we don't have a column.
            let original_line = if original_column.is_some() {
                original_line
            } else {
                orig.original_line
            };

            let original_column = if original_column.is_some() {
                Some(orig.original_column)
            } else {
                None
            };

            (mappings, original_line, original_column)
        } else {
            ([].iter(), original_line, original_column)
        };

        AllGeneratedLocationsFor {
            mappings,
            source,
            original_line,
            original_column,
        }
    }
}

impl<O: Default> Default for Mappings<O> {
    #[inline]
    fn default() -> Mappings<O> {
        Mappings {
            by_generated: vec![],
            by_original: None,
            computed_column_spans: false,
            observer: Default::default(),
        }
    }
}

/// An iterator returned by `Mappings::all_generated_locations_for`.
#[derive(Debug)]
pub struct AllGeneratedLocationsFor<'a> {
    mappings: slice::Iter<'a, Mapping>,
    source: u32,
    original_line: u32,
    original_column: Option<u32>,
}

impl<'a> Iterator for AllGeneratedLocationsFor<'a> {
    type Item = &'a Mapping;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.mappings.next() {
            None => None,
            Some(m) => {
                let m_orig = m.original.as_ref().unwrap();

                if m_orig.source != self.source || m_orig.original_line != self.original_line {
                    return None;
                }

                if let Some(original_column) = self.original_column {
                    if m_orig.original_column != original_column {
                        return None;
                    }
                }

                Some(m)
            }
        }
    }
}

/// A single bidirectional mapping.
///
/// Always contains generated location information.
///
/// Might contain original location information, and if so, might also have an
/// associated name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mapping {
    /// The generated line.
    pub generated_line: u32,

    /// The generated column.
    pub generated_column: u32,

    /// The end column of this mapping's generated location span.
    ///
    /// Before `Mappings::computed_column_spans` has been called, this is always
    /// `None`. After `Mappings::computed_column_spans` has been called, it
    /// either contains `Some` column at which the generated location ends
    /// (exclusive), or it contains `None` if it spans until the end of the
    /// generated line.
    pub last_generated_column: Option<u32>,

    /// The original location information, if any.
    pub original: Option<OriginalLocation>,
}

impl Default for Mapping {
    #[inline]
    fn default() -> Mapping {
        Mapping {
            generated_line: 0,
            generated_column: 0,
            last_generated_column: None,
            original: None,
        }
    }
}

/// Original location information within a mapping.
///
/// Contains a source filename, an original line, and an original column. Might
/// also contain an associated name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalLocation {
    /// The source filename.
    pub source: u32,

    /// The original line.
    pub original_line: u32,

    /// The original column.
    pub original_column: u32,

    /// The associated name, if any.
    pub name: Option<u32>,
}

#[inline]
fn is_mapping_separator(byte: u8) -> bool {
    byte == b';' || byte == b','
}

#[inline]
fn read_relative_vlq<B>(previous: &mut u32, input: &mut B) -> Result<(), Error>
where
    B: Iterator<Item = u8>,
{
    let decoded = vlq::decode(input)?;
    let (new, overflowed) = (*previous as i64).overflowing_add(decoded);
    if overflowed || new > (u32::MAX as i64) {
        return Err(Error::UnexpectedlyBigNumber);
    }

    if new < 0 {
        return Err(Error::UnexpectedNegativeNumber);
    }

    *previous = new as u32;
    Ok(())
}

/// Parse a source map's `"mappings"` string into a queryable `Mappings`
/// structure.
pub fn parse_mappings<O: Observer>(input: &[u8]) -> Result<Mappings<O>, Error> {
    let _observer = O::ParseMappings::default();

    let mut generated_line = 0;
    let mut generated_column = 0;
    let mut original_line = 0;
    let mut original_column = 0;
    let mut source = 0;
    let mut name = 0;

    let mut mappings = Mappings::default();

    // `input.len() / 2` is the upper bound on how many mappings the string
    // might contain. There would be some sequence like `A,A,A,...` or
    // `A;A;A;...`.
    let mut by_generated = Vec::with_capacity(input.len() / 2);

    let mut input = input.iter().cloned().peekable();

    while let Some(byte) = input.peek().cloned() {
        match byte {
            b';' => {
                generated_line += 1;
                generated_column = 0;
                input.next().unwrap();
            }
            b',' => {
                input.next().unwrap();
            }
            _ => {
                let mut mapping = Mapping::default();
                mapping.generated_line = generated_line;

                // First is a generated column that is always present.
                read_relative_vlq(&mut generated_column, &mut input)?;
                mapping.generated_column = generated_column as u32;

                // Read source, original line, and original column if the
                // mapping has them.
                mapping.original = if input.peek().cloned().map_or(true, is_mapping_separator) {
                    None
                } else {
                    read_relative_vlq(&mut source, &mut input)?;
                    read_relative_vlq(&mut original_line, &mut input)?;
                    read_relative_vlq(&mut original_column, &mut input)?;

                    Some(OriginalLocation {
                        source: source,
                        original_line: original_line,
                        original_column: original_column,
                        name: if input.peek().cloned().map_or(true, is_mapping_separator) {
                            None
                        } else {
                            read_relative_vlq(&mut name, &mut input)?;
                            Some(name)
                        },
                    })
                };

                by_generated.push(mapping);
            }
        }
    }

    let _observer = O::SortByGeneratedLocation::default();
    by_generated.sort_by(comparators::ByGeneratedLocation::compare);
    mappings.by_generated = by_generated;
    Ok(mappings)
}
