pub fn longest_subsequence<T: Eq>(a: &[T], b: &[T]) -> usize {
    let mut max = 0;
    for o1 in 0..a.len() {
        for o2 in 0..b.len() {
            let total = a[o1..].iter().zip(b[o2..].iter()).take_while(|(x, y)| x == y).count();
            if total > max {
                max = total;
            }
        }
    }
    max
}

#[test]
fn test_longest_subsequence() {
    assert_eq!(longest_subsequence(&[0, 1, 2, 3, 4, 5, 6], &[3, 3, 4, 6]), 2);
}