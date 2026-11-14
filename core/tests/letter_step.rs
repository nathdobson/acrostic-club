// Tests for `Letter`'s `Step` impl: iteration over letter ranges, and the rule that one jump of n
// steps matches n single steps, which `Step` requires even across overflow.

#![feature(step_trait)]

use std::iter::Step;

use acrostic_core::letter::Letter;

// Returns the letter for an ASCII letter byte, such as `b'Y'`.
fn letter(byte: u8) -> Letter {
    Letter::new(byte).expect("the byte should be an ASCII letter")
}

// Returns the letter at `index`, where 0 is A.
fn letter_at(index: usize) -> Letter {
    Letter::from_index(index).expect("the index should be below 26")
}

#[test]
fn iterating_all_letters_forward_visits_each_letter_once() {
    let mut letters = Letter::all();
    let mut visited = String::new();
    while let Some(next) = letters.next() {
        visited.push(next.to_char());
        assert!(visited.len() <= 26, "iteration should stop after Z");
    }

    assert_eq!("ABCDEFGHIJKLMNOPQRSTUVWXYZ", visited);
}

#[test]
fn iterating_all_letters_backward_visits_each_letter_once() {
    let mut letters = Letter::all();
    let mut visited = String::new();
    while let Some(next) = letters.next_back() {
        visited.push(next.to_char());
        assert!(visited.len() <= 26, "iteration should stop after A");
    }

    assert_eq!("ZYXWVUTSRQPONMLKJIHGFEDCBA", visited);
}

#[test]
fn nth_of_the_last_letter_returns_z_and_exhausts_the_range() {
    let mut letters = Letter::all();

    assert_eq!(Some(letter(b'Z')), letters.nth(25));
    assert_eq!(None, letters.next());
}

#[test]
fn nth_past_the_end_returns_none_and_exhausts_the_range() {
    let mut letters = Letter::all();

    assert_eq!(None, letters.nth(26));
    assert_eq!(None, letters.next());
}

#[test]
fn stepping_forward_past_z_saturates_at_z() {
    assert_eq!((letter(b'Z'), true), Step::forward_overflowing(letter(b'Y'), 2));
    assert_eq!((letter(b'Z'), true), Step::forward_overflowing(letter(b'D'), usize::MAX));
}

#[test]
fn stepping_backward_past_a_saturates_at_a() {
    assert_eq!((letter(b'A'), true), Step::backward_overflowing(letter(b'B'), 2));
    assert_eq!((letter(b'A'), true), Step::backward_overflowing(letter(b'D'), usize::MAX));
}

#[test]
fn one_forward_jump_matches_single_steps_from_every_letter() {
    for start in 0..26 {
        for count in 0..60 {
            let jump = Step::forward_overflowing(letter_at(start), count);
            let steps = (0..count).fold((letter_at(start), false), |(at, overflowed), _| {
                let (next, overflow) = Step::forward_overflowing(at, 1);
                (next, overflowed || overflow)
            });
            assert_eq!(steps, jump, "start {start}, count {count}");
        }
    }
}

#[test]
fn one_backward_jump_matches_single_steps_from_every_letter() {
    for start in 0..26 {
        for count in 0..60 {
            let jump = Step::backward_overflowing(letter_at(start), count);
            let steps = (0..count).fold((letter_at(start), false), |(at, overflowed), _| {
                let (next, overflow) = Step::backward_overflowing(at, 1);
                (next, overflowed || overflow)
            });
            assert_eq!(steps, jump, "start {start}, count {count}");
        }
    }
}
