# Tasks

Keep this file lean, so that it reads at a glance as the queue of open work.
Usually prune a finished task rather than marking it `[x]`, and keep each entry
to a sentence or two. Record any context worth keeping somewhere more durable,
such as a commit message or a code comment.

Status key: `[ ]` not started · `[~]` in progress · `[x]` done · `[-]` dropped

- [ ] **Revisit the 15,000-word cap on the tries** — the word-pair trie
      likely held the cap down; weigh how many more puzzles a higher cap
      solves against more obscure answers and slower searches.
- [ ] **Skip exact duplicates in the tries** — 187 of the 15,000 words
      `build_tries` keeps repeat an earlier word's letter sequence, like
      `album's` and `américa`; skipping them before the cap frees their slots.
      Keep anagrams such as `aboard`, though `Search::get_words` turns each
      entry into the most frequent word with its letters, so `abroad` wins.
- [ ] **Decide how `randomize1` should treat six-letter answers** — it only
      swaps an answer for a shorter word, but the tries hold no word under
      six letters, so it never changes a six-letter answer. On 40
      sample puzzles, about three quarters of its searches found no word. Its
      `old.count() > 4` check suggests it expected five-letter words.

## Backlog

- [ ] **Delete the published puzzles outside the length limits** — wait
      until 1,000 or more published puzzles fit the limits, then delete
      `PUBLISHED_PUZZLES_OUTSIDE_LENGTH_LIMITS` in `quote.rs` along with its
      tests and the puzzles it lets through.

### Known data issues {#known-data-issues}

- [ ] **Split words joined with no punctuation between them** — some
      quotes lost a space with nothing left to mark it, as in `theyhad`, so
      `quote_to_cells` can't tell they're two words. A language model split
      all 27 examples it was given in a test. The splits belong in a pass of
      stage 0's `reformat_quote`; rerunning `puzzle quote` and then
      `puzzle rebuild-cells` carries them into a committed puzzle.
- [ ] **Prune clues that better clues outrank** — importing only adds, so the
      bank still holds clues that no puzzle names and no new puzzle would
      pick, such as the wrong "Daughter's son." for `nephew` and "Michele's
      husband on 'The Office'." for `dwight`.
- [ ] **Find fair clues for the answers whose clues are all flawed** —
      `atherton`, `bellator`, `bharatiya`, `cavendish`, `erebidae`, `filmfare`,
      `kannada`, `penrith`, `rowspan`, `sahitya`, and `vidhan`. Some may be
      too obscure to clue fairly and better kept out of puzzles instead.
- [ ] **Add better clues for the answers that still have one flat clue** —
      753 answers kept only their original clue, which a review found flat.
      Add new clues with `global import-clues`, then rerun `puzzle chat` on
      the clued puzzles so that they pick up the better ones.
- [ ] **Drop sentence-ending periods from the bank's older clues** — the
      imported clues lost theirs, but most clues from before the import still
      end in one. Keep a period that closes an abbreviation, as in "e.g.". An
      edit that changes a clue's text drops its review, which a change of
      punctuation alone needn't do.
- [ ] **Quote 223339 uses `‟` as an apostrophe** — in `mama‟s`, so its
      reformatted text and its cells split the word.

### Tidy-ups

- [ ] **Print each set under its own name in `ConflictSet`'s debug output** —
      `lexicals_down`, `writtens_down`, and `forms_down` each print
      `pages_down`.
- [ ] **Return errors from `mmap_bytes` instead of panicking** — it panics
      when it can't open or map its file, though it returns `io::Result`.
- [ ] **Split `cluebank.rs` into smaller modules** — it holds tag building,
      storage, editing, importing, and two command wrappers in one file.
      Giving tag building and importing modules of their own would let a
      change to one of them be made without reading the rest.

## Long-term goals

- [ ] **Make puzzle generation deterministic, reproducible, and hermetic** —
      puzzle numbers must stay fixed forever, since completion is tracked by
      number. Anything else about a puzzle, such as the words it solves to,
      may change through a deliberate breaking change, but never as a side
      effect of some other change.
