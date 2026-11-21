use std::io::{self, ErrorKind, Read};
use std::iter;
use std::ops::RangeInclusive;
use std::sync::LazyLock;
use std::time::Instant;

use acrostic_core::letter::LetterSet;
use csv::StringRecord;
use itertools::Itertools;
use safe_once_async::async_lazy::AsyncLazy;
use safe_once_async::detached::{JoinTransparent, spawn_transparent};
use safe_once_async::sync::AsyncLazyLock;
use serde::Deserialize;
use serde::Serialize;

use crate::add_letters::{quote_to_cells, source_to_letters};
use crate::puzzle::Puzzle;
use crate::reformat::reformat_quote;
use crate::util::lazy_async::CloneError;
use crate::{PACKAGE_PATH, error_with_path, read_path, read_path_to_string, write_path};

#[derive(Debug, Serialize, Deserialize)]
pub struct Quote {
    pub quote: String,
    pub source: String,
}

pub static QUOTES: LazyLock<AsyncLazyLock<JoinTransparent<io::Result<Vec<Quote>>>>> =
    LazyLock::new(|| {
        AsyncLazy::new(spawn_transparent(async {
            let path = PACKAGE_PATH.join("build/quotes.json");
            serde_json::from_str(&read_path_to_string(&path).await?)
                .map_err(error_with_path("parse", &path))
        }))
    });

// The records of the quote file that get no puzzle number.
//
// Puzzle numbers must stay stable: published links and solvers' saved progress refer to puzzles by
// number. The numbers in use came from an earlier hand-written parser, which merged a few groups of
// records into single entries and dropped one empty record. Each merged group's first record keeps
// its entry's number, and the records listed here get none, so every later puzzle number stays
// where that parser put it.
//
// The list fits the Quotes-500K submodule at commit b82f2b88. Another commit of the submodule may
// need a different list; `test_pinned_quote_file_matches_committed_puzzles` catches one that no
// longer fits.
#[rustfmt::skip]
const PINNED_UNNUMBERED_RECORDS: &[usize] = &[
    // Merged with record 29,312 by a backslash after this record's opening quotation mark.
    29_313,
    // Merged with record 57,766, because this record spreads its quote across several fields and
    // has an empty field before its topics.
    57_767,
    // Merged with record 301,314 by a backslash after record 301,317's opening quotation mark.
    301_315, 301_316, 301_317,
    // An empty record, which the earlier parser dropped.
    422_587,
];

// How long a puzzle's quote and source may each be.
//
// In an acrostic, the answers' first letters, read in order, spell the source, so a puzzle has as
// many answers as its source has letters. The answers use each of the quote's letters once, so the
// quote's letters set the answers' combined length.
//
// The limits follow the New York Times acrostics' length distributions. Across 504 of its puzzles
// from 2000 to 2023, collected at github.com/lordjoe/Acrostics, the 1st to 99th percentiles run
// from 166 to 183 quote letters and from 20 to 26 answers. 97% of the puzzles meet both ranges and
// the minimum average answer length below.
const QUOTE_LENGTH_LIMITS: RangeInclusive<usize> = 166..=183;
const SOURCE_LENGTH_LIMITS: RangeInclusive<usize> = 20..=26;

// The fewest quote letters a puzzle may hold per answer, on average. With fewer, the answers grow
// so short that the answer search often fails, and the Times rarely goes below this threshold: 95%
// of its puzzles have at least 6.6 letters per answer.
const MINIMUM_AVERAGE_ANSWER_LENGTH: f64 = 6.5;

// The published puzzles whose quotes fall outside the current length limits. Stage 0 built them
// under earlier rules, and this allowlist lets them through so that rebuilding the puzzles keeps
// them on the site until enough puzzles within the limits are ready to replace them. A puzzle
// counts as published once it has a `stage3.json`, since the site serves only those.
#[rustfmt::skip]
pub const PUBLISHED_PUZZLES_OUTSIDE_LENGTH_LIMITS: &[usize] = &[
    1_455, 3_653, 4_113, 4_426, 6_492, 6_895, 7_042, 7_598, 8_863, 8_868,
    8_922, 9_113, 9_328, 9_513, 10_517, 10_908, 11_461, 14_790, 15_392, 15_550,
    17_027, 17_781, 18_569, 19_573, 20_724, 23_340, 24_513, 25_118, 25_645, 25_924,
    26_408, 26_476, 26_576, 27_045, 27_323, 27_492, 29_085, 29_089, 30_625, 30_922,
    31_136, 31_686, 35_944, 36_308, 37_008, 41_406, 41_578, 42_312, 43_882, 45_767,
    45_812, 45_964, 46_413, 47_432, 49_844, 52_113, 52_427, 52_545, 52_546, 52_598,
    53_316, 55_056, 57_067, 57_081, 58_822, 59_025, 59_071, 59_492, 59_650, 59_838,
    59_925, 60_679, 61_021, 63_625, 64_397, 65_901, 66_954, 67_180, 69_681, 69_792,
    70_715, 73_105, 73_294, 73_423, 73_779, 73_818, 73_901, 74_436, 75_038, 75_892,
    76_156, 76_213, 76_361, 80_330, 81_705, 81_736, 81_766, 82_175, 82_836, 82_901,
    83_348, 84_519, 84_621, 86_170, 86_791, 88_371, 88_588, 89_896, 90_165, 90_491,
    91_804, 93_807, 94_603, 97_612, 99_100, 99_397, 99_671, 106_792, 107_718, 108_164,
    108_608, 108_788, 112_789, 113_781, 114_449, 114_639, 116_391, 117_172, 117_890, 119_949,
    122_040, 122_312, 122_840, 123_233, 126_723, 127_236, 129_221, 130_088, 130_457, 131_531,
    131_913, 132_409, 134_613, 135_780, 136_772, 137_687, 137_873, 138_351, 141_556, 141_705,
    141_912, 141_992, 142_003, 142_549, 142_883, 142_932, 143_713, 146_247, 147_447, 147_587,
    148_588, 149_190, 149_514, 151_104, 151_743, 151_895, 152_467, 152_674, 153_013, 153_521,
    155_264, 155_314, 156_332, 159_503, 163_197, 165_612, 166_356, 167_469, 170_288, 170_945,
    172_378, 173_477, 173_731, 174_257, 176_123, 177_346, 177_382, 178_031, 179_070, 180_087,
    181_742, 181_749, 181_775, 181_946, 182_727, 183_091, 183_457, 183_570, 185_445, 185_772,
    186_025, 186_444, 186_690, 188_640, 188_787, 189_188, 189_605, 189_894, 190_118, 190_670,
    191_246, 195_375, 196_076, 196_097, 196_168, 196_283, 196_313, 196_602, 197_036, 197_097,
    197_332, 199_185, 199_302, 199_737, 199_996,
];

// Whether the quote numbered `puzzle_number`, holding `letters`, fits the length limits, or else
// belongs to a published puzzle on the allowlist.
fn passes_length_limits(puzzle_number: usize, letters: &QuoteLetters) -> bool {
    PUBLISHED_PUZZLES_OUTSIDE_LENGTH_LIMITS.contains(&puzzle_number) || letters.fits_length_limits()
}

impl Quote {
    // Builds a quote from one record of the quote file. A record holds the quote, its source, and
    // its comma-separated topics, followed by empty padding fields:
    //
    // - `Carpe diem.,Horace,"time, life",,,` holds the quote `Carpe diem.`, the source `Horace`,
    //   and the topics `time` and `life`.
    // - `Veni, vidi, vici.,Julius Caesar,history,,,` splits its quote across three fields. The
    //   last two fields are still the source and the topics, and the pieces before them rejoin,
    //   with commas, into the quote.
    // - `Carpe diem.,Horace,,,` has no topics, so its two remaining fields are the quote and the
    //   source.
    // - `Veni, vidi, vici.,Julius Caesar,,,` splits its quote and has no topics. The pieces after
    //   the first start with a space, so they rejoin the quote, and the last field is the source.
    // - `Carpe diem.,Horace, Odes,,,` splits its source and has no topics. ` Odes` starts with a
    //   space, so it rejoins the source, giving `Horace, Odes`.
    //
    // The topics are discarded, since puzzles use only the quote and the source. The parser still
    // has to find the topics field, though: the source is the field just before the topics.
    fn from_record(record: &StringRecord) -> Self {
        let mut fields: Vec<&str> = record.iter().collect();
        while fields.last() == Some(&"") {
            fields.pop();
        }
        // A field written without quotation marks splits at each of its commas. Where a space
        // followed a comma, the next piece starts with that space, as ` vici.` and ` Odes` do. A
        // whole field almost never starts with whitespace, so a field that does rejoins the field
        // before it. Rejoining may misread the records listed in `tasks.md #known-data-issues`.
        let mut rejoined_fields: Vec<String> = vec![];
        for field in fields {
            if field.starts_with(char::is_whitespace)
                && let Some(previous_field) = rejoined_fields.last_mut()
            {
                previous_field.push(',');
                previous_field.push_str(field);
            } else {
                rejoined_fields.push(field.to_string());
            }
        }
        // Note: Records 396,296 and 396,331 of the pinned file hold a second list of topics after
        // an empty field, as in `Quote,Author,adversity,,writing-craft`. Read by position, each
        // quote swallows its source, which comes out empty. Both quotes fall far short of the
        // length limits, so no puzzle comes from either.
        let (quote_pieces, source) = match rejoined_fields.as_slice() {
            [quote, source] => (std::slice::from_ref(quote), source.as_str()),
            // Rejoining misses a split with no space after its comma, so the quote can still span
            // several fields. For example, `One, two,three,Author,topic` rejoins into the fields
            // `One, two`, `three`, `Author`, and `topic`, and the quote is `One, two,three`. The
            // last field holds the topics, which are discarded.
            [quote_pieces @ .., source, _topics] => (quote_pieces, source.as_str()),
            // No fields, or a quote alone.
            quote_pieces => (quote_pieces, ""),
        };
        Self {
            quote: quote_pieces.join(","),
            source: source.to_string(),
        }
    }

    // Reads the letters of the quote and of its source, just as the answer search will read them.
    // Fails on a quote holding a character that a puzzle has no cell for, such as `☥`.
    fn letters(&self) -> io::Result<QuoteLetters> {
        Ok(QuoteLetters {
            quote: LetterSet::from_str(&quote_to_cells(&self.quote)?),
            source: LetterSet::from_str(&source_to_letters(&self.source)),
        })
    }
}

// The letters of a quote and of its source, which decide whether the quote can make a puzzle.
struct QuoteLetters {
    quote: LetterSet,
    source: LetterSet,
}

impl QuoteLetters {
    // Whether the quote and its source each fall within their length limits, with answers that
    // average at least the minimum length.
    fn fits_length_limits(&self) -> bool {
        let quote_length = self.quote.count();
        let source_length = self.source.count();
        QUOTE_LENGTH_LIMITS.contains(&quote_length)
            && SOURCE_LENGTH_LIMITS.contains(&source_length)
            && quote_length as f64 >= MINIMUM_AVERAGE_ANSWER_LENGTH * source_length as f64
    }

    // The letters the source needs beyond those the quote holds: for a source with two Y's and a
    // quote with one, a single Y. A puzzle needs this set to be empty, since each answer starts
    // with its letter of the source and takes all its letters from the quote.
    fn missing_source_letters(&self) -> LetterSet {
        self.source
            .iter()
            .flat_map(|(letter, source_count)| {
                let missing_count = source_count.saturating_sub(self.quote[letter] as usize);
                iter::repeat_n(letter, missing_count)
            })
            .collect()
    }
}

// Parses the quote file's CSV into a list indexed by puzzle number, giving every record except
// `unnumbered_records` the next number in turn. Fails when the input can't be read or isn't UTF-8,
// or when its records differ in width. Malformed CSV, such as an unclosed quotation mark, never
// fails on its own: the csv crate reads it leniently.
pub fn parse_quotes(input: impl Read, unnumbered_records: &[usize]) -> io::Result<Vec<Quote>> {
    // The csv crate reads the first record as a header unless told otherwise. This file has no
    // header row, and skipping its first record would shift every puzzle number.
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(input);
    let mut quotes = vec![];
    for (record_index, record) in reader.records().enumerate() {
        let record = record?;
        if unnumbered_records.contains(&record_index) {
            continue;
        }
        quotes.push(Quote::from_record(&record));
    }
    Ok(quotes)
}

// Parses the pinned Quotes-500K file, given its brotli-compressed bytes.
pub fn parse_pinned_quote_file(compressed: &[u8]) -> io::Result<Vec<Quote>> {
    parse_quotes(
        brotli::Decompressor::new(compressed, 4096),
        PINNED_UNNUMBERED_RECORDS,
    )
}

// Writes the pinned Quotes-500K file's quotes, indexed by puzzle number, to build/quotes.json.
pub async fn build_quotes() -> io::Result<()> {
    let start = Instant::now();
    let quote_file = PACKAGE_PATH.join("submodules/Quotes-500K/quotes.csv.br");
    let compressed = read_path(&quote_file).await?;
    let quotes =
        parse_pinned_quote_file(&compressed).map_err(error_with_path("parse", &quote_file))?;
    // Git ignores `build/`, so a fresh checkout lacks the folder.
    let build_folder = PACKAGE_PATH.join("build");
    tokio::fs::create_dir_all(&build_folder)
        .await
        .map_err(error_with_path("create", &build_folder))?;
    write_path(
        &build_folder.join("quotes.json"),
        serde_json::to_string_pretty(&quotes)?.as_bytes(),
    )
    .await?;
    let elapsed = start.elapsed();
    println!("{elapsed:?}");
    Ok(())
}

// Starts the puzzle numbered `puzzle_number` from its quote: stage 0's puzzle holds just the
// quote's source and reformatted text. The solved puzzle shows the reformatted text, such as
// `end. Begin` for `end.Begin`. Fails when the reformatted quote can't make a puzzle, as when it
// falls outside the length limits; `check_quote` gives every reason.
pub fn start_puzzle(puzzle_number: usize, quote: &Quote) -> io::Result<Puzzle> {
    let reformatted_quote = Quote {
        quote: reformat_quote(&quote.quote)?,
        source: quote.source.clone(),
    };
    check_quote(puzzle_number, &reformatted_quote)?;
    let puzzle = Puzzle {
        quote: reformatted_quote.quote,
        quote_letters: None,
        source: reformatted_quote.source,
        source_letters: None,
        clues: None,
        chat: None,
    };
    Ok(puzzle)
}

// Checks that a reformatted quote, numbered `puzzle_number`, can make a puzzle. Fails when the
// quote holds a character that a puzzle has no cell for, when the quote or its source falls
// outside the length limits and the puzzle isn't allowlisted, or when the quote lacks letters that
// the answers need to spell the source.
fn check_quote(puzzle_number: usize, quote: &Quote) -> io::Result<()> {
    let letters = quote.letters()?;
    if !passes_length_limits(puzzle_number, &letters) {
        let quote_length = letters.quote.count();
        let source_length = letters.source.count();
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "unusable quote: the quote holds {quote_length} letters and the source \
                 {source_length}, but a puzzle needs {QUOTE_LENGTH_LIMITS:?} and \
                 {SOURCE_LENGTH_LIMITS:?}, with answers averaging at least \
                 {MINIMUM_AVERAGE_ANSWER_LENGTH} letters"
            ),
        ));
    }
    let missing_letters = letters.missing_source_letters();
    if missing_letters.count() > 0 {
        let source = &quote.source;
        let missing_letter_list = missing_letters.multiset_iter().join(", ");
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "unusable quote: the source {source:?} needs letters that its quote lacks: \
                 {missing_letter_list}"
            ),
        ));
    }
    Ok(())
}

// Starts the puzzle numbered `pindex` from its quote, and writes it to stage0.json.
pub async fn add_quote(pindex: usize) -> io::Result<()> {
    let quotes = QUOTES.get().await.clone_error_static()?;
    let puzzle = start_puzzle(pindex, &quotes[pindex])?;
    puzzle.write(pindex, "stage0.json").await?;
    Ok(())
}
