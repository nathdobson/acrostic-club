// Tests for `save_vec` and `restore_vec`. The test below is commented out because it doesn't
// compile: it calls both functions synchronously, but they are async.

// #[test]
// fn test_mmap_allocator() {
//     let file = "/tmp/test_mmap_allocator";
//     let value = vec![1u32, 2, 3].into_boxed_slice();
//     save_vec(Path::new(file), &value);
//     assert_eq!(*value, *restore_vec::<u32>(Path::new(file)));
// }
