# Banned words in clue scoring: findings

These findings back the `tasks.md` entry **Use the lemma list during the
ontology walk**. Delete this file once that task is done.

## How clue scoring bans words

`ClueClient::score` rejects a candidate clue that gives its answer away. It
builds a list of banned words for the answer, then rejects the clue if the
clue's letters, with spaces and punctuation dropped, contain any banned word of
three or more letters. The banned words come from three places:

- **The answer itself.**
- **The lemma list** (`build/lemma.en.txt`): the answer's forms if it is a base
  word, such as "swimming" for "swim", or its base words if it is a form.
- **The ontology** (`Ontology::get_conflicts`): the answer's related words.

## What a related word is

A related word is any headword that `ConflictSet`'s walk reaches from the
answer through DBnary's English dictionary:

1. **Start:** every entry spelled like the answer, in any part of speech.
2. **Same page:** every entry on the same Wiktionary page.
3. **Shared etymology:** every entry the same etymology section describes, plus
   every entry described by an etymology one "etymologically related" link
   away.
4. **Derivation:** entries joined by DBnary's derivation links, with steps 2 to
   4 applied to each in turn, so the walk follows whole chains of derivations.

The median answer gets 10 related words, and one gets 5,178. That fan-out likely
explains junk such as "the" and "and" turning up as related words.

## Data and method

Measured in November 2026 on:

- the 4,627 clues committed in 225 puzzles, which passed older versions of the
  check, so they understate what slips through;
- the 240 raw candidate clues the model wrote for puzzle 88371, from the chat
  cache;
- the 4,221 distinct answers of the 481 puzzles that reached the answers stage.

A temporary test printed `get_conflicts` for every answer and for each base word
the lemma list gives, and Python scripts compared the banned lists and matching
rules.

## Word-form sources

| Source                        | Answers listed | Verb forms           |
| ----------------------------- | -------------: | -------------------- |
| Lemma list                    |          3,377 | Yes                  |
| English Speller Database      |          4,064 | Yes, irregular forms |
| DBnary's June 2026 word forms |          2,666 | 18 verbs in all      |

Banning each answer's whole word family from any one of these, or from all three
at once, catches no clue that today's check misses. The 97 answers no source
lists are names, foreign words, run-together compounds such as "threeday", and
junk such as "rowspan", none of which need word forms. So the lemma list
suffices. Should the English Speller Database replace it, the place to swap it
in is `Lemma` in `lemma.rs`, whose `alternates` and `canonicals` clue scoring
calls.

## Walking from base words

Also banning the related words of each base word that `canonicals` gives a
plural or past-tense answer newly rejects six committed clues, two of them
giveaways:

- **RETITLED**, "Changed title after initial release due to various reasons.":
  bans "title", a giveaway.
- **TWEETED**, "Published an update via Twitter.": bans "twitter", a giveaway.
- **APPLIED**, "This kind of mathematics often involves problem-solving.": bans
  "the", inside "mathematics".
- **ENCOMPASSED**, "Embraced or included.": bans "ace" and "embrace".
- **ENCOUNTERED**, "Met face to face with": bans "ace", inside "face".
- **SUITED**, "Attired properly for an occasion.": bans "cas", inside
  "occasion".

Banning the lemma list's forms of each related word instead rejects seven good
clues and no giveaway, such as POTATO's "Often fried or mashed." for "fried", so
that direction should stay out.

## Matching rules

Matching against the clue's words, split at spaces and punctuation but not at
hyphens or apostrophes, removes most of the junk:

| Rule                              | Lost today   | Base words then add      |
| --------------------------------- | ------------ | ------------------------ |
| Letters anywhere (today)          | None         | 2 giveaways, 4 junk      |
| Within one word                   | 2 junk       | 2 giveaways, 4 junk      |
| Start or end of a word            | 3 junk       | 2 giveaways, 2 junk      |
| Whole word or start of a word     | 3 junk       | 2 giveaways, 1 debatable |
| Whole word, or 4+ at start or end | 4, see below | 3 giveaways, 1 debatable |

"Lost today" counts clues today's rule rejects that the new rule lets through;
"base words then add" counts the clues that walking from base words then newly
rejects. The junk lost is "goback" across "go back" for ONE-WAY, "ingo" across
"beginning of" for OUTSET, and "and" inside "stands" for YORKER. The last rule
also loses a giveaway, "net" inside "netting" for NETTED, which comes back once
the walk starts from base words. The debatable rejection is ENCOMPASSED's
"Embraced or included.", whose "Embraced" starts with "embrace".

"Whole word or start of a word" gives up nothing of value here, though it would
miss a compound ending in a banned word, such as "bulldog" for DOG. Nothing in
this data needed one, but the data can't rule it out.

Some junk survives any matching rule, because the walk relates the wrong words:
"and" for OUTRIGHT, "fried" for POTATO, "being" for SPIRIT. Filtering the walk
itself would be a separate change.

## One possible approach

The approach is still open. One option, in `ClueClient::score`:

1. Match each banned word only as a whole word of the clue or at a word's start.
2. Also ban `get_conflicts` of each base word in `self.lemma.canonicals(word)`.

Then measure again, on the committed clues and on fresh raw candidates.
