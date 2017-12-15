#[macro_use]
extern crate quickcheck;
extern crate source_map_mappings;
extern crate vlq;

use quickcheck::{Arbitrary, Gen};
use std::fmt;
use std::i64;
use std::marker::PhantomData;

trait VlqRange: 'static + Send + Copy + Clone + fmt::Debug + fmt::Display {
    fn low() -> i64;
    fn high() -> i64;
}


#[derive(Copy, Clone, Debug)]
struct Vlq<R>(i64, PhantomData<R>);

impl<R> Arbitrary for Vlq<R>
where
    R: VlqRange
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Vlq(g.gen_range(R::low(), R::high()), PhantomData)
    }
}

impl<R> fmt::Display for Vlq<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut v = vec![];
        vlq::encode(self.0, &mut v).unwrap();
        write!(f, "{}", String::from_utf8_lossy(&v))
    }
}

#[derive(Clone, Debug)]
enum Mapping<R> {
    Generated {
        generated_column: Vlq<R>,
    },
    Original {
        generated_column: Vlq<R>,
        source: Vlq<R>,
        original_line: Vlq<R>,
        original_column: Vlq<R>,
    },
    OriginalWithName {
        generated_column: Vlq<R>,
        source: Vlq<R>,
        original_line: Vlq<R>,
        original_column: Vlq<R>,
        name: Vlq<R>,
    }
}

impl<R> Arbitrary for Mapping<R>
where
    R: VlqRange
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        match g.gen_range(0, 3) {
            0 => Mapping::Generated {
                generated_column: Vlq::<R>::arbitrary(g)
            },
            1 => Mapping::Original {
                generated_column: Vlq::<R>::arbitrary(g),
                source: Vlq::<R>::arbitrary(g),
                original_line: Vlq::<R>::arbitrary(g),
                original_column: Vlq::<R>::arbitrary(g),
            },
            2 => Mapping::OriginalWithName {
                generated_column: Vlq::<R>::arbitrary(g),
                source: Vlq::<R>::arbitrary(g),
                original_line: Vlq::<R>::arbitrary(g),
                original_column: Vlq::<R>::arbitrary(g),
                name: Vlq::<R>::arbitrary(g),
            },
            _ => unreachable!(),
        }
    }
}

impl<R: Copy> fmt::Display for Mapping<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Mapping::Generated { generated_column } => generated_column.fmt(f),
            Mapping::Original {
                generated_column,
                source,
                original_line,
                original_column,
            } => {
                generated_column.fmt(f)?;
                source.fmt(f)?;
                original_line.fmt(f)?;
                original_column.fmt(f)
            }
            Mapping::OriginalWithName {
                generated_column,
                source,
                original_line,
                original_column,
                name,
            } => {
                generated_column.fmt(f)?;
                source.fmt(f)?;
                original_line.fmt(f)?;
                original_column.fmt(f)?;
                name.fmt(f)
            }
        }
    }
}

#[derive(Clone, Debug)]
struct GeneratedLine<R>(Vec<Mapping<R>>);

impl<R> Arbitrary for GeneratedLine<R>
where
    R: VlqRange
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        GeneratedLine(Vec::arbitrary(g))
    }
}

impl<R: Copy> fmt::Display for GeneratedLine<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut needs_comma = false;
        for m in &self.0 {
            if needs_comma {
                write!(f, ",")?;
            }
            m.fmt(f)?;
            needs_comma = true;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct Mappings<R>(Vec<GeneratedLine<R>>);

impl<R> Arbitrary for Mappings<R>
where
    R: VlqRange
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Mappings(Vec::arbitrary(g))
    }
}

impl<R: Copy> fmt::Display for Mappings<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut needs_semi = false;
        for line in &self.0 {
            if needs_semi {
                write!(f, ";")?;
            }
            line.fmt(f)?;
            needs_semi = true;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
struct FullRange;

impl fmt::Display for FullRange {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

impl VlqRange for FullRange {
    fn low() -> i64 { i64::MIN }
    fn high() -> i64 { i64::MAX }
}

#[derive(Copy, Clone, Debug)]
struct SmallPositives;

impl fmt::Display for SmallPositives {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

impl VlqRange for SmallPositives {
    fn low() -> i64 { 0 }
    fn high() -> i64 { 5 }
}


quickcheck! {
    fn parse_without_panicking(mappings: Mappings<FullRange>) -> () {
        let mappings_string = mappings.to_string();
        let _ = source_map_mappings::parse_mappings(mappings_string.as_bytes());
    }

    fn parse_valid_mappings(mappings: Mappings<SmallPositives>) -> Result<(), source_map_mappings::Error> {
        let mappings_string = mappings.to_string();
        source_map_mappings::parse_mappings(mappings_string.as_bytes())?;
        Ok(())
    }
}
