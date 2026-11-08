//! The path to the node being read, like `dependencies.serde.typo1`, for
//! error messages (serde_yaml 0.9's `Path`).

use std::fmt::{self, Display};

#[derive(Copy, Clone)]
pub(crate) enum Path<'a> {
    Root,
    Seq { parent: &'a Path<'a>, index: usize },
    Map { parent: &'a Path<'a>, key: &'a str },
    Alias { parent: &'a Path<'a> },
    Unknown { parent: &'a Path<'a> },
}

impl Display for Path<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct Parent<'a>(&'a Path<'a>);

        impl Display for Parent<'_> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self.0 {
                    Path::Root => Ok(()),
                    path => write!(f, "{}.", path),
                }
            }
        }

        match self {
            Path::Root => f.write_str("."),
            Path::Seq { parent, index } => write!(f, "{}[{}]", parent, index),
            Path::Map { parent, key } => write!(f, "{}{}", Parent(parent), key),
            Path::Alias { parent } => write!(f, "{}", parent),
            Path::Unknown { parent } => write!(f, "{}?", Parent(parent)),
        }
    }
}
