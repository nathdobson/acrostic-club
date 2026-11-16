// Tests for `quote_to_cells`: how a quote's text becomes the puzzle's cells.

use acrostic_generator::add_letters::quote_to_cells;

#[test]
fn test_quote_to_cells() {
    assert_eq!(quote_to_cells("０１２３４５６７８９"), "0123456789");
    assert_eq!(
        quote_to_cells("ＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲＳＴＵＶＷＸＹＺ"),
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    );
    assert_eq!(
        quote_to_cells("ａｂｃｄｅｆｇｈｉｊｋｌｍｎｏｐｑｒｓｔｕｖｗｘｙｚ"),
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    );
}
