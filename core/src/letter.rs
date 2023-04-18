use std::{iter, mem};
use std::cmp::Ordering;
use std::default::default;
use std::fmt::{Debug, Display, Formatter};
use std::iter::Step;
use std::ops::{Add, Index, IndexMut, Range, RangeInclusive, Sub};

use rand::distributions::{Distribution, Standard};
use rand::Rng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error, Visitor};
use any_ascii::any_ascii;

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Default)]
pub struct Letter(u8);

impl Debug for Letter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { Display::fmt(self, f) }
}

impl Display for Letter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.to_char()) }
}

impl Letter {
    pub fn new(x: u8) -> Result<Self, u8> {
        match x {
            b'a'..=b'z' => Ok(Letter(x - b'a')),
            b'A'..=b'Z' => Ok(Letter(x - b'A')),
            x => Err(x),
        }
    }
    pub fn to_char(&self) -> char { (self.0 + 'A' as u8) as char }
    pub fn from_index(x: usize) -> Option<Self> {
        if x < Self::LETTERS {
            return Some(Letter(x as u8));
        } else {
            None
        }
    }
    pub fn index(self) -> usize { self.0 as usize }
    pub const LETTERS: usize = 26;
    pub const MIN: Letter = Letter(0);
    pub const MAX: Letter = Letter((Self::LETTERS - 1) as u8);
    pub fn all() -> RangeInclusive<Letter> { Self::MIN..=Self::MAX }
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Copy, Clone, Default)]
#[repr(C)]
pub struct LetterMap<V>([V; Letter::LETTERS]);

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Copy, Clone, Default)]
#[repr(C)]
pub struct LetterSet(LetterMap<u8>);


impl<V> LetterMap<V> {
    pub fn new() -> Self
    where
        V: Default,
    {
        LetterMap(default())
    }
    pub fn map<V2>(self, f: impl FnMut(V) -> V2) -> LetterMap<V2> { LetterMap(self.0.map(f)) }
    pub fn iter(&self) -> impl Iterator<Item = (Letter, &V)> + Clone {
        self.0
            .iter()
            .enumerate()
            .map(|(l, v)| (Letter(l.try_into().unwrap()), v))
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Letter, &mut V)> {
        self.0
            .iter_mut()
            .enumerate()
            .map(|(l, v)| (Letter(l.try_into().unwrap()), v))
    }
    pub fn into_iter(self) -> impl Iterator<Item = (Letter, V)> {
        self.0
            .into_iter()
            .enumerate()
            .map(|(l, v)| (Letter::from_index(l).unwrap(), v))
    }
    pub fn zip<V2>(self, other: LetterMap<V2>) -> LetterMap<(V, V2)> {
        LetterMap(self.0.zip(other.0))
    }
    pub fn is_subset(self, other: Self) -> bool
    where
        V: Ord,
    {
        self.zip(other).iter().all(|(_, (a, b))| a <= b)
    }
}

impl LetterSet {
    pub fn from_str(w: &str) -> Self {
        let mut letters = LetterSet::new();
        for c in any_ascii(&w).chars() {
            if let Ok(letter) = Letter::new(c.try_into().unwrap()) {
                letters[letter] += 1;
            }
        }
        letters
    }
    pub fn from_counts(w: &[u8]) -> Self {
        assert_eq!(w.len(), Letter::LETTERS);
        LetterSet(LetterMap::<u8>(w.try_into().unwrap()))
    }
    pub fn count(&self) -> usize { self.0.iter().map(|(_, x)| *x as usize).sum::<usize>() }
    pub fn multiset_iter<'a>(&'a self) -> impl 'a + Iterator<Item = Letter> + Clone {
        self.iter()
            .flat_map(|(l, c)| iter::repeat(l).take(c as usize))
    }
    pub fn new() -> Self { LetterSet(LetterMap::new()) }
    pub fn iter<'a>(&'a self) -> impl 'a + Iterator<Item = (Letter, usize)> + Clone {
        self.0.iter().map(|(x, y)| (x, *y as usize))
    }
    pub fn is_subset(self, other: Self) -> bool {
        self.0.zip(other.0).iter().all(|(_, (a, b))| a <= b)
    }
}

impl<V> Index<Letter> for LetterMap<V> {
    type Output = V;
    fn index(&self, index: Letter) -> &Self::Output { &self.0[index.0 as usize] }
}

impl<V> IndexMut<Letter> for LetterMap<V> {
    fn index_mut(&mut self, index: Letter) -> &mut Self::Output { &mut self.0[index.0 as usize] }
}

impl Index<Letter> for LetterSet {
    type Output = u8;
    fn index(&self, index: Letter) -> &Self::Output { &self.0[index] }
}

impl IndexMut<Letter> for LetterSet {
    fn index_mut(&mut self, index: Letter) -> &mut Self::Output { &mut self.0[index] }
}

impl<T: Debug> Debug for LetterMap<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl Debug for LetterSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (l, x) in self.iter() {
            let c: isize = x.try_into().ok().unwrap();
            for _ in 0..c.abs() {
                if c < 0 {
                    write!(f, "-")?;
                }
                write!(f, "{}", l)?;
            }
        }
        Ok(())
    }
}

impl Step for Letter {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        Some(end.0.checked_sub(start.0)? as usize)
    }
    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        Letter::from_index((start.0 as usize).checked_add(count)?)
    }
    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        Letter::from_index((start.0 as usize).checked_sub(count)?)
    }
}

impl FromIterator<Letter> for LetterSet {
    fn from_iter<T: IntoIterator<Item = Letter>>(iter: T) -> Self {
        let mut set = LetterSet::new();
        for letter in iter {
            set[letter] += 1;
        }
        set
    }
}

impl<V> FromIterator<V> for LetterMap<V> {
    fn from_iter<T: IntoIterator<Item = V>>(iter: T) -> Self {
        LetterMap(
            iter.into_iter()
                .collect::<Vec<_>>()
                .try_into()
                .ok()
                .unwrap(),
        )
    }
}

impl<A, B> Sub<LetterMap<B>> for LetterMap<A>
where
    A: Sub<B>,
{
    type Output = LetterMap<A::Output>;
    fn sub(self, rhs: LetterMap<B>) -> Self::Output { self.zip(rhs).map(|(a, b)| a - b) }
}

impl<A, B> Add<LetterMap<B>> for LetterMap<A>
where
    A: Add<B>,
{
    type Output = LetterMap<A::Output>;
    fn add(self, rhs: LetterMap<B>) -> Self::Output { self.zip(rhs).map(|(a, b)| a + b) }
}

impl Sub<LetterSet> for LetterSet {
    type Output = LetterSet;
    fn sub(self, rhs: LetterSet) -> Self::Output { LetterSet(self.0 - rhs.0) }
}

impl Add<LetterSet> for LetterSet {
    type Output = LetterSet;
    fn add(self, rhs: LetterSet) -> Self::Output { LetterSet(self.0 + rhs.0) }
}

impl Distribution<Letter> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Letter {
        Letter(rng.gen_range(0..Letter::LETTERS).try_into().unwrap())
    }
}

impl Serialize for Letter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_char(('A' as u32 + self.0 as u32).try_into().unwrap())
    }
}

impl<'de> Deserialize<'de> for Letter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Vis;
        impl<'de> Visitor<'de> for Vis {
            type Value = Letter;
            fn expecting(&self, f: &mut Formatter) -> std::fmt::Result { write!(f, "character") }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Letter::new(v.chars().next().unwrap().try_into().unwrap()).unwrap())
            }
        }
        deserializer.deserialize_char(Vis)
    }
}
