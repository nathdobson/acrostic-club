# Tasks

Status key: `[ ]` not started · `[~]` in progress · `[x]` done · `[-]` dropped

- [ ] **Decide whether to build puzzles for the newly qualifying quotes** —
      the csv parser changed some quotes' lengths, so 26 numbers now pass
      `add_quote`'s length limits that didn't before.
- [ ] **Handle backslashes in the letters stage** — `quote_to_cells` panics
      on `\`, which the csv parser keeps as written. None of the 14 quotes
      holding one passes `add_quote`'s length limits yet.
- [ ] **Read records with an empty field mid-record** — see the TODO in
      `Quote::from_record`.
- [ ] **Give records without topics an empty topic list** — see the TODO in
      `Quote::from_record`.

## Backlog

### Known data issues {#known-data-issues}

- [ ] **Quote 296448's source may end in a topic** — the source reads
      `Jen Wilde, queens of geek`, but ` queens of geek` may be a topic
      written without hyphens and with a leading space.
