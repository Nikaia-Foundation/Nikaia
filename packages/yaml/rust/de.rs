//! `from_str` (ADR-yaml-1 D10): serde's `Deserializer` over the native
//! parser's events, as serde_yaml 0.9 reads libyaml's. Aliases are followed
//! here, not expanded by the parser, so the repetition limit of serde_yaml
//! holds; plain scalars are resolved serde_yaml's way.

use crate::error::{self, Error, ErrorImpl, Mark, Result};
use crate::native;
use crate::path::Path;
use serde::de::value::StrDeserializer;
use serde::de::{self, Deserialize, DeserializeSeed, Expected, IgnoredAny, Unexpected, Visitor};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::num::ParseIntError;

const BOOL: &str = "tag:yaml.org,2002:bool";
const INT: &str = "tag:yaml.org,2002:int";
const FLOAT: &str = "tag:yaml.org,2002:float";
const NULL: &str = "tag:yaml.org,2002:null";

/// Reads a `T` from a stream of exactly one YAML document.
///
/// # Errors
///
/// Where the input is no YAML, holds more than one document, or does not
/// fit `T`.
pub fn from_str<'de, T>(s: &'de str) -> Result<T>
where
    T: Deserialize<'de>,
{
    from_slice(s.as_bytes())
}

/// Reads a `T` from a stream of exactly one YAML document, as bytes.
///
/// # Errors
///
/// As `from_str`.
pub fn from_slice<'de, T>(v: &'de [u8]) -> Result<T>
where
    T: Deserialize<'de>,
{
    let (document, more) = load(v)?;
    let mut pos = 0;
    let mut jumpcount = 0;
    let t = T::deserialize(&mut DeserializerFromEvents {
        document: &document,
        pos: &mut pos,
        jumpcount: &mut jumpcount,
        path: Path::Root,
        remaining_depth: 128,
        current_enum: None,
    })?;
    if let Some(mark) = document.error {
        return Err(error::new(ErrorImpl::UnknownAnchor(mark)));
    }
    if more {
        Err(error::new(ErrorImpl::MoreThanOneDocument))
    } else {
        Ok(t)
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Style {
    Plain,
    Quoted,
    Block,
}

struct Scalar<'de> {
    tag: Option<String>,
    /// Borrowed from the input where the text is written there as it reads.
    value: Cow<'de, str>,
    style: Style,
    literal: bool,
}

enum Event<'de> {
    Alias(usize),
    Scalar(Scalar<'de>),
    SequenceStart(Option<String>),
    SequenceEnd,
    MappingStart(Option<String>),
    MappingEnd,
    Void,
}

struct Document<'de> {
    events: Vec<(Event<'de>, Mark)>,
    /// An alias to no anchor ends the events here.
    error: Option<Mark>,
    /// The anchor of each alias: its id to its node's index in `events`.
    aliases: BTreeMap<usize, usize>,
}

/// The first document's events, and whether another document follows, as
/// serde_yaml's loader gives them: no document is one `Void` event.
fn load(input: &[u8]) -> Result<(Document<'_>, bool)> {
    let docs = native::documents(input, native::default_limits())
        .map_err(|thrown| error::parse(thrown.held()))?;
    let lines = Lines::new(input);
    let mut document = Document {
        events: Vec::new(),
        error: None,
        aliases: BTreeMap::new(),
    };
    let mut anchors: BTreeMap<&str, usize> = BTreeMap::new();
    let mut count = 0;
    let mut more = false;
    for event in native::events(&docs) {
        let mark = lines.mark(event.start as usize);
        let converted = match event.kind {
            native::EventKind::StreamStart | native::EventKind::StreamEnd => continue,
            native::EventKind::DocumentStart => {
                count += 1;
                if count > 1 {
                    more = true;
                    break;
                }
                continue;
            }
            native::EventKind::DocumentEnd => continue,
            _ if document.error.is_some() => continue,
            native::EventKind::Alias => match anchors.get(event.value) {
                Some(id) => Event::Alias(*id),
                None => {
                    document.error = Some(mark);
                    continue;
                }
            },
            native::EventKind::Scalar => Event::Scalar(scalar(&event)),
            native::EventKind::SequenceStart => Event::SequenceStart(event.tag.as_deref().map(String::from)),
            native::EventKind::SequenceEnd => Event::SequenceEnd,
            native::EventKind::MappingStart => Event::MappingStart(event.tag.as_deref().map(String::from)),
            native::EventKind::MappingEnd => Event::MappingEnd,
        };
        if let Some(anchor) = event.anchor {
            let id = anchors.len();
            anchors.insert(anchor, id);
            document.aliases.insert(id, document.events.len());
        }
        document.events.push((converted, mark));
    }
    if count == 0 {
        document.events.push((Event::Void, lines.mark(input.len())));
    }
    Ok((document, more))
}

/// A scalar event: its text decoded, borrowed where the input holds it as
/// it reads.
fn scalar<'de>(event: &native::Event<'de, 'de>) -> Scalar<'de> {
    let (style, literal) = match event.style {
        native::Style::Plain => (Style::Plain, false),
        native::Style::Single | native::Style::Double => (Style::Quoted, false),
        native::Style::Literal => (Style::Block, true),
        native::Style::Folded => (Style::Block, false),
    };
    let raw: &'de str = event.value;
    let value = match &event.text {
        // A block scalar's text is folded and chomped; one a core tag reads
        // as a number, a `bool` or null has no text, and is its trimmed raw.
        Some(text) if style == Style::Block => Cow::Owned(text.as_str().to_owned()),
        None if style == Style::Block => Cow::Borrowed(raw.trim()),
        None => Cow::Borrowed(raw),
        Some(text) if text == raw => Cow::Borrowed(raw),
        Some(text) => match borrowed_inside(raw, text, style) {
            Some(borrowed) => Cow::Borrowed(borrowed),
            None => Cow::Owned(text.as_str().to_owned()),
        },
    };
    Scalar {
        tag: event.tag.as_deref().map(String::from),
        value,
        style,
        literal,
    }
}

/// The text as written between the quotes of `raw`, where it reads so.
fn borrowed_inside<'de>(raw: &'de str, text: &str, style: Style) -> Option<&'de str> {
    let offset = if style == Style::Quoted { 1 } else { 0 };
    let end = raw.len().checked_sub(offset)?;
    let start = end.checked_sub(text.len())?;
    let inside = raw.get(start..end)?;
    (inside == text).then_some(inside)
}

/// Line starts of the input, to give a byte offset its line and column.
struct Lines {
    starts: Vec<usize>,
    input: Vec<bool>,
}

impl Lines {
    fn new(input: &[u8]) -> Self {
        let mut starts = vec![0];
        let mut i = 0;
        while i < input.len() {
            match input[i] {
                b'\n' => starts.push(i + 1),
                b'\r' if input.get(i + 1) != Some(&b'\n') => starts.push(i + 1),
                _ => {}
            }
            i += 1;
        }
        // Whether each byte starts a character, for columns in characters.
        let input = input.iter().map(|b| b & 0xC0 != 0x80).collect();
        Lines { starts, input }
    }

    fn mark(&self, index: usize) -> Mark {
        let index = index.min(self.input.len());
        let line = match self.starts.binary_search(&index) {
            Ok(line) => line,
            Err(next) => next - 1,
        };
        let column = self.input[self.starts[line]..index]
            .iter()
            .filter(|c| **c)
            .count();
        Mark {
            index,
            line,
            column,
        }
    }
}

struct DeserializerFromEvents<'de, 'document> {
    document: &'document Document<'de>,
    pos: &'document mut usize,
    jumpcount: &'document mut usize,
    path: Path<'document>,
    remaining_depth: u8,
    current_enum: Option<CurrentEnum<'document>>,
}

#[derive(Copy, Clone)]
struct CurrentEnum<'document> {
    name: Option<&'static str>,
    tag: &'document str,
}

impl<'de, 'document> DeserializerFromEvents<'de, 'document> {
    fn peek_event(&self) -> Result<&'document Event<'de>> {
        self.peek_event_mark().map(|(event, _mark)| event)
    }

    fn peek_event_mark(&self) -> Result<(&'document Event<'de>, Mark)> {
        match self.document.events.get(*self.pos) {
            Some((event, mark)) => Ok((event, *mark)),
            None => Err(match self.document.error {
                Some(mark) => error::new(ErrorImpl::UnknownAnchor(mark)),
                None => error::new(ErrorImpl::EndOfStream),
            }),
        }
    }

    fn next_event_mark(&mut self) -> Result<(&'document Event<'de>, Mark)> {
        self.peek_event_mark().map(|(event, mark)| {
            *self.pos += 1;
            self.current_enum = None;
            (event, mark)
        })
    }

    fn next_event(&mut self) -> Result<&'document Event<'de>> {
        self.next_event_mark().map(|(event, _mark)| event)
    }

    fn jump<'anchor>(
        &'anchor mut self,
        pos: &'anchor mut usize,
    ) -> Result<DeserializerFromEvents<'de, 'anchor>> {
        *self.jumpcount += 1;
        if *self.jumpcount > self.document.events.len() * 100 {
            return Err(error::new(ErrorImpl::RepetitionLimitExceeded));
        }
        match self.document.aliases.get(pos) {
            Some(found) => {
                *pos = *found;
                Ok(DeserializerFromEvents {
                    document: self.document,
                    pos,
                    jumpcount: self.jumpcount,
                    path: Path::Alias { parent: &self.path },
                    remaining_depth: self.remaining_depth,
                    current_enum: None,
                })
            }
            None => panic!("unresolved alias: {}", *pos),
        }
    }

    fn ignore_any(&mut self) -> Result<()> {
        enum Nest {
            Sequence,
            Mapping,
        }
        let mut stack = Vec::new();
        loop {
            match self.next_event()? {
                Event::Alias(_) | Event::Scalar(_) | Event::Void => {}
                Event::SequenceStart(_) => stack.push(Nest::Sequence),
                Event::MappingStart(_) => stack.push(Nest::Mapping),
                Event::SequenceEnd => match stack.pop() {
                    Some(Nest::Sequence) => {}
                    None | Some(Nest::Mapping) => panic!("unexpected end of sequence"),
                },
                Event::MappingEnd => match stack.pop() {
                    Some(Nest::Mapping) => {}
                    None | Some(Nest::Sequence) => panic!("unexpected end of mapping"),
                },
            }
            if stack.is_empty() {
                return Ok(());
            }
        }
    }

    fn visit_sequence<V>(&mut self, visitor: V, mark: Mark) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let (value, len) = self.recursion_check(mark, |de| {
            let mut seq = SeqAccess {
                empty: false,
                de,
                len: 0,
            };
            let value = visitor.visit_seq(&mut seq)?;
            Ok((value, seq.len))
        })?;
        self.end_sequence(len)?;
        Ok(value)
    }

    fn visit_mapping<V>(&mut self, visitor: V, mark: Mark) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let (value, len) = self.recursion_check(mark, |de| {
            let mut map = MapAccess {
                empty: false,
                de,
                len: 0,
                key: None,
            };
            let value = visitor.visit_map(&mut map)?;
            Ok((value, map.len))
        })?;
        self.end_mapping(len)?;
        Ok(value)
    }

    fn end_sequence(&mut self, len: usize) -> Result<()> {
        let total = {
            let mut seq = SeqAccess {
                empty: false,
                de: self,
                len,
            };
            while de::SeqAccess::next_element::<IgnoredAny>(&mut seq)?.is_some() {}
            seq.len
        };
        match self.next_event()? {
            Event::SequenceEnd | Event::Void => {}
            _ => panic!("expected a SequenceEnd event"),
        }
        if total == len {
            Ok(())
        } else {
            struct ExpectedSeq(usize);
            impl Expected for ExpectedSeq {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    if self.0 == 1 {
                        write!(f, "sequence of 1 element")
                    } else {
                        write!(f, "sequence of {} elements", self.0)
                    }
                }
            }
            Err(de::Error::invalid_length(total, &ExpectedSeq(len)))
        }
    }

    fn end_mapping(&mut self, len: usize) -> Result<()> {
        let total = {
            let mut map = MapAccess {
                empty: false,
                de: self,
                len,
                key: None,
            };
            while de::MapAccess::next_entry::<IgnoredAny, IgnoredAny>(&mut map)?.is_some() {}
            map.len
        };
        match self.next_event()? {
            Event::MappingEnd | Event::Void => {}
            _ => panic!("expected a MappingEnd event"),
        }
        if total == len {
            Ok(())
        } else {
            struct ExpectedMap(usize);
            impl Expected for ExpectedMap {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    if self.0 == 1 {
                        write!(f, "map containing 1 entry")
                    } else {
                        write!(f, "map containing {} entries", self.0)
                    }
                }
            }
            Err(de::Error::invalid_length(total, &ExpectedMap(len)))
        }
    }

    fn recursion_check<F: FnOnce(&mut Self) -> Result<T>, T>(
        &mut self,
        mark: Mark,
        f: F,
    ) -> Result<T> {
        let previous_depth = self.remaining_depth;
        self.remaining_depth = match previous_depth.checked_sub(1) {
            Some(depth) => depth,
            None => return Err(error::new(ErrorImpl::RecursionLimitExceeded(mark))),
        };
        let result = f(self);
        self.remaining_depth = previous_depth;
        result
    }
}

struct SeqAccess<'de, 'document, 'seq> {
    empty: bool,
    de: &'seq mut DeserializerFromEvents<'de, 'document>,
    len: usize,
}

impl<'de> de::SeqAccess<'de> for SeqAccess<'de, '_, '_> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.empty {
            return Ok(None);
        }
        match self.de.peek_event()? {
            Event::SequenceEnd | Event::Void => Ok(None),
            _ => {
                let mut element_de = DeserializerFromEvents {
                    document: self.de.document,
                    pos: self.de.pos,
                    jumpcount: self.de.jumpcount,
                    path: Path::Seq {
                        parent: &self.de.path,
                        index: self.len,
                    },
                    remaining_depth: self.de.remaining_depth,
                    current_enum: None,
                };
                self.len += 1;
                seed.deserialize(&mut element_de).map(Some)
            }
        }
    }
}

struct MapAccess<'de, 'document, 'map> {
    empty: bool,
    de: &'map mut DeserializerFromEvents<'de, 'document>,
    len: usize,
    key: Option<&'document str>,
}

impl<'de> de::MapAccess<'de> for MapAccess<'de, '_, '_> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.empty {
            return Ok(None);
        }
        match self.de.peek_event()? {
            Event::MappingEnd | Event::Void => Ok(None),
            Event::Scalar(scalar) => {
                self.len += 1;
                self.key = Some(&scalar.value);
                seed.deserialize(&mut *self.de).map(Some)
            }
            _ => {
                self.len += 1;
                self.key = None;
                seed.deserialize(&mut *self.de).map(Some)
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let mut value_de = DeserializerFromEvents {
            document: self.de.document,
            pos: self.de.pos,
            jumpcount: self.de.jumpcount,
            path: match self.key {
                Some(key) => Path::Map {
                    parent: &self.de.path,
                    key,
                },
                None => Path::Unknown {
                    parent: &self.de.path,
                },
            },
            remaining_depth: self.de.remaining_depth,
            current_enum: None,
        };
        seed.deserialize(&mut value_de)
    }
}

struct EnumAccess<'de, 'document, 'variant> {
    de: &'variant mut DeserializerFromEvents<'de, 'document>,
    name: Option<&'static str>,
    tag: &'document str,
}

impl<'de, 'variant> de::EnumAccess<'de> for EnumAccess<'de, '_, 'variant> {
    type Error = Error;
    type Variant = DeserializerFromEvents<'de, 'variant>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let str_de = StrDeserializer::<Error>::new(self.tag);
        let variant = seed.deserialize(str_de)?;
        let visitor = DeserializerFromEvents {
            document: self.de.document,
            pos: self.de.pos,
            jumpcount: self.de.jumpcount,
            path: self.de.path,
            remaining_depth: self.de.remaining_depth,
            current_enum: Some(CurrentEnum {
                name: self.name,
                tag: self.tag,
            }),
        };
        Ok((variant, visitor))
    }
}

impl<'de> de::VariantAccess<'de> for DeserializerFromEvents<'de, '_> {
    type Error = Error;

    fn unit_variant(mut self) -> Result<()> {
        Deserialize::deserialize(&mut self)
    }

    fn newtype_variant_seed<T>(mut self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut self)
    }

    fn tuple_variant<V>(mut self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(&mut self, visitor)
    }

    fn struct_variant<V>(mut self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_struct(&mut self, "", fields, visitor)
    }
}

struct UnitVariantAccess<'de, 'document, 'variant> {
    de: &'variant mut DeserializerFromEvents<'de, 'document>,
}

impl<'de> de::EnumAccess<'de> for UnitVariantAccess<'de, '_, '_> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        Ok((seed.deserialize(&mut *self.de)?, self))
    }
}

impl<'de> de::VariantAccess<'de> for UnitVariantAccess<'de, '_, '_> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        Err(de::Error::invalid_type(
            Unexpected::UnitVariant,
            &"newtype variant",
        ))
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(de::Error::invalid_type(
            Unexpected::UnitVariant,
            &"tuple variant",
        ))
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(de::Error::invalid_type(
            Unexpected::UnitVariant,
            &"struct variant",
        ))
    }
}

/// The scalar's text to the visitor, borrowed where it can be.
fn visit_text<'de, V>(visitor: V, scalar: &Scalar<'de>) -> Result<V::Value>
where
    V: Visitor<'de>,
{
    match &scalar.value {
        Cow::Borrowed(borrowed) => visitor.visit_borrowed_str(borrowed),
        Cow::Owned(owned) => visitor.visit_str(owned),
    }
}

fn visit_scalar<'de, V>(visitor: V, scalar: &Scalar<'de>, tagged_already: bool) -> Result<V::Value>
where
    V: Visitor<'de>,
{
    let v: &str = &scalar.value;
    if let (Some(tag), false) = (&scalar.tag, tagged_already) {
        if tag == BOOL {
            return match parse_bool(v) {
                Some(v) => visitor.visit_bool(v),
                None => Err(de::Error::invalid_value(Unexpected::Str(v), &"a boolean")),
            };
        } else if tag == INT {
            return match visit_int(visitor, v) {
                Ok(result) => result,
                Err(_) => Err(de::Error::invalid_value(Unexpected::Str(v), &"an integer")),
            };
        } else if tag == FLOAT {
            return match parse_f64(v) {
                Some(v) => visitor.visit_f64(v),
                None => Err(de::Error::invalid_value(Unexpected::Str(v), &"a float")),
            };
        } else if tag == NULL {
            return match parse_null(v) {
                Some(()) => visitor.visit_unit(),
                None => Err(de::Error::invalid_value(Unexpected::Str(v), &"null")),
            };
        } else if tag.starts_with('!') && scalar.style == Style::Plain {
            return visit_untagged_scalar(visitor, scalar);
        }
    } else if scalar.style == Style::Plain {
        return visit_untagged_scalar(visitor, scalar);
    }
    visit_text(visitor, scalar)
}

fn parse_null(scalar: &str) -> Option<()> {
    match scalar {
        "null" | "Null" | "NULL" | "~" => Some(()),
        _ => None,
    }
}

fn parse_bool(scalar: &str) -> Option<bool> {
    match scalar {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

fn parse_unsigned_int<T>(
    scalar: &str,
    from_str_radix: fn(&str, radix: u32) -> std::result::Result<T, ParseIntError>,
) -> Option<T> {
    let unpositive = scalar.strip_prefix('+').unwrap_or(scalar);
    for (prefix, radix) in [("0x", 16), ("0o", 8), ("0b", 2)] {
        if let Some(rest) = unpositive.strip_prefix(prefix) {
            if rest.starts_with(['+', '-']) {
                return None;
            }
            if let Ok(int) = from_str_radix(rest, radix) {
                return Some(int);
            }
        }
    }
    if unpositive.starts_with(['+', '-']) {
        return None;
    }
    if digits_but_not_number(scalar) {
        return None;
    }
    from_str_radix(unpositive, 10).ok()
}

fn parse_signed_int<T>(
    scalar: &str,
    from_str_radix: fn(&str, radix: u32) -> std::result::Result<T, ParseIntError>,
) -> Option<T> {
    let unpositive = if let Some(unpositive) = scalar.strip_prefix('+') {
        if unpositive.starts_with(['+', '-']) {
            return None;
        }
        unpositive
    } else {
        scalar
    };
    for (prefix, radix) in [("0x", 16), ("0o", 8), ("0b", 2)] {
        if let Some(rest) = unpositive.strip_prefix(prefix) {
            if rest.starts_with(['+', '-']) {
                return None;
            }
            if let Ok(int) = from_str_radix(rest, radix) {
                return Some(int);
            }
        }
        if let Some(rest) = scalar
            .strip_prefix('-')
            .and_then(|s| s.strip_prefix(prefix))
            && let Ok(int) = from_str_radix(&format!("-{}", rest), radix)
        {
            return Some(int);
        }
    }
    if digits_but_not_number(scalar) {
        return None;
    }
    from_str_radix(unpositive, 10).ok()
}

fn parse_negative_int<T>(
    scalar: &str,
    from_str_radix: fn(&str, radix: u32) -> std::result::Result<T, ParseIntError>,
) -> Option<T> {
    for (prefix, radix) in [("-0x", 16), ("-0o", 8), ("-0b", 2)] {
        if let Some(rest) = scalar.strip_prefix(prefix)
            && let Ok(int) = from_str_radix(&format!("-{}", rest), radix)
        {
            return Some(int);
        }
    }
    if digits_but_not_number(scalar) {
        return None;
    }
    from_str_radix(scalar, 10).ok()
}

pub(crate) fn parse_f64(scalar: &str) -> Option<f64> {
    let unpositive = if let Some(unpositive) = scalar.strip_prefix('+') {
        if unpositive.starts_with(['+', '-']) {
            return None;
        }
        unpositive
    } else {
        scalar
    };
    if let ".inf" | ".Inf" | ".INF" = unpositive {
        return Some(f64::INFINITY);
    }
    if let "-.inf" | "-.Inf" | "-.INF" = scalar {
        return Some(f64::NEG_INFINITY);
    }
    if let ".nan" | ".NaN" | ".NAN" = scalar {
        return Some(f64::NAN.copysign(1.0));
    }
    if let Ok(float) = unpositive.parse::<f64>()
        && float.is_finite()
    {
        return Some(float);
    }
    None
}

/// Leading zeros followed by digits are a string in YAML 1.2.
pub(crate) fn digits_but_not_number(scalar: &str) -> bool {
    let scalar = scalar.strip_prefix(['-', '+']).unwrap_or(scalar);
    scalar.len() > 1 && scalar.starts_with('0') && scalar[1..].bytes().all(|b| b.is_ascii_digit())
}

/// The integer to the visitor, or the visitor back where `v` is none.
pub(crate) fn visit_int<'de, V>(visitor: V, v: &str) -> std::result::Result<Result<V::Value>, V>
where
    V: Visitor<'de>,
{
    if let Some(int) = parse_unsigned_int(v, u64::from_str_radix) {
        return Ok(visitor.visit_u64(int));
    }
    if let Some(int) = parse_negative_int(v, i64::from_str_radix) {
        return Ok(visitor.visit_i64(int));
    }
    if let Some(int) = parse_unsigned_int(v, u128::from_str_radix) {
        return Ok(visitor.visit_u128(int));
    }
    if let Some(int) = parse_negative_int(v, i128::from_str_radix) {
        return Ok(visitor.visit_i128(int));
    }
    Err(visitor)
}

fn visit_untagged_scalar<'de, V>(visitor: V, scalar: &Scalar<'de>) -> Result<V::Value>
where
    V: Visitor<'de>,
{
    let v: &str = &scalar.value;
    if v.is_empty() || parse_null(v) == Some(()) {
        return visitor.visit_unit();
    }
    if let Some(boolean) = parse_bool(v) {
        return visitor.visit_bool(boolean);
    }
    let visitor = match visit_int(visitor, v) {
        Ok(result) => return result,
        Err(visitor) => visitor,
    };
    if !digits_but_not_number(v)
        && let Some(float) = parse_f64(v)
    {
        return visitor.visit_f64(float);
    }
    visit_text(visitor, scalar)
}

fn is_plain_or_tagged_literal_scalar(
    expected: &str,
    scalar: &Scalar,
    tagged_already: bool,
) -> bool {
    match (scalar.style, &scalar.tag, tagged_already) {
        (Style::Plain, _, _) => true,
        (Style::Block, Some(tag), false) if scalar.literal => tag == expected,
        _ => false,
    }
}

fn invalid_type(event: &Event, exp: &dyn Expected) -> Error {
    enum Void {}

    struct InvalidType<'a> {
        exp: &'a dyn Expected,
    }

    impl Visitor<'_> for InvalidType<'_> {
        type Value = Void;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.exp.fmt(f)
        }
    }

    match event {
        Event::Alias(_) => unreachable!(),
        Event::Scalar(scalar) => match visit_scalar(InvalidType { exp }, scalar, false) {
            Ok(void) => match void {},
            Err(invalid_type) => invalid_type,
        },
        Event::SequenceStart(_) => de::Error::invalid_type(Unexpected::Seq, exp),
        Event::MappingStart(_) => de::Error::invalid_type(Unexpected::Map, exp),
        Event::SequenceEnd => panic!("unexpected end of sequence"),
        Event::MappingEnd => panic!("unexpected end of mapping"),
        Event::Void => error::new(ErrorImpl::EndOfStream),
    }
}

/// A local tag as an enum's variant: `!Foo` gives `Foo`, `!` stays.
fn parse_tag(tag: &Option<String>) -> Option<&str> {
    let tag = tag.as_deref()?;
    let rest = tag.strip_prefix('!')?;
    Some(if rest.is_empty() { tag } else { rest })
}

/// Reads one number, typed, from a scalar that is plain or tagged and
/// literal, following aliases; `$parse` gives the number or none.
macro_rules! deserialize_number {
    ($name:ident, $tag:expr, $visit:ident, $parse:expr) => {
        fn $name<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            let tagged_already = self.current_enum.is_some();
            let (next, mark) = self.next_event_mark()?;
            loop {
                match next {
                    &Event::Alias(mut pos) => break self.jump(&mut pos)?.$name(visitor),
                    Event::Scalar(scalar)
                        if is_plain_or_tagged_literal_scalar($tag, scalar, tagged_already) =>
                    {
                        if let Some(n) = $parse(&*scalar.value) {
                            break visitor.$visit(n);
                        }
                    }
                    _ => {}
                }
                break Err(invalid_type(next, &visitor));
            }
            .map_err(|err| error::fix_mark(err, mark, self.path))
        }
    };
}

impl<'de> de::Deserializer<'de> for &mut DeserializerFromEvents<'de, '_> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let tagged_already = self.current_enum.is_some();
        let (next, mark) = self.next_event_mark()?;
        fn enum_tag(tag: &Option<String>, tagged_already: bool) -> Option<&str> {
            if tagged_already {
                return None;
            }
            parse_tag(tag)
        }
        let result = match next {
            &Event::Alias(mut pos) => self.jump(&mut pos)?.deserialize_any(visitor),
            Event::Scalar(scalar) => match enum_tag(&scalar.tag, tagged_already) {
                Some(tag) => {
                    *self.pos -= 1;
                    visitor.visit_enum(EnumAccess {
                        de: self,
                        name: None,
                        tag,
                    })
                }
                None => visit_scalar(visitor, scalar, tagged_already),
            },
            Event::SequenceStart(tag) => match enum_tag(tag, tagged_already) {
                Some(tag) => {
                    *self.pos -= 1;
                    visitor.visit_enum(EnumAccess {
                        de: self,
                        name: None,
                        tag,
                    })
                }
                None => self.visit_sequence(visitor, mark),
            },
            Event::MappingStart(tag) => match enum_tag(tag, tagged_already) {
                Some(tag) => {
                    *self.pos -= 1;
                    visitor.visit_enum(EnumAccess {
                        de: self,
                        name: None,
                        tag,
                    })
                }
                None => self.visit_mapping(visitor, mark),
            },
            Event::SequenceEnd => panic!("unexpected end of sequence"),
            Event::MappingEnd => panic!("unexpected end of mapping"),
            Event::Void => visitor.visit_none(),
        };
        // serde's errors know no place: give them the node's.
        result.map_err(|err| error::fix_mark(err, mark, self.path))
    }

    deserialize_number!(deserialize_bool, BOOL, visit_bool, parse_bool);
    deserialize_number!(deserialize_i64, INT, visit_i64, |v| parse_signed_int(
        v,
        i64::from_str_radix
    ));
    deserialize_number!(deserialize_i128, INT, visit_i128, |v| parse_signed_int(
        v,
        i128::from_str_radix
    ));
    deserialize_number!(deserialize_u64, INT, visit_u64, |v| parse_unsigned_int(
        v,
        u64::from_str_radix
    ));
    deserialize_number!(deserialize_u128, INT, visit_u128, |v| parse_unsigned_int(
        v,
        u128::from_str_radix
    ));
    deserialize_number!(deserialize_f64, FLOAT, visit_f64, parse_f64);

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_f64(visitor)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let (next, mark) = self.next_event_mark()?;
        match next {
            Event::Scalar(scalar) => visit_text(visitor, scalar),
            &Event::Alias(mut pos) => self.jump(&mut pos)?.deserialize_str(visitor),
            other => Err(invalid_type(other, &visitor)),
        }
        .map_err(|err: Error| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(error::new(ErrorImpl::BytesUnsupported))
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(error::new(ErrorImpl::BytesUnsupported))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let is_some = match self.peek_event()? {
            &Event::Alias(mut pos) => {
                *self.pos += 1;
                return self.jump(&mut pos)?.deserialize_option(visitor);
            }
            Event::Scalar(scalar) => {
                let tagged_already = self.current_enum.is_some();
                if scalar.style != Style::Plain {
                    true
                } else if let (Some(tag), false) = (&scalar.tag, tagged_already) {
                    if tag == NULL {
                        if let Some(()) = parse_null(&scalar.value) {
                            false
                        } else {
                            return Err(de::Error::invalid_value(
                                Unexpected::Str(&scalar.value),
                                &"null",
                            ));
                        }
                    } else {
                        true
                    }
                } else {
                    !scalar.value.is_empty() && parse_null(&scalar.value).is_none()
                }
            }
            Event::SequenceStart(_) | Event::MappingStart(_) => true,
            Event::SequenceEnd => panic!("unexpected end of sequence"),
            Event::MappingEnd => panic!("unexpected end of mapping"),
            Event::Void => false,
        };
        if is_some {
            visitor.visit_some(self)
        } else {
            *self.pos += 1;
            self.current_enum = None;
            visitor.visit_none()
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let tagged_already = self.current_enum.is_some();
        let (next, mark) = self.next_event_mark()?;
        match next {
            Event::Scalar(scalar) => {
                let is_null = if scalar.style != Style::Plain {
                    false
                } else if let (Some(tag), false) = (&scalar.tag, tagged_already) {
                    tag == NULL && parse_null(&scalar.value).is_some()
                } else {
                    scalar.value.is_empty() || parse_null(&scalar.value).is_some()
                };
                if is_null {
                    visitor.visit_unit()
                } else {
                    Err(de::Error::invalid_value(
                        Unexpected::Str(&scalar.value),
                        &"null",
                    ))
                }
            }
            &Event::Alias(mut pos) => self.jump(&mut pos)?.deserialize_unit(visitor),
            Event::Void => visitor.visit_unit(),
            other => Err(invalid_type(other, &visitor)),
        }
        .map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        let (_event, mark) = self.peek_event_mark()?;
        self.recursion_check(mark, |de| visitor.visit_newtype_struct(de))
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let (next, mark) = self.next_event_mark()?;
        match next {
            &Event::Alias(mut pos) => self.jump(&mut pos)?.deserialize_seq(visitor),
            Event::SequenceStart(_) => self.visit_sequence(visitor, mark),
            other => {
                if is_empty_node(other) {
                    visitor.visit_seq(SeqAccess {
                        empty: true,
                        de: self,
                        len: 0,
                    })
                } else {
                    Err(invalid_type(other, &visitor))
                }
            }
        }
        .map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let (next, mark) = self.next_event_mark()?;
        match next {
            &Event::Alias(mut pos) => self.jump(&mut pos)?.deserialize_map(visitor),
            Event::MappingStart(_) => self.visit_mapping(visitor, mark),
            other => {
                if is_empty_node(other) {
                    visitor.visit_map(MapAccess {
                        empty: true,
                        de: self,
                        len: 0,
                        key: None,
                    })
                } else {
                    Err(invalid_type(other, &visitor))
                }
            }
        }
        .map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let (next, mark) = self.peek_event_mark()?;
        let result = if let Some(current_enum) = self.current_enum {
            if let Event::Scalar(scalar) = next
                && !scalar.value.is_empty()
            {
                visitor.visit_enum(UnitVariantAccess { de: self })
            } else {
                let message = if let Some(name) = current_enum.name {
                    format!(
                        "deserializing nested enum in {}::{} from YAML is not supported yet",
                        name, current_enum.tag,
                    )
                } else {
                    format!(
                        "deserializing nested enum in !{} from YAML is not supported yet",
                        current_enum.tag,
                    )
                };
                Err(error::new(ErrorImpl::Message(message, None)))
            }
        } else {
            match next {
                &Event::Alias(mut pos) => {
                    *self.pos += 1;
                    self.jump(&mut pos)?
                        .deserialize_enum(name, variants, visitor)
                }
                Event::Scalar(scalar) => {
                    if let Some(tag) = parse_tag(&scalar.tag) {
                        return visitor.visit_enum(EnumAccess {
                            de: self,
                            name: Some(name),
                            tag,
                        });
                    }
                    visitor.visit_enum(UnitVariantAccess { de: self })
                }
                Event::MappingStart(tag) => {
                    if let Some(tag) = parse_tag(tag) {
                        return visitor.visit_enum(EnumAccess {
                            de: self,
                            name: Some(name),
                            tag,
                        });
                    }
                    let err =
                        de::Error::invalid_type(Unexpected::Map, &"a YAML tag starting with '!'");
                    Err(error::fix_mark(err, mark, self.path))
                }
                Event::SequenceStart(tag) => {
                    if let Some(tag) = parse_tag(tag) {
                        return visitor.visit_enum(EnumAccess {
                            de: self,
                            name: Some(name),
                            tag,
                        });
                    }
                    let err =
                        de::Error::invalid_type(Unexpected::Seq, &"a YAML tag starting with '!'");
                    Err(error::fix_mark(err, mark, self.path))
                }
                Event::SequenceEnd => panic!("unexpected end of sequence"),
                Event::MappingEnd => panic!("unexpected end of mapping"),
                Event::Void => Err(error::new(ErrorImpl::EndOfStream)),
            }
        };
        result.map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.ignore_any()?;
        visitor.visit_unit()
    }
}

/// Nothing, or an empty plain scalar: read as an empty sequence or mapping.
fn is_empty_node(event: &Event) -> bool {
    match event {
        Event::Void => true,
        Event::Scalar(scalar) => scalar.value.is_empty() && scalar.style == Style::Plain,
        _ => false,
    }
}
