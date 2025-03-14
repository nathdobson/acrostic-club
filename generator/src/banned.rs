use std::collections::HashSet;
use std::sync::LazyLock;

static CARDINALS: &[&str] = &[
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
    "thirty",
    "forty",
    "fourty",
    "fifty",
    "sixty",
    "seventy",
    "eighty",
    "ninety",
    "hundred",
    "thousand",
    "million",
    "billion",
    "trillion",
    "quadrillion",
    "quintillion",
    "sextillion",
    "octillion",
    "nonillion",
    "decillion",
];

static ORDINALS: &[&str] = &[
    "zeroth",
    "zeroeth",
    "first",
    "second",
    "third",
    "fourth",
    "fifth",
    "sixth",
    "seventh",
    "eighth",
    "nineth",
    "ninth",
    "tenth",
    "eleventh",
    "twelfth",
    "thirteenth",
    "fourteenth",
    "fifteenth",
    "sixteenth",
    "seventeenth",
    "eighteenth",
    "nineteenth",
    "twentieth",
    "thirtieth",
    "fortieth",
    "fourtieth",
    "fiftieth",
    "sixtieth",
    "seventieth",
    "eightieth",
    "ninetieth",
    "hundredth",
    "thousandth",
    "millionth",
    "billionth",
    "trillionth",
    "quadrillionth",
    "quintillionth",
    "sextillionth",
    "octillionth",
    "nonillionth",
    "decillionth",
];

static LUDICROUS: &[&str] = &["london-based"];

pub static BANNED_WORDS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    CARDINALS
        .iter()
        .chain(ORDINALS.iter())
        .chain(LUDICROUS.iter())
        .map(|x| x.to_string())
        .collect()
});
