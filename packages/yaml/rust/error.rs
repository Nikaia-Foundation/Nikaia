//! The facade's `Error` (ADR-yaml-1 D10): serde_yaml 0.9's messages, and the
//! line and column of the native error or of the node where reading stopped.

use crate::native;
use serde::{de, ser};
use std::error::Error as StdError;
use std::fmt::{self, Debug, Display};
use std::io;
use std::sync::Arc;

/// An error that happened while reading YAML into a Rust value.
pub struct Error(Box<ErrorImpl>);

/// `Result<T, nikaia_yaml::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// A place in the input: 0-based internally, as libyaml counts, 1-based in
/// every message and in `Location`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Mark {
    pub(crate) index: usize,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

impl Display for Mark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.line != 0 || self.column != 0 {
            write!(f, "line {} column {}", self.line + 1, self.column + 1)
        } else {
            write!(f, "position {}", self.index)
        }
    }
}

#[derive(Debug)]
pub(crate) enum ErrorImpl {
    /// A message from serde or from the facade, with the place and the path
    /// of the node it is about once known.
    Message(String, Option<Pos>),
    /// The native parser refused the input.
    Parse(String, Mark),
    /// A syntax error in libyaml's words: the problem, where, and what
    /// libyaml was doing, written after the place.
    Libyaml(&'static str, Mark, &'static str),
    EndOfStream,
    MoreThanOneDocument,
    RecursionLimitExceeded(Mark),
    RepetitionLimitExceeded,
    BytesUnsupported,
    UnknownAnchor(Mark),
    Io(io::Error),
    SerializeNestedEnum,
    ScalarInMerge,
    TaggedInMerge,
    ScalarInMergeElement,
    SequenceInMergeElement,
    EmptyTag,
    FailedToParseNumber,

    Shared(Arc<ErrorImpl>),
}

#[derive(Debug)]
pub(crate) struct Pos {
    mark: Mark,
    path: String,
}

/// Where an error happened: the byte offset, and the line and the column,
/// both from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    index: usize,
    line: usize,
    column: usize,
}

impl Location {
    /// The byte offset in the input.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The line, from 1.
    pub fn line(&self) -> usize {
        self.line
    }

    /// The column, from 1, in characters.
    pub fn column(&self) -> usize {
        self.column
    }

    fn from_mark(mark: Mark) -> Self {
        Location {
            index: mark.index,
            line: mark.line + 1,
            column: mark.column + 1,
        }
    }
}

impl Error {
    /// Where the error happened, where the input tells.
    pub fn location(&self) -> Option<Location> {
        self.0.mark().map(Location::from_mark)
    }
}

pub(crate) fn new(inner: ErrorImpl) -> Error {
    Error(Box::new(inner))
}

pub(crate) fn shared(shared: Arc<ErrorImpl>) -> Error {
    Error(Box::new(ErrorImpl::Shared(shared)))
}

impl Error {
    /// The error, to hand to each document of a stream that failed.
    pub(crate) fn shared(self) -> Arc<ErrorImpl> {
        if let ErrorImpl::Shared(err) = *self.0 {
            err
        } else {
            Arc::from(self.0)
        }
    }
}

/// The native parser's error: its message, and its place (line and column
/// from 1, in characters; the byte offset).
pub(crate) fn parse(e: &native::Error) -> Error {
    let mark = Mark {
        index: e.offset.max(0) as usize,
        line: (e.line - 1).max(0) as usize,
        column: (e.column - 1).max(0) as usize,
    };
    new(match e.kind {
        native::ErrorKind::DepthLimit => ErrorImpl::RecursionLimitExceeded(mark),
        native::ErrorKind::Reserved => ErrorImpl::Libyaml(
            "found character that cannot start any token",
            mark,
            "while scanning for the next token",
        ),
        native::ErrorKind::NoContent => ErrorImpl::Libyaml(
            "did not find expected node content",
            mark,
            "while parsing a block node",
        ),
        native::ErrorKind::UnknownAnchor => ErrorImpl::UnknownAnchor(mark),
        // **A scalar its core tag does not fit** (ADR-yaml-1 D5) is serde's
        // invalid value, as serde_yaml's deserializer of the tag says it.
        native::ErrorKind::InvalidTagged => ErrorImpl::Message(
            format!(
                "invalid value: string {:?}, expected {}",
                e.text.as_str(),
                expected_by(e.tag.as_str())
            ),
            Some(Pos {
                mark,
                path: ".".to_owned(),
            }),
        ),
        _ => ErrorImpl::Parse(e.message.as_str().to_owned(), mark),
    })
}

/// What serde_yaml's deserializer of a core tag expects.
fn expected_by(tag: &str) -> &'static str {
    match tag.rsplit(':').next() {
        Some("bool") => "a boolean",
        Some("int") => "an integer",
        Some("float") => "a float",
        Some("null") => "null",
        _ => "a string",
    }
}

/// Gives a message without a place the place and the path of the node it is
/// about.
pub(crate) fn fix_mark(mut error: Error, mark: Mark, path: crate::path::Path) -> Error {
    if let ErrorImpl::Message(_, none @ None) = error.0.as_mut() {
        *none = Some(Pos {
            mark,
            path: path.to_string(),
        });
    }
    error
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.0.source()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.message_no_mark(f)?;
        match self.0.mark() {
            Some(mark) if mark.line != 0 || mark.column != 0 => write!(f, " at {}", mark)?,
            // libyaml's own errors give the offset where they know no line.
            Some(mark) if mark.index != 0 && self.0.is_parse() => write!(f, " at {}", mark)?,
            _ => {}
        }
        if let Some(context) = self.0.context() {
            write!(f, ", {}", context)?;
        }
        Ok(())
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct MessageNoMark<'a>(&'a ErrorImpl);
        impl Display for MessageNoMark<'_> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                self.0.message_no_mark(f)
            }
        }
        f.write_str("Error(")?;
        Debug::fmt(&MessageNoMark(&self.0).to_string(), f)?;
        if let Some(mark) = self.0.mark() {
            write!(f, ", line: {}, column: {}", mark.line + 1, mark.column + 1)?;
        }
        f.write_str(")")
    }
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        new(ErrorImpl::Message(msg.to_string(), None))
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        new(ErrorImpl::Message(msg.to_string(), None))
    }
}

impl ErrorImpl {
    fn is_parse(&self) -> bool {
        match self {
            ErrorImpl::Parse(..) | ErrorImpl::Libyaml(..) => true,
            ErrorImpl::Shared(err) => err.is_parse(),
            _ => false,
        }
    }

    fn context(&self) -> Option<&'static str> {
        match self {
            ErrorImpl::Libyaml(_, _, context) => Some(context),
            ErrorImpl::Shared(err) => err.context(),
            _ => None,
        }
    }

    fn mark(&self) -> Option<Mark> {
        match self {
            ErrorImpl::Message(_, Some(Pos { mark, .. }))
            | ErrorImpl::Parse(_, mark)
            | ErrorImpl::Libyaml(_, mark, _)
            | ErrorImpl::RecursionLimitExceeded(mark)
            | ErrorImpl::UnknownAnchor(mark) => Some(*mark),
            ErrorImpl::Shared(err) => err.mark(),
            _ => None,
        }
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            ErrorImpl::Io(err) => err.source(),
            ErrorImpl::Shared(err) => err.source(),
            _ => None,
        }
    }

    fn message_no_mark(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorImpl::Message(msg, None) => f.write_str(msg),
            ErrorImpl::Message(msg, Some(Pos { path, .. })) => {
                if path != "." {
                    write!(f, "{}: ", path)?;
                }
                f.write_str(msg)
            }
            ErrorImpl::Parse(msg, _) => f.write_str(msg),
            ErrorImpl::Libyaml(problem, _, _) => f.write_str(problem),
            ErrorImpl::EndOfStream => f.write_str("EOF while parsing a value"),
            ErrorImpl::MoreThanOneDocument => f.write_str(
                "deserializing from YAML containing more than one document is not supported",
            ),
            ErrorImpl::RecursionLimitExceeded(_) => f.write_str("recursion limit exceeded"),
            ErrorImpl::RepetitionLimitExceeded => f.write_str("repetition limit exceeded"),
            ErrorImpl::BytesUnsupported => {
                f.write_str("serialization and deserialization of bytes in YAML is not implemented")
            }
            ErrorImpl::UnknownAnchor(_) => f.write_str("unknown anchor"),
            ErrorImpl::Io(err) => Display::fmt(err, f),
            ErrorImpl::SerializeNestedEnum => {
                f.write_str("serializing nested enums in YAML is not supported yet")
            }
            ErrorImpl::ScalarInMerge => {
                f.write_str("expected a mapping or list of mappings for merging, but found scalar")
            }
            ErrorImpl::TaggedInMerge => f.write_str("unexpected tagged value in merge"),
            ErrorImpl::ScalarInMergeElement => {
                f.write_str("expected a mapping for merging, but found scalar")
            }
            ErrorImpl::SequenceInMergeElement => {
                f.write_str("expected a mapping for merging, but found sequence")
            }
            ErrorImpl::EmptyTag => f.write_str("empty YAML tag is not allowed"),
            ErrorImpl::FailedToParseNumber => f.write_str("failed to parse YAML number"),
            ErrorImpl::Shared(err) => err.message_no_mark(f),
        }
    }
}
