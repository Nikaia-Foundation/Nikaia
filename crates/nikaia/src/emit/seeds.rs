//! **A map at a package's boundary is a type parameter each use fills**
//! ([ADR-010](../../../../docs/specification/adr/adr-010.md) D9, #581 step 5).
//!
//! A map position in a `pub` struct's fields or in a `pub` function's
//! parameters and result is a hidden type parameter `__S0`, `__S1`, ... of that
//! item, in the order the declaration writes them, bounded by
//! `nikaia_std::hash::Seed`. A `pub` struct's parameters default to
//! `nikaia_std::hash::Keyed`. A mention of such a struct is a position that
//! carries its parameters too: in a `pub` function's signature they become
//! that function's, anywhere else the package's own choice fills them
//! (`Fast` where the package's input is trusted, `Keyed` otherwise).
//!
//! The types are written by the type emitter as it always did; this reads the
//! text of one written type and puts the parameters in. A type's text is one
//! well-formed run of paths and angle brackets, which is all it has to be.

use std::collections::BTreeMap;

use crate::ast::{Item, Type};
use crate::parser::Parsed;

const SEED: &str = "nikaia_std::hash::Seed";
const KEYED: &str = "nikaia_std::hash::Keyed";
const FAST: &str = "nikaia_std::hash::Fast";

/// What the packages read declare: which `pub` structs hold maps (and how
/// many positions, nested structs counted), and which `pub` functions have
/// positions no argument can fix.
#[derive(Debug, Clone, Default)]
pub struct Seeds {
    /// The hidden parameters of each `pub` struct, by name.
    types: BTreeMap<String, usize>,
    /// Calls that have to be filled by hand: a `pub` function with no type
    /// parameter of its own whose hidden parameters are not all fixed by an
    /// argument, with their number.
    calls: BTreeMap<String, (usize, usize)>,
    /// Some `pub` function holds a map position.
    functions: bool,
}

/// Where the parameters of a type being written come from.
#[derive(Debug, Clone, Default)]
pub struct Place {
    /// An item that declares parameters is being written: its positions are
    /// numbered from `next`, and a type written twice shares them (`seen`).
    declaring: bool,
    next: usize,
    seen: Vec<(String, String)>,
    /// Inside the `impl` of a type that declares parameters: a mention of the
    /// type is `Self`'s.
    within: Option<(String, Vec<String>)>,
}

impl Place {
    /// An item that declares parameters starts here, after `base` of its
    /// `impl`'s.
    pub fn declare(&mut self, base: usize) {
        self.declaring = true;
        self.next = base;
        self.seen.clear();
    }

    /// The item is written: how many parameters it declared past `base`.
    pub fn finish(&mut self, base: usize) -> usize {
        let n = self.next.saturating_sub(base);
        self.declaring = false;
        self.next = 0;
        self.seen.clear();
        n
    }

    pub fn enter(&mut self, target: &str, params: Vec<String>) {
        self.within = Some((target.to_string(), params));
    }

    pub fn leave(&mut self) {
        self.within = None;
    }

    pub fn within(&self) -> bool {
        self.within.is_some()
    }
}

fn last(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

fn is_map(name: &str) -> bool {
    matches!(
        last(name),
        "HashMap" | "HashSet" | "TrustedMap" | "TrustedSet"
    )
}

impl Seeds {
    /// Reads every file of every package in the build.
    pub fn read(units: &[&Parsed]) -> Seeds {
        let mut seeds = Seeds::default();
        // Fields of every public struct, then the count of each, until the
        // counts stop growing (a struct holds another one that holds a map).
        let mut declared: Vec<(String, Vec<(&Parsed, &Type)>)> = Vec::new();
        for parsed in units {
            for item in &parsed.program.items {
                if let Item::Struct {
                    name,
                    fields,
                    is_public: true,
                    is_extern: false,
                    ..
                } = &item.node
                {
                    declared.push((
                        parsed.text(*name).to_string(),
                        fields.iter().map(|f| (*parsed, &f.ty)).collect(),
                    ));
                }
            }
        }
        for (name, _) in &declared {
            seeds.types.insert(name.clone(), 0);
        }
        for _ in 0..8 {
            let mut changed = false;
            for (name, fields) in &declared {
                let n: usize = fields.iter().map(|(p, t)| seeds.count(p, t)).sum();
                if seeds.types.get(name) != Some(&n) {
                    seeds.types.insert(name.clone(), n);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        seeds.types.retain(|_, n| *n > 0);
        for parsed in units {
            for item in &parsed.program.items {
                if let Item::Fn {
                    name: Some(name),
                    generics,
                    args,
                    ret_type,
                    is_public,
                    receiver: None,
                    ..
                } = &item.node
                {
                    if !*is_public
                        && !args
                            .iter()
                            .map(|a| &a.ty)
                            .chain(ret_type.iter())
                            .any(|t| seeds.mentions_a_struct(parsed, t))
                    {
                        continue;
                    }
                    let params: Vec<&Type> = args.iter().map(|a| &a.ty).collect();
                    let mut seen = std::collections::BTreeSet::new();
                    let fixed: usize = params
                        .iter()
                        .map(|t| seeds.distinct(parsed, t, &mut seen))
                        .sum();
                    let total: usize = fixed
                        + ret_type
                            .iter()
                            .map(|t| seeds.distinct(parsed, t, &mut seen))
                            .sum::<usize>();
                    if total > 0 {
                        seeds.functions = true;
                    }
                    if total > fixed {
                        // A type parameter of its own comes first, and nothing
                        // fixes it from here: the call leaves it to `_`.
                        seeds.calls.insert(
                            parsed.text(*name).to_string(),
                            (total + generics.len(), fixed + generics.len()),
                        );
                    }
                }
            }
        }
        seeds
    }

    fn count(&self, parsed: &Parsed, ty: &Type) -> usize {
        let name = parsed.text(ty.name);
        let own = if ty.code.is_some() || ty.is_tuple {
            0
        } else if is_map(name) {
            1
        } else {
            self.types.get(last(name)).copied().unwrap_or(0)
        };
        own + ty
            .generics
            .iter()
            .map(|g| self.count(parsed, g))
            .sum::<usize>()
    }

    /// The parameters a type needs beyond those `seen` already has: a type
    /// written twice in one item is one position's worth.
    fn distinct(
        &self,
        parsed: &Parsed,
        ty: &Type,
        seen: &mut std::collections::BTreeSet<String>,
    ) -> usize {
        let name = parsed.text(ty.name);
        let own = if ty.code.is_some() || ty.is_tuple {
            0
        } else if is_map(name) {
            1
        } else {
            self.types.get(last(name)).copied().unwrap_or(0)
        };
        if own > 0 && !seen.insert(self.key(parsed, ty, true)) {
            return 0;
        }
        own + ty
            .generics
            .iter()
            .map(|g| self.distinct(parsed, g, seen))
            .sum::<usize>()
    }

    fn key(&self, parsed: &Parsed, ty: &Type, top: bool) -> String {
        let name = last(parsed.text(ty.name));
        let name = match name {
            "TrustedMap" => "HashMap",
            "TrustedSet" => "HashSet",
            other => other,
        };
        let inner: Vec<String> = ty
            .generics
            .iter()
            .map(|g| self.key(parsed, g, false))
            .collect();
        let flags = match top {
            true => String::new(),
            false => format!(
                "{}{}{}",
                if ty.is_view { "&" } else { "" },
                if ty.is_nullable { "?" } else { "" },
                if ty.is_slice { "[]" } else { "" }
            ),
        };
        format!("{flags}{name}<{}>", inner.join(","))
    }

    /// Whether a type mentions a `pub` struct that declares parameters.
    pub fn mentions_a_struct(&self, parsed: &Parsed, ty: &Type) -> bool {
        let name = parsed.text(ty.name);
        (!ty.is_tuple && ty.code.is_none() && !is_map(name) && self.arity(name) > 0)
            || ty
                .generics
                .iter()
                .any(|g| self.mentions_a_struct(parsed, g))
    }

    /// The positions of a type, for the item that declares them.
    pub fn positions(&self, parsed: &Parsed, ty: &Type) -> usize {
        self.count(parsed, ty)
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty() && !self.functions
    }

    /// The `pub` functions a call has to fill, and how many positions each has.
    pub fn calls(&self) -> &BTreeMap<String, (usize, usize)> {
        &self.calls
    }

    /// Whether a `pub` struct of this name declares parameters.
    pub fn arity(&self, name: &str) -> usize {
        self.types.get(last(name)).copied().unwrap_or(0)
    }

    /// The declared parameters of an item, `n` of them from `first`.
    pub fn declared(first: usize, n: usize, defaulted: bool) -> Vec<String> {
        (first..first + n)
            .map(|at| match defaulted {
                true => format!("__S{at}: {SEED} = {KEYED}"),
                false => format!("__S{at}: {SEED}"),
            })
            .collect()
    }

    /// The same names, as arguments.
    pub fn applied(first: usize, n: usize) -> Vec<String> {
        (first..first + n).map(|at| format!("__S{at}")).collect()
    }

    /// One written type, with the parameters in. `fast` says what the package
    /// fills an ordinary mention with.
    pub fn write(&self, text: &str, place: &mut Place, fast: bool) -> String {
        if self.is_empty() && !place.declaring {
            return text.to_string();
        }
        let mut out = String::with_capacity(text.len() + 16);
        let bytes = text.as_bytes();
        let mut at = 0;
        self.run(text, bytes, &mut at, &mut out, place, fast, false);
        out
    }

    /// Copies until the end, or until the `>` that closes the arguments being
    /// read.
    #[allow(clippy::too_many_arguments)]
    fn run(
        &self,
        text: &str,
        bytes: &[u8],
        at: &mut usize,
        out: &mut String,
        place: &mut Place,
        fast: bool,
        inside: bool,
    ) {
        while *at < bytes.len() {
            let c = bytes[*at];
            if inside && c == b'>' {
                return;
            }
            if c == b'-' && bytes.get(*at + 1) == Some(&b'>') {
                out.push_str("->");
                *at += 2;
                continue;
            }
            if c == b'\'' {
                // A lifetime: the tick and its name.
                out.push('\'');
                *at += 1;
                while *at < bytes.len()
                    && (bytes[*at].is_ascii_alphanumeric() || bytes[*at] == b'_')
                {
                    out.push(bytes[*at] as char);
                    *at += 1;
                }
                continue;
            }
            if c.is_ascii_alphabetic() || c == b'_' {
                let start = *at;
                loop {
                    match bytes.get(*at) {
                        Some(b) if b.is_ascii_alphanumeric() || *b == b'_' => *at += 1,
                        Some(b':')
                            if bytes.get(*at + 1) == Some(&b':')
                                && bytes
                                    .get(*at + 2)
                                    .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_') =>
                        {
                            *at += 2
                        }
                        _ => break,
                    }
                }
                let path = &text[start..*at];
                self.mention(text, bytes, at, out, path, place, fast);
                continue;
            }
            out.push(c as char);
            // Anything that is not ASCII is copied whole.
            if c >= 0x80 {
                out.pop();
                let rest = &text[*at..];
                let ch = rest.chars().next().unwrap_or(' ');
                out.push(ch);
                *at += ch.len_utf8();
                continue;
            }
            *at += 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn mention(
        &self,
        text: &str,
        bytes: &[u8],
        at: &mut usize,
        out: &mut String,
        path: &str,
        place: &mut Place,
        fast: bool,
    ) {
        let has_args = bytes.get(*at) == Some(&b'<');
        let map = is_map(path) && has_args;
        let k = if map || is_map(path) {
            0
        } else {
            self.arity(path)
        };
        if !map && k == 0 {
            out.push_str(path);
            return;
        }
        // The text of the whole mention, to know a type written before.
        let start = *at - path.len();
        let end = if has_args { closing(bytes, *at) } else { *at };
        if place.declaring {
            let key = format!("{}{}", last(&renamed(path)), &text[*at..end]);
            if let Some((_, written)) = place
                .seen
                .iter()
                .find(|(k, _)| *k == key && !within_hit(place, path))
            {
                out.push_str(written);
                *at = end;
                return;
            }
        }
        let before = out.len();
        // What this mention adds, decided before its arguments are read: the
        // numbers go in the order the types start.
        let mut added: Vec<String> = Vec::new();
        let mut name = path.to_string();
        if map {
            if place.declaring {
                added.push(format!("__S{}", place.next));
                place.next += 1;
                name = renamed(path);
            }
        } else {
            match &place.within {
                Some((target, params)) if last(path) == target.as_str() => {
                    added = params.clone();
                }
                _ if place.declaring => {
                    added = Seeds::applied(place.next, k);
                    place.next += k;
                }
                _ => {
                    let fill = if fast { FAST } else { KEYED };
                    added = vec![fill.to_string(); k];
                }
            }
        }
        out.push_str(&name);
        if has_args {
            out.push('<');
            *at += 1;
            self.run(text, bytes, at, out, place, fast, true);
            for a in &added {
                out.push_str(", ");
                out.push_str(a);
            }
            if bytes.get(*at) == Some(&b'>') {
                out.push('>');
                *at += 1;
            }
        } else if !added.is_empty() {
            out.push('<');
            out.push_str(&added.join(", "));
            out.push('>');
        }
        if place.declaring && !within_hit(place, path) {
            let key = format!("{}{}", last(&renamed(path)), &text[start + path.len()..end]);
            place.seen.push((key, out[before..].to_string()));
        }
    }
}

fn within_hit(place: &Place, path: &str) -> bool {
    place
        .within
        .as_ref()
        .is_some_and(|(target, _)| last(path) == target.as_str())
}

/// The index just after the `>` that closes the `<` at `open`.
fn closing(bytes: &[u8], open: usize) -> usize {
    let mut depth = 0usize;
    let mut at = open;
    while at < bytes.len() {
        match bytes[at] {
            b'<' => depth += 1,
            b'-' if bytes.get(at + 1) == Some(&b'>') => at += 1,
            b'>' => {
                depth -= 1;
                if depth == 0 {
                    return at + 1;
                }
            }
            _ => {}
        }
        at += 1;
    }
    bytes.len()
}

/// `TrustedMap` is `HashMap` with a hasher of its own.
fn renamed(path: &str) -> String {
    let head = &path[..path.len() - last(path).len()];
    let name = match last(path) {
        "TrustedMap" => "HashMap",
        "TrustedSet" => "HashSet",
        other => other,
    };
    format!("{head}{name}")
}
