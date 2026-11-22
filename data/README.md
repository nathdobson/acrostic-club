# Data

This folder holds data for the generator:

- `cluebank.json`: every clue written for each answer word. Its format is
  described in `generator/src/cluebank.rs`.
- `lexicon.json`: candidate answer words, each rated for familiarity and
  liveliness.

## The lexicon

`lexicon.json` rates English words as candidate puzzle answers. Each line holds
one word and its two ratings:

```json
"spatula": {"familiarity": 7, "liveliness": 6},
```

Words appear as wordfreq writes them: lowercase, with any accents and
apostrophes. The file keeps every word that was rated, including the junk and
the poor answers, so that code can set its own cutoffs.

### The ratings

Both ratings run from 1 to 7. The prompt, linked at the end, gives the full
scales; in brief:

- **Familiarity** says how many American adults would know the word: recognize
  it as a word or a name and have a rough idea what it means or refers to. A 1
  means almost no one would, a 4 about half, a 5 most, and a 7 everyone.
- **Liveliness** says how good an answer the word would make, for a solver who
  knows it. A 1 is a poor answer, such as junk, an awkward form, or a grim
  word. A 2 is dull: jargon or a bland inflected form. A 4 is a solid ordinary
  answer, and a 6 is vivid, concrete, or fun.

The two are separate on purpose: an obscure word can still be lively, and an
everyday word can be dull. Each rating is one model's judgment, so expect some
noise word by word.

### How the words were chosen and rated

The candidates are the words of six or more letters in wordfreq's English word
list, version 3.1.1. wordfreq blends word frequencies from several kinds of
text, such as subtitles, news, books, and Wikipedia. A word that only one kind
of text favors ranks low, as Wikipedia's markup and taxonomy terms do. The list
stops at a Zipf frequency of 1, about once per 100 million words.

- **Above Zipf 2**, about once per 10 million words: Claude Opus 5.5 rated
  every candidate.
- **Between Zipf 1 and 2**: Claude Haiku 5.5 first rated every candidate, with
  the same prompt. Opus then rated the words Haiku scored highly enough: a
  familiarity of 4 or more, or of 3 with a liveliness of 3 or more. Opus also
  rated a random sample of 2,000 words from this band. The other words that
  Haiku set aside are not in the file.

### License and credits

`lexicon.json` adapts the data of
[wordfreq](https://github.com/rspeer/wordfreq), by Robyn Speer. wordfreq's data
is licensed under the Creative Commons Attribution-ShareAlike 4.0 license
([CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/)), so
`lexicon.json` is licensed under CC BY-SA 4.0 as well. The adaptation changes
wordfreq's English word list in four ways:

- It keeps only the words chosen above.
- It drops their frequencies.
- It adds a few rare number words that wordfreq lacks.
- It adds the ratings.

wordfreq's data draws on these sources, which its authors credit:

- the SUBTLEX word lists by Marc Brysbaert and others, which are freely
  available at <http://crr.ugent.be/programs-data/subtitle-frequencies>;
- OpenSubtitles (<http://www.opensubtitles.org/>), through the OPUS
  OpenSubtitles 2018 corpus;
- the Google Books Ngram Viewer (<http://books.google.com/ngrams>);
- Wikipedia, the Leeds Internet Corpus, and ParaCrawl.

The ratings were made with Claude Opus 5.5 and Claude Haiku 5.5, by Anthropic.

### The prompt

Each request sent the system prompt in
[`generator/prompts/lexicon_rating.md`](../generator/prompts/lexicon_rating.md),
then a batch of words, one per line. Replies had to match this JSON schema:

```json
{
  "type": "object",
  "properties": {
    "ratings": {"type": "array", "items": {"type": "string"}}
  },
  "required": ["ratings"],
  "additionalProperties": false
}
```
