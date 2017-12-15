#![deny(missing_debug_implementations)]

extern crate vlq;

mod comparators;

use std::marker::PhantomData;
use std::mem;
use std::u32;

#[derive(Debug)]
pub enum Error {
    UnexpectedNegativeNumber,
    UnexpectedlyBigNumber,
    Vlq(vlq::Error),
}

impl From<vlq::Error> for Error {
    fn from(e: vlq::Error) -> Error {
        Error::Vlq(e)
    }
}

#[derive(Debug)]
enum LazilySorted<T, F> {
    Sorted(Vec<T>, PhantomData<F>),
    Unsorted(Vec<T>),
}

impl<T, F> LazilySorted<T, F>
where
    F: comparators::ComparatorFunction<T>,
{
    fn sort(&mut self) {
        let me = mem::replace(self, LazilySorted::Unsorted(vec![]));
        let items = match me {
            LazilySorted::Sorted(items, _) => items,
            LazilySorted::Unsorted(mut items) => {
                items.sort_unstable_by(F::compare);
                items
            }
        };
        mem::replace(self, LazilySorted::Sorted(items, PhantomData));
    }
}

#[derive(Debug)]
pub struct Mappings {
    by_generated: LazilySorted<Mapping, comparators::ByGeneratedLocation>,
    by_original: LazilySorted<Mapping, comparators::ByOriginalLocation>,
}

impl Mappings {
    pub fn by_generated_location(&mut self) -> &[Mapping] {
        self.by_generated.sort();
        match self.by_generated {
            LazilySorted::Sorted(ref items, _) => items,
            LazilySorted::Unsorted(_) => unreachable!(),
        }
    }

    pub fn by_original_location(&mut self) -> &[Mapping] {
        self.by_original.sort();
        match self.by_original {
            LazilySorted::Sorted(ref items, _) => items,
            LazilySorted::Unsorted(_) => unreachable!(),
        }
    }
}

impl Default for Mappings {
    fn default() -> Mappings {
        Mappings {
            by_generated: LazilySorted::Unsorted(vec![]),
            by_original: LazilySorted::Unsorted(vec![]),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Mapping {
    generated_line: u32,
    generated_column: u32,
    original: Option<OriginalLocation>,
}

impl Default for Mapping {
    fn default() -> Mapping {
        Mapping {
            generated_line: 0,
            generated_column: 0,
            original: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OriginalLocation {
    source: u32,
    original_line: u32,
    original_column: u32,
    name: Option<u32>,
}

#[inline]
fn is_mapping_separator(byte: u8) -> bool {
    byte == b';' || byte == b','
}

#[inline]
fn read_relative_positive_vlq<B>(previous: &mut u32, input: &mut B) -> Result<(), Error>
where
    B: Iterator<Item = u8>,
{
    let decoded = vlq::decode(input)?;
    let (new, overflowed) = (*previous as i64).overflowing_add(decoded);
    if overflowed || new > (u32::MAX as i64) {
        return Err(Error::UnexpectedlyBigNumber);
    }

    if new < 0 {
        return Err(Error::UnexpectedNegativeNumber)
    }

    *previous = new as u32;
    Ok(())
}

pub fn parse_mappings(input: &[u8]) -> Result<Mappings, Error> {
    let mut generated_line = 0;
    let mut generated_column = 0;
    let mut original_line = 0;
    let mut original_column = 0;
    let mut source = 0;
    let mut name = 0;

    let mut mappings = Mappings::default();
    let mut by_generated = vec![];
    let mut by_original = vec![];

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
                read_relative_positive_vlq(&mut generated_column, &mut input)?;
                mapping.generated_column = generated_column as u32;

                // Read source, original line, and original column if the
                // mapping has them.
                mapping.original = if input.peek().cloned().map_or(true, is_mapping_separator) {
                    None
                } else {
                    read_relative_positive_vlq(&mut source, &mut input)?;
                    read_relative_positive_vlq(&mut original_line, &mut input)?;
                    read_relative_positive_vlq(&mut original_column, &mut input)?;

                    Some(OriginalLocation {
                        source: source,
                        original_line: original_line,
                        original_column: original_column,
                        name: if input.peek().cloned().map_or(true, is_mapping_separator) {
                            None
                        } else {
                            read_relative_positive_vlq(&mut name, &mut input)?;
                            Some(name)
                        },
                    })
                };

                if mapping.original.is_some() {
                    by_original.push(mapping.clone());
                }
                by_generated.push(mapping);
            }
        }
    }

    mappings.by_original = LazilySorted::Unsorted(by_original);
    mappings.by_generated = LazilySorted::Unsorted(by_generated);
    Ok(mappings)
}
