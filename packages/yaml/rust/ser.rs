//! `Serializer`, `to_string` and `to_writer` (#1515; ADR-yaml-1 D10): serde's
//! `Serializer` over the native writer. serde_yaml 0.9.34's serializer
//! (David Tolnay, MIT OR Apache-2.0) with its state machine for enum tags
//! kept; where it hands libyaml's emitter an event, this builds a node of the
//! native tree, and each finished document is written by `native::Writer`,
//! which picks the styles libyaml picks.

use crate::error::{self, Error, ErrorImpl};
use crate::native;
use crate::value::tagged::{self, MaybeTag};
use serde::ser;
use std::fmt::Display;
use std::io;
use std::mem;

type Result<T, E = Error> = std::result::Result<T, E>;

/// A structure for serializing Rust values into YAML.
///
/// # Example
///
/// ```
/// use serde::Serialize;
/// use std::collections::BTreeMap;
///
/// let mut buffer = Vec::new();
/// let mut ser = nikaia_yaml::Serializer::new(&mut buffer);
///
/// let mut object = BTreeMap::new();
/// object.insert("k", 107);
/// object.serialize(&mut ser).unwrap();
///
/// object.insert("J", 74);
/// object.serialize(&mut ser).unwrap();
///
/// assert_eq!(buffer, b"k: 107\n---\nJ: 74\nk: 107\n");
/// ```
pub struct Serializer<W> {
    depth: usize,
    state: State,
    /// The collections open, innermost last.
    open: Vec<Open>,
    /// The root of the document, once it is complete.
    root: Option<Built>,
    out: native::Writer,
    writer: W,
}

enum State {
    NothingInParticular,
    CheckForTag,
    CheckForDuplicateTag,
    FoundTag(String),
    AlreadyTagged,
}

/// A node as the serializer builds it: its tag and what it holds, owned, so
/// the native nodes written from it can be views of it.
struct Built {
    tag: Option<String>,
    item: Item,
}

enum Item {
    Null,
    Bool(bool),
    Int(String),
    Float(String),
    /// A string, in the style the writer picks for it.
    Str(String),
    /// A `char`, single-quoted.
    Char(String),
    Seq(Vec<Built>),
    Map(Vec<(Built, Built)>),
}

/// A collection being built: its tag and its items so far; a mapping's key
/// waits for its value.
struct Open {
    tag: Option<String>,
    kind: OpenKind,
}

enum OpenKind {
    Seq(Vec<Built>),
    Map(Vec<(Built, Built)>, Option<Built>),
}

impl<W> Serializer<W>
where
    W: io::Write,
{
    /// Creates a new YAML serializer.
    pub fn new(writer: W) -> Self {
        Serializer {
            depth: 0,
            state: State::NothingInParticular,
            open: Vec::new(),
            root: None,
            out: native::writer(),
            writer,
        }
    }

    /// Calls [`.flush()`](io::Write::flush) on the underlying `io::Write`
    /// object.
    pub fn flush(&mut self) -> Result<()> {
        self.writer
            .flush()
            .map_err(|err| error::new(ErrorImpl::Io(err)))
    }

    /// Unwrap the underlying `io::Write` object from the `Serializer`.
    pub fn into_inner(mut self) -> Result<W> {
        let end = self.out.end();
        self.put(&end)?;
        self.flush()?;
        Ok(self.writer)
    }

    fn put(&mut self, text: &str) -> Result<()> {
        self.writer
            .write_all(text.as_bytes())
            .map_err(|err| error::new(ErrorImpl::Io(err)))
    }

    fn emit_scalar(&mut self, item: Item) -> Result<()> {
        self.flush_mapping_start()?;
        let tag = self.take_tag();
        self.value_start();
        self.add(Built { tag, item });
        self.value_end()
    }

    fn emit_sequence_start(&mut self) -> Result<()> {
        self.flush_mapping_start()?;
        self.value_start();
        let tag = self.take_tag();
        self.open.push(Open {
            tag,
            kind: OpenKind::Seq(Vec::new()),
        });
        Ok(())
    }

    fn emit_sequence_end(&mut self) -> Result<()> {
        self.close();
        self.value_end()
    }

    fn emit_mapping_start(&mut self) -> Result<()> {
        self.flush_mapping_start()?;
        self.value_start();
        let tag = self.take_tag();
        self.open.push(Open {
            tag,
            kind: OpenKind::Map(Vec::new(), None),
        });
        Ok(())
    }

    fn emit_mapping_end(&mut self) -> Result<()> {
        self.close();
        self.value_end()
    }

    /// The innermost open collection, closed and added to the one around it.
    fn close(&mut self) {
        if let Some(open) = self.open.pop() {
            let item = match open.kind {
                OpenKind::Seq(items) => Item::Seq(items),
                OpenKind::Map(entries, _) => Item::Map(entries),
            };
            self.add(Built {
                tag: open.tag,
                item,
            });
        }
    }

    fn add(&mut self, built: Built) {
        match self.open.last_mut() {
            None => self.root = Some(built),
            Some(Open {
                kind: OpenKind::Seq(items),
                ..
            }) => items.push(built),
            Some(Open {
                kind: OpenKind::Map(entries, key),
                ..
            }) => match key.take() {
                Some(key) => entries.push((key, built)),
                None => *key = Some(built),
            },
        }
    }

    fn value_start(&mut self) {
        self.depth += 1;
    }

    /// At the end of the root, the document is written.
    fn value_end(&mut self) -> Result<()> {
        self.depth -= 1;
        if self.depth == 0
            && let Some(root) = self.root.take()
        {
            let text = self.out.document(&native_node(&root));
            self.put(&text)?;
        }
        Ok(())
    }

    fn take_tag(&mut self) -> Option<String> {
        let state = mem::replace(&mut self.state, State::NothingInParticular);
        if let State::FoundTag(mut tag) = state {
            if !tag.starts_with('!') {
                tag.insert(0, '!');
            }
            Some(tag)
        } else {
            self.state = state;
            None
        }
    }

    fn flush_mapping_start(&mut self) -> Result<()> {
        if let State::CheckForTag = self.state {
            self.state = State::NothingInParticular;
            self.emit_mapping_start()?;
        } else if let State::CheckForDuplicateTag = self.state {
            self.state = State::NothingInParticular;
        }
        Ok(())
    }
}

/// The native node of `built`, its text views of it.
fn native_node(built: &Built) -> native::Node<'_, '_, '_, '_> {
    let node = match &built.item {
        Item::Null => native::node(native::Value::Null),
        Item::Bool(v) => native::node(native::Value::Bool(*v)),
        Item::Int(t) => native::node(native::Value::Int(t)),
        Item::Float(t) => native::node(native::Value::Float(t)),
        Item::Str(t) => native::string_node(t.clone().into()),
        Item::Char(t) => native::quoted_node(t.clone().into()),
        Item::Seq(items) => {
            native::node(native::Value::Seq(items.iter().map(native_node).collect()))
        }
        Item::Map(entries) => native::node(native::Value::Map(
            entries
                .iter()
                .map(|(key, value)| native::Entry {
                    key: native_node(key),
                    value: native_node(value),
                })
                .collect(),
        )),
    };
    match &built.tag {
        Some(tag) => native::tagged(node, tag.clone().into()),
        None => node,
    }
}

/// A float as serde_yaml writes it: `.inf`, `-.inf`, `.nan`, else the
/// shortest text that reads back as the same float, as `ryu` writes it.
fn float_text<F: Display + std::fmt::Debug>(
    v: F,
    infinite: bool,
    positive: bool,
    nan: bool,
) -> String {
    if infinite && positive {
        ".inf".to_owned()
    } else if infinite {
        "-.inf".to_owned()
    } else if nan {
        ".nan".to_owned()
    } else {
        format!("{v:?}")
    }
}

impl<W> ser::Serializer for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.emit_scalar(Item::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.emit_scalar(Item::Int(v.to_string()))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.emit_scalar(Item::Int(v.to_string()))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.emit_scalar(Item::Int(v.to_string()))
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.emit_scalar(Item::Int(v.to_string()))
    }

    fn serialize_i128(self, v: i128) -> Result<()> {
        self.emit_scalar(Item::Int(v.to_string()))
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.emit_scalar(Item::Int(v.to_string()))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.emit_scalar(Item::Int(v.to_string()))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.emit_scalar(Item::Int(v.to_string()))
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.emit_scalar(Item::Int(v.to_string()))
    }

    fn serialize_u128(self, v: u128) -> Result<()> {
        self.emit_scalar(Item::Int(v.to_string()))
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        let text = float_text(v, v.is_infinite(), v.is_sign_positive(), v.is_nan());
        self.emit_scalar(Item::Float(text))
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        let text = float_text(v, v.is_infinite(), v.is_sign_positive(), v.is_nan());
        self.emit_scalar(Item::Float(text))
    }

    fn serialize_char(self, value: char) -> Result<()> {
        self.emit_scalar(Item::Char(value.to_string()))
    }

    /// serde_yaml's style for a string, literal with a line break and
    /// single-quoted where it would read back as another kind, is the
    /// native writer's for a plain string node.
    fn serialize_str(self, value: &str) -> Result<()> {
        self.emit_scalar(Item::Str(value.to_owned()))
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<()> {
        Err(error::new(ErrorImpl::BytesUnsupported))
    }

    fn serialize_unit(self) -> Result<()> {
        self.emit_scalar(Item::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        if let State::FoundTag(_) = self.state {
            return Err(error::new(ErrorImpl::SerializeNestedEnum));
        }
        self.state = State::FoundTag(variant.to_owned());
        value.serialize(&mut *self)
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<V>(self, value: &V) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.emit_sequence_start()?;
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        self.emit_sequence_start()?;
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.emit_sequence_start()?;
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _enm: &'static str,
        _idx: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        if let State::FoundTag(_) = self.state {
            return Err(error::new(ErrorImpl::SerializeNestedEnum));
        }
        self.state = State::FoundTag(variant.to_owned());
        self.emit_sequence_start()?;
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        if len == Some(1) {
            self.state = if let State::FoundTag(_) = self.state {
                self.emit_mapping_start()?;
                State::CheckForDuplicateTag
            } else {
                State::CheckForTag
            };
        } else {
            self.emit_mapping_start()?;
        }
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.emit_mapping_start()?;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _enm: &'static str,
        _idx: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        if let State::FoundTag(_) = self.state {
            return Err(error::new(ErrorImpl::SerializeNestedEnum));
        }
        self.state = State::FoundTag(variant.to_owned());
        self.emit_mapping_start()?;
        Ok(self)
    }

    fn collect_str<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + Display,
    {
        let string = if let State::CheckForTag | State::CheckForDuplicateTag = self.state {
            match tagged::check_for_tag(value) {
                MaybeTag::NotTag(string) => string,
                MaybeTag::Tag(string) => {
                    return if let State::CheckForDuplicateTag = self.state {
                        Err(error::new(ErrorImpl::SerializeNestedEnum))
                    } else {
                        self.state = State::FoundTag(string);
                        Ok(())
                    };
                }
            }
        } else {
            value.to_string()
        };

        self.serialize_str(&string)
    }
}

impl<W> ser::SerializeSeq for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, elem: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        elem.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_sequence_end()
    }
}

impl<W> ser::SerializeTuple for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, elem: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        elem.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_sequence_end()
    }
}

impl<W> ser::SerializeTupleStruct for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, value: &V) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_sequence_end()
    }
}

impl<W> ser::SerializeTupleVariant for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, v: &V) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        v.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_sequence_end()
    }
}

impl<W> ser::SerializeMap for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.flush_mapping_start()?;
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
    where
        K: ?Sized + ser::Serialize,
        V: ?Sized + ser::Serialize,
    {
        key.serialize(&mut **self)?;
        let tagged = matches!(self.state, State::FoundTag(_));
        value.serialize(&mut **self)?;
        if tagged {
            self.state = State::AlreadyTagged;
        }
        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::CheckForTag = self.state {
            self.emit_mapping_start()?;
        }
        if !matches!(self.state, State::AlreadyTagged) {
            self.emit_mapping_end()?;
        }
        self.state = State::NothingInParticular;
        Ok(())
    }
}

impl<W> ser::SerializeStruct for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, key: &'static str, value: &V) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        ser::Serializer::serialize_str(&mut **self, key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_mapping_end()
    }
}

impl<W> ser::SerializeStructVariant for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, field: &'static str, v: &V) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        ser::Serializer::serialize_str(&mut **self, field)?;
        v.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_mapping_end()
    }
}

/// Serialize the given data structure as YAML into the IO stream.
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// return an error.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write,
    T: ?Sized + ser::Serialize,
{
    let mut serializer = Serializer::new(writer);
    value.serialize(&mut serializer)
}

/// Serialize the given data structure as a String of YAML.
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// return an error.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: ?Sized + ser::Serialize,
{
    let mut vec = Vec::with_capacity(128);
    to_writer(&mut vec, value)?;
    String::from_utf8(vec).map_err(|err| error::new(ErrorImpl::Message(err.to_string(), None)))
}
