# acrostic-club
Acrostic generator and player

## Testing

`cargo test` runs the fast tests, which need nothing outside the repository.
The other tests are slow or need local resources, such as the generator's
outputs in `build/` or a running language model. Each of them carries an
`#[ignore]` attribute stating its reason, so `cargo test` skips it unless asked:

```bash
# Run only the ignored tests.
cargo test -- --ignored

# Run every test.
cargo test -- --include-ignored
```

To run one ignored test, name its target and add the flag, as in
`cargo test --test ontology -- --ignored`.
