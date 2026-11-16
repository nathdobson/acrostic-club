// Tests for `FlatTrieTable`. The test below is commented out because it doesn't compile: it
// calls `FlatTrieTable::new` synchronously, but that constructor is private and async.

// #[test]
// fn test_flat_trie_table() {
//     let table = FlatTrieTable::new();
//     for x in table.unary.iter() {
//         println!("{:?}", x.0);
//         format!("{:?}", x.1);
//     }
//     for x in table.binary.iter() {
//         println!("{:?}", x.0);
//         format!("{:?}", x.1);
//     }
// }
