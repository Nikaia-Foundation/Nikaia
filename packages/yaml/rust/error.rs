//! The facade's `Error` (ADR-yaml-1 D10): serde_yaml 0.9's messages, and the
//! line and column of the native error or of the node where reading stopped.

use crate::native;
use serde::de;
use std::fmt::{self, Debug, Display};

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
    EndOfStream,
    MoreThanOneDocument,
    RecursionLimitExceeded(Mark),
    RepetitionLimitExceeded,
    BytesUnsupported,
    UnknownAnchor(Mark),
    FailedToParseNumber,
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

/// The native parser's error: its message, and its place (line and column
/// from 1, in characters; the byte offset).
pub(crate) fn parse(e: &native::Error) -> Error {
    let mark = Mark {
        index: e.offset.max(0) as usize,
        line: (e.line - 1).max(0) as usize,
        column: (e.column - 1).max(0) as usize,
    };
    new(ErrorImpl::Parse(e.message.as_str().to_owned(), mark))
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

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.message_no_mark(f)?;
        if let Some(mark) = self.0.mark()
            && (mark.line != 0 || mark.column != 0)
        {
            write!(f, " at {}", mark)?;
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

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        new(ErrorImpl::Message(msg.to_string(), None))
    }
}

impl ErrorImpl {
    fn mark(&self) -> Option<Mark> {
        match self {
            ErrorImpl::Message(_, Some(Pos { mark, .. }))
            | ErrorImpl::Parse(_, mark)
            | ErrorImpl::RecursionLimitExceeded(mark)
            | ErrorImpl::UnknownAnchor(mark) => Some(*mark),
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
            ErrorImpl::FailedToParseNumber => f.write_str("failed to parse YAML number"),
        }
    }
}
