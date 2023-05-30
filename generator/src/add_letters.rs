use std::io;
use crate::puzzle::Puzzle;
use crate::string::{GraphemeString, LetterString};
use std::fmt::Write;
// pub fn segment(s: &str) -> Vec<EitherOrBoth<Letter, String>> {
//     let mut result = vec![];
//     for grapheme in s.graphemes(true) {
//         let mut letters = vec![];
//         let ascii = any_ascii(grapheme);
//         for c in ascii.bytes() {
//             if let Ok(l) = Letter::new(c) {
//                 letters.push(l);
//             }
//         }
//         if letters.len() == 0 {
//             result.push(EitherOrBoth::Right(grapheme.to_string()));
//         } else {
//             for (index, letter) in letters.iter().enumerate() {
//                 if index == 0 {
//                     result.push(EitherOrBoth::Both(*letter, grapheme.to_string()));
//                 } else {
//                     result.push(EitherOrBoth::Left(*letter))
//                 }
//             }
//         }
//     }
//     result
// }

fn quote_to_cells(input: &str) -> String {
    let mut cells = String::new();
    for grapheme in GraphemeString::from_str(&input).graphemes() {
        if grapheme.letters().is_empty() {
            let content = if grapheme.ascii().is_empty() { grapheme.string() } else { grapheme.ascii() };
            if content.chars().all(|x| x.is_ascii_digit()) {
                write!(&mut cells, "{}", content).unwrap();
                continue;
            }
            match content {
                " " => {
                    if cells.chars().next_back() != Some(' ') {
                        write!(&mut cells, "{}", content).unwrap();
                    }
                }

                "." | "," | ";" | "'" | "\"" | "!" | "?" | "‘" | "’" | ":"
                | "&" | "*" | "(" | ")" | "”" | "“" | "…" | "\n" | "\u{a0}"
                | "$" | "~" | "\t" | "_" | "/" | "´" | "[" | "]" | "#" | "..." => {}

                "-" | "—" | "–" => {
                    write!(&mut cells, "-").unwrap();
                }
                x => { panic!("{:?}", x); }
            }
        } else {
            cells.extend(grapheme.letters().iter().map(|x| x.to_char()));
        }
    }
    cells
}


pub async fn add_letters(pindex: usize) -> io::Result<()> {
    let mut puzzle = Puzzle::read(pindex, "stage0.json").await?;
    puzzle
        .quote_letters
        .get_or_insert_with(|| quote_to_cells(&puzzle.quote));
    puzzle.source_letters.get_or_insert_with(|| {
        LetterString::from_str(&puzzle.source).iter().map(|x| x.to_char()).collect()
    });
    puzzle.write(pindex, "stage1.json").await?;
    Ok(())
}

#[test]
fn test_quote_to_cells() {
    assert_eq!(quote_to_cells("０１２３４５６７８９"), "0123456789");
    assert_eq!(quote_to_cells("ＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲＳＴＵＶＷＸＹＺ"), "ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    assert_eq!(quote_to_cells("ａｂｃｄｅｆｇｈｉｊｋｌｍｎｏｐｑｒｓｔｕｖｗｘｙｚ"), "ABCDEFGHIJKLMNOPQRSTUVWXYZ");
}