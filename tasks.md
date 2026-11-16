# Tasks

Keep this file lean, so that it reads at a glance as the queue of open work.
Usually prune a finished task rather than marking it `[x]`, and keep each entry
to a sentence or two. Record any context worth keeping somewhere more durable,
such as a commit message or a code comment.

Status key: `[ ]` not started · `[~]` in progress · `[x]` done · `[-]` dropped

- [ ] **Decide whether to build puzzles for the newly qualifying quotes** —
      the csv parser changed some quotes' lengths, so 26 numbers now pass
      `add_quote`'s length limits that didn't before.
- [ ] **Handle backslashes in the letters stage** — `quote_to_cells` panics
      on `\`, which the csv parser keeps as written. None of the 14 quotes
      holding one passes `add_quote`'s length limits yet.
- [ ] **Read records with an empty field mid-record** — see the TODO in
      `Quote::from_record`.
- [ ] **Create `build/` when building the quote list** — `global quotes`
      writes `build/quotes.json` without creating `build/`, so it fails on a
      fresh checkout.
- [ ] **Skip quotes another puzzle already uses** — some quotes appear in
      several records, so two puzzle numbers can yield the same puzzle, as
      415441 and 417219 do.
- [ ] **Count letters, not bytes, in `add_quote`'s length limits** — the
      limits count spaces, punctuation, and every byte of a multi-byte
      character, but puzzle size depends only on letters.

## Backlog

### Known data issues {#known-data-issues}

- [ ] **Quote 296448's source may end in a topic** — the source reads
      `Jen Wilde, queens of geek`, but ` queens of geek` may be a topic
      written without hyphens and with a leading space.
- [ ] **Split words joined with no punctuation between them** — some
      quotes lost a space with nothing left to mark it, as in `theyhad`, so
      `quote_to_cells` can't tell they're two words. A language model split
      all 27 examples it was given in a test. Once the spaces are in a
      committed puzzle's quote, `puzzle rebuild-cells` carries them into
      its cells and clue indices.

### Tidy-ups

- [ ] **Delete dead and commented-out code** — `#![expect(dead_code)]` in
      `generator/src/lib.rs` covers the items nothing uses; drop it once none
      remain.
- [ ] **Replace `.unwrap()` with `.expect()`** — give each call a message
      stating the invariant that should hold.
- [ ] **Use inline named format arguments** — replace positional
      placeholders, as in `add_quote`'s `format!("bad quote {} {}", …)`, with
      names such as `{elapsed:?}`, binding an expression to a descriptive
      local first.
- [ ] **Merge `restore_vec` and `mmap_bytes`** — the two near-copies panic
      when they can't open or map their file, though both return
      `io::Result`; the merged function could return those errors instead.
