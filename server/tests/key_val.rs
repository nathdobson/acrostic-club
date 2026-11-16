// Tests for `KeyVal`'s JSON form.

use acrostic_server::key_val::KeyVal;

#[test]
fn an_empty_key_val_serializes_as_an_empty_json_object() {
    let key_val = KeyVal::<usize, usize>::default();
    let json = serde_json::to_string(&key_val).expect("an empty map should serialize to JSON");
    assert_eq!(json, "{}");
}
