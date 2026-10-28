//! **`std::regex`, a pattern that arrives at run time**
//! ([ADR-regex-1](../../../docs/specification/adr/adr-regex-1.md) D3, D8):
//! `regex::dynamic(pattern)` compiles RE2's dialect (D1) with
//! `regex-automata`'s `meta::Regex`, as `std`'s maps wrap `hashbrown`'s table
//! (ADR-296 D42). No type of the crate leaves this file: a program, the ledger
//! and a message see `regex::Regex[regex::Dynamic]`, `regex::Captures`,
//! `regex::Error` and `text::Match`.
//!
//! **The engine is the type's parameter from the start** (D3): the engines a
//! pattern the build knows is compiled to (D6) come later, as types, and a
//! program's type does not change the day they do. `regex::Dynamic` is the
//! floor and today the only one.
//!
//! **No positions yet**: a match is its text, a view of the searched text;
//! `start`, `end` and `from:` arrive with ADR-374's `text::Pos`.

use std::marker::PhantomData;

use regex_automata::meta;

/// **What a regular expression runs on** (D3): one type per engine, and the
/// calls of each are its own.
pub trait Engine {}

/// **The engine of a pattern compiled while the program runs** (D8):
/// `regex-automata`'s `meta::Regex`.
#[derive(Clone, Copy, Debug)]
pub struct Dynamic;

impl Engine for Dynamic {}

/// **A compiled pattern, typed by its engine** (D3). Read-only, so one value
/// is searched from any number of threads at once.
#[derive(Clone, Debug)]
pub struct Regex<E> {
    inner: meta::Regex,
    engine: PhantomData<E>,
}

/// **A pattern that does not compile** (D8): the crate's message, as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    said: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.said)
    }
}

impl std::error::Error for Error {}

/// **One match, a view of the searched text** (ADR-374's `text::Match`,
/// without its positions until that is built).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match<'s> {
    text: &'s str,
}

impl<'s> Match<'s> {
    /// The matched text, a view of the text that was searched.
    pub fn text(&self) -> &'s str {
        self.text
    }
}

impl std::fmt::Display for Match<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.text)
    }
}

/// **The groups of one match** (D3): group 0 is the whole match.
#[derive(Clone, Debug)]
pub struct Captures<'s> {
    hay: &'s str,
    caps: regex_automata::util::captures::Captures,
}

impl<'s> Captures<'s> {
    /// Group `i`, or `None` where it did not take part or there is no such
    /// group.
    pub fn group(&self, i: i64) -> Option<Match<'s>> {
        let i = usize::try_from(i).ok()?;
        let span = self.caps.get_group(i)?;
        Some(Match {
            text: &self.hay[span.range()],
        })
    }

    /// The group called `name`, or `None` where it did not take part or no
    /// group has that name.
    pub fn named(&self, name: &str) -> Option<Match<'s>> {
        let span = self.caps.get_group_by_name(name)?;
        Some(Match {
            text: &self.hay[span.range()],
        })
    }
}

/// **A pattern known only at run time, compiled** (D3, D8), or the crate's
/// message where it does not compile.
pub fn dynamic(pattern: &str) -> Result<Regex<Dynamic>, Error> {
    match meta::Regex::new(pattern) {
        Ok(inner) => Ok(Regex {
            inner,
            engine: PhantomData,
        }),
        Err(error) => Err(Error {
            said: match error.syntax_error() {
                Some(syntax) => syntax.to_string(),
                None => error.to_string(),
            },
        }),
    }
}

impl Regex<Dynamic> {
    /// Whether the pattern matches anywhere in `s`.
    pub fn is_match(&self, s: &str) -> bool {
        self.inner.is_match(s)
    }

    /// The leftmost-first match in `s`, or `None`.
    pub fn find<'s>(&self, s: &'s str) -> Option<Match<'s>> {
        let found = self.inner.find(s)?;
        Some(Match {
            text: &s[found.range()],
        })
    }

    /// Every match in `s`, left to right, not overlapping.
    pub fn find_all<'r, 's>(&'r self, s: &'s str) -> impl Iterator<Item = Match<'s>> + 'r
    where
        's: 'r,
    {
        self.inner.find_iter(s).map(move |found| Match {
            text: &s[found.range()],
        })
    }

    /// The groups of the leftmost-first match in `s`, or `None`.
    pub fn captures<'s>(&self, s: &'s str) -> Option<Captures<'s>> {
        let mut caps = self.inner.create_captures();
        self.inner.captures(s, &mut caps);
        if !caps.is_match() {
            return None;
        }
        Some(Captures { hay: s, caps })
    }

    /// `s` with every match replaced by `with`, in which `$1` and `${name}`
    /// stand for a group's text (the crate's interpolation; `$$` is a `$`).
    pub fn replace_all(&self, s: &str, with: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut last = 0;
        for caps in self.inner.captures_iter(s) {
            let Some(whole) = caps.get_group(0) else {
                continue;
            };
            out.push_str(&s[last..whole.start]);
            caps.interpolate_string_into(s, with, &mut out);
            last = whole.end;
        }
        out.push_str(&s[last..]);
        out
    }

    /// The pieces of `s` between the matches, each a view of `s`.
    pub fn split<'r, 's>(&'r self, s: &'s str) -> impl Iterator<Item = &'s str> + 'r
    where
        's: 'r,
    {
        self.inner.split(s).map(move |span| &s[span.range()])
    }
}
