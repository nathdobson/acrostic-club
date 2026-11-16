// Tests for `GraphemeString`'s debug form.

use acrostic_generator::string::GraphemeString;

#[test]
fn test_string() {
    assert_eq!(
        format!("{:?}", GraphemeString::from_str("*straßé*")),
        "*STRA[SS/ß][E/é]*"
    );
}
