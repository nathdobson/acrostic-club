// Tests for `KeyVal`'s JSON form.

use acrostic_server::key_val::KeyVal;

#[test]
fn test() {
    assert_eq!(
        serde_json::to_string(&KeyVal::<usize, usize>::default()).unwrap(),
        "{}"
    );
}
