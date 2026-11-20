// Restores the spaces that a quote's text lost before it reached this repository, wherever
// punctuation between two words stands in for one, as in `end.Begin`. The solved puzzle shows the
// restored text, and stage 1 lays it out as cells, so the cells break the words there too.

use std::collections::HashSet;
use std::io;
use std::ops::Range;

use crate::words_and_marks::{
    Mark, MarksPlacement, Run, place_marks, split_into_runs, words_beside,
};

// Marks that open what follows them, so that a lost space goes before them rather than after:
// quotation marks, including the reversed `‟`, the German `„`, and the French `«`; brackets; the
// Spanish `¿` and `¡`; and `#` and `~`, which start a hashtag and a quote's attribution.
const OPENING_MARKS: &[char] = &['“', '‘', '‟', '„', '«', '(', '[', '{', '¿', '¡', '#', '~'];

// Quotation marks that look the same whether they open a quotation or close it.
const STRAIGHT_QUOTATION_MARKS: &[char] = &['"', '\''];

// Punctuation that ends a sentence or clause.
const ENDING_PUNCTUATION: &[char] = &[',', '.', ';', ':', '!', '?', '…'];

// Adds a space wherever punctuation between two words stands in for a space the text lost:
// `end.Begin` becomes `end. Begin`, while `don't` and `3:16` stay as they are. The space goes
// before the first mark that opens what follows, as in `said, “Go`, or else after all the marks.
//
// Fails on a quote that no puzzle can show, such as one holding `☥`: finding where words break
// means telling every character apart.
pub fn restore_lost_spaces(quote: &str) -> io::Result<String> {
    let opening_straight_marks = find_opening_straight_marks(quote);
    let mut restored = String::new();
    let mut copied_up_to = 0;
    for lost_space in find_lost_spaces(quote)? {
        let space_offset = choose_space_offset(quote, &lost_space, &opening_straight_marks);
        restored.push_str(&quote[copied_up_to..space_offset]);
        restored.push(' ');
        copied_up_to = space_offset;
    }
    restored.push_str(&quote[copied_up_to..]);
    Ok(restored)
}

// Finds each place where punctuation between two words stands in for a space the text lost. Gives
// each as the byte range of the marks among which the space belongs: `3..4`, the `.`, for
// `end.Begin`.
//
// The marks between two words lost no space when one of them already shows a cell between the
// words, as whitespace, a dash, or a slash does; when they sit inside a word, as the apostrophe
// does in `don't`; or when they sit inside a number, as the colon does in `3:16`. A symbol beside
// a number stays outside the range, since a space between the two would part them: for `17%of`,
// the range is the empty `3..3`, just after the `%`.
fn find_lost_spaces(quote: &str) -> io::Result<Vec<Range<usize>>> {
    let runs = split_into_runs(quote)?;
    let mut lost_spaces = vec![];
    for (index, run) in runs.iter().enumerate() {
        let (marks, mark_offsets) = match run {
            Run::Marks {
                marks,
                mark_offsets,
                ..
            } => (marks, mark_offsets),
            Run::Word(_) => continue,
        };
        // Marks at either end of the quote separate nothing.
        let (Some(word_before), Some(word_after)) = words_beside(&runs, index) else {
            continue;
        };
        let between = match place_marks(marks, Some(word_before), Some(word_after)) {
            // Marks inside a number, as in `3:16`, stand in for no space.
            MarksPlacement::InsideNumber => continue,
            MarksPlacement::OutsideNumber { between, .. } => between,
        };
        let between_marks = &marks[between.clone()];
        let shows_a_break_already = between_marks
            .iter()
            .any(|mark| mark.cell_between_words().is_some());
        // No marks left between the words means that a symbol beside a number ran straight into
        // the next word, as in `17%of`.
        let is_one_word =
            !between_marks.is_empty() && is_inside_word(between_marks, word_before, word_after);
        if shows_a_break_already || is_one_word {
            continue;
        }
        lost_spaces.push(mark_offsets[between.start]..mark_offsets[between.end]);
    }
    Ok(lost_spaces)
}

// Whether marks between two words, none of which keeps the words apart, sit inside one word.
//
// Apostrophes, brackets, and a few symbols sit inside words: `don't`, `[T]he`, `f*ck`, `AT&T`.
// So does a dot between single letters, as in `U.S.A.` Other punctuation between two words stands
// in for a space or line break that the text lost, as in `end.Begin` or `love#life`.
fn is_inside_word(marks: &[Mark], word_before: &str, word_after: &str) -> bool {
    let is_single_letter = |word: &str| {
        word.len() == 1
            && word
                .chars()
                .all(|character| character.is_ascii_alphabetic())
    };
    match marks {
        [Mark::Dot] if is_single_letter(word_before) && is_single_letter(word_after) => true,
        _ => marks.iter().all(|mark| matches!(mark, Mark::InsideWord)),
    }
}

// Chooses where a lost space goes among the marks at `lost_space`, a byte range of `quote`:
// before the first mark that opens what follows, or else just after the marks.
fn choose_space_offset(
    quote: &str,
    lost_space: &Range<usize>,
    opening_straight_marks: &HashSet<usize>,
) -> usize {
    let is_opening = |&(offset, character): &(usize, char)| {
        OPENING_MARKS.contains(&character) || opening_straight_marks.contains(&offset)
    };
    quote[lost_space.clone()]
        .char_indices()
        .map(|(index, character)| (lost_space.start + index, character))
        .find(is_opening)
        .map_or(lost_space.end, |(offset, _)| offset)
}

// The part a quotation mark plays in its quotation.
#[derive(Clone, Copy, PartialEq)]
enum QuotationMarkRole {
    Opening,
    Closing,
}

impl QuotationMarkRole {
    // The role a quotation mark plays when the mark beside it plays this one.
    fn opposite(self) -> Self {
        match self {
            QuotationMarkRole::Opening => QuotationMarkRole::Closing,
            QuotationMarkRole::Closing => QuotationMarkRole::Opening,
        }
    }
}

// Finds the straight quotation marks that open a quotation rather than close one, as their byte
// offsets in `quote`: in `"Go,"he said`, just the first mark's offset, 0. A `'` between two
// letters or digits is an apostrophe, as in `don't`, so it plays no part. The characters beside a
// mark show its role where they can, as a space before it shows that it opens; any other mark
// takes the role opposite to the mark beside it.
fn find_opening_straight_marks(quote: &str) -> HashSet<usize> {
    let characters: Vec<(usize, char)> = quote.char_indices().collect();
    let mut opening_offsets = HashSet::new();
    for &kind in STRAIGHT_QUOTATION_MARKS {
        let mut offsets = vec![];
        let mut shown_roles = vec![];
        for (index, &(offset, character)) in characters.iter().enumerate() {
            if character != kind {
                continue;
            }
            let character_before = index
                .checked_sub(1)
                .map(|index_before| characters[index_before].1);
            let character_after = characters.get(index + 1).map(|&(_, after)| after);
            let is_apostrophe = kind == '\''
                && character_before.is_some_and(char::is_alphanumeric)
                && character_after.is_some_and(char::is_alphanumeric);
            if is_apostrophe {
                continue;
            }
            offsets.push(offset);
            shown_roles.push(shown_role(kind, character_before, character_after));
        }
        let roles = infer_roles(&shown_roles);
        let opening_marks = offsets
            .into_iter()
            .zip(roles)
            .filter(|&(_, role)| role == QuotationMarkRole::Opening);
        opening_offsets.extend(opening_marks.map(|(offset, _)| offset));
    }
    opening_offsets
}

// The role that the characters beside a straight quotation mark show it plays, where they show
// one. Each neighbor is `None` at the quote's edge. The first of these signs that applies decides:
//
// - Two marks side by side close one quotation and open the next, as in `"Is it?""Yes."`.
// - A mark with whitespace or the quote's edge before it, and none after it, opens, as in `he
//   said "Go`.
// - A mark with whitespace or the quote's edge after it, and none before it, closes, as in `"Go,"
//   he said`.
// - A mark just before punctuation that ends a sentence or clause closes, as in `"Go",he said`,
//   since no quotation starts with that punctuation.
fn shown_role(
    mark: char,
    character_before: Option<char>,
    character_after: Option<char>,
) -> Option<QuotationMarkRole> {
    let has_whitespace_before = character_before.is_none_or(char::is_whitespace);
    let has_whitespace_after = character_after.is_none_or(char::is_whitespace);
    let has_ending_punctuation_after =
        character_after.is_some_and(|after| ENDING_PUNCTUATION.contains(&after));
    if character_after == Some(mark) {
        Some(QuotationMarkRole::Closing)
    } else if character_before == Some(mark) {
        Some(QuotationMarkRole::Opening)
    } else if has_whitespace_before && !has_whitespace_after {
        Some(QuotationMarkRole::Opening)
    } else if has_whitespace_after && !has_whitespace_before {
        Some(QuotationMarkRole::Closing)
    } else if has_ending_punctuation_after {
        Some(QuotationMarkRole::Closing)
    } else {
        None
    }
}

// Gives the role of each of a quote's marks of one kind, in order, given the role that the
// characters beside each mark show, or `None` where they show none. A mark whose role isn't shown
// takes the opposite role to the mark just before it: in `"Go."He left`, the first mark opens, so
// the second closes. Marks before the first with a shown role take the opposite role to the mark
// just after them instead. When no mark's role is shown, the marks take turns, starting with an
// opening one.
fn infer_roles(shown_roles: &[Option<QuotationMarkRole>]) -> Vec<QuotationMarkRole> {
    let mut roles = shown_roles.to_vec();
    for index in 1..roles.len() {
        if roles[index].is_none() {
            roles[index] = roles[index - 1].map(QuotationMarkRole::opposite);
        }
    }
    for index in (1..roles.len()).rev() {
        if roles[index - 1].is_none() {
            roles[index - 1] = roles[index].map(QuotationMarkRole::opposite);
        }
    }
    let role_by_turn = |index: usize| {
        if index % 2 == 0 {
            QuotationMarkRole::Opening
        } else {
            QuotationMarkRole::Closing
        }
    };
    roles
        .into_iter()
        .enumerate()
        .map(|(index, role)| role.unwrap_or_else(|| role_by_turn(index)))
        .collect()
}
