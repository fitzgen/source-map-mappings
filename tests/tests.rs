extern crate source_map_mappings;

use source_map_mappings::parse_mappings;

#[test]
fn parse_empty_mappings() {
    let mut mappings = parse_mappings(&[]).expect("should parse OK");
    assert!(mappings.by_generated_location().is_empty());
    assert!(mappings.by_original_location().is_empty());
}

#[test]
fn invalid_mappings() {
    assert!(parse_mappings(b"...").is_err());
}
