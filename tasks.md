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
- [ ] **Give records without topics an empty topic list** — see the TODO in
      `Quote::from_record`.

## Backlog

### Known data issues {#known-data-issues}

- [ ] **Quote 296448's source may end in a topic** — the source reads
      `Jen Wilde, queens of geek`, but ` queens of geek` may be a topic
      written without hyphens and with a leading space.
