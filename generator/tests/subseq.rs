// Tests for `longest_subsequence`.

use acrostic_generator::subseq::longest_subsequence;

#[test]
fn test_longest_subsequence() {
    assert_eq!(
        longest_subsequence(&[0, 1, 2, 3, 4, 5, 6], &[3, 3, 4, 6]),
        2
    );
}
