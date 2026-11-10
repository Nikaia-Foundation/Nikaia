//! serde_yaml 0.9's `tests/test_error.rs`.

use crate::Case;
use nikaia_yaml::value::{Tag, TaggedValue};
use nikaia_yaml::{Deserializer, Value};
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Debug;

pub const CASES: &[Case] = &[
    ("error::test_scan_error", test_scan_error),
    ("error::test_incorrect_type", test_incorrect_type),
    (
        "error::test_incorrect_nested_type",
        test_incorrect_nested_type,
    ),
    ("error::test_empty", test_empty),
    ("error::test_missing_field", test_missing_field),
    ("error::test_unknown_anchor", test_unknown_anchor),
    (
        "error::test_ignored_unknown_anchor",
        test_ignored_unknown_anchor,
    ),
    ("error::test_bytes", test_bytes),
    ("error::test_two_documents", test_two_documents),
    (
        "error::test_second_document_syntax_error",
        test_second_document_syntax_error,
    ),
    ("error::test_missing_enum_tag", test_missing_enum_tag),
    (
        "error::test_serialize_nested_enum",
        test_serialize_nested_enum,
    ),
    (
        "error::test_deserialize_nested_enum",
        test_deserialize_nested_enum,
    ),
    ("error::test_variant_not_a_seq", test_variant_not_a_seq),
    (
        "error::test_struct_from_sequence",
        test_struct_from_sequence,
    ),
    ("error::test_bad_bool", test_bad_bool),
    ("error::test_bad_int", test_bad_int),
    ("error::test_bad_float", test_bad_float),
    ("error::test_bad_null", test_bad_null),
    ("error::test_short_tuple", test_short_tuple),
    ("error::test_long_tuple", test_long_tuple),
    ("error::test_invalid_scalar_type", test_invalid_scalar_type),
    (
        "error::test_infinite_recursion_objects",
        test_infinite_recursion_objects,
    ),
    (
        "error::test_infinite_recursion_arrays",
        test_infinite_recursion_arrays,
    ),
    (
        "error::test_infinite_recursion_newtype",
        test_infinite_recursion_newtype,
    ),
    (
        "error::test_finite_recursion_objects",
        test_finite_recursion_objects,
    ),
    (
        "error::test_finite_recursion_arrays",
        test_finite_recursion_arrays,
    ),
    ("error::test_billion_laughs", test_billion_laughs),
    ("error::test_duplicate_keys", test_duplicate_keys),
];

fn test_error<'de, T>(yaml: &'de str, expected: &str)
where
    T: Deserialize<'de> + Debug,
{
    let result = nikaia_yaml::from_str::<T>(yaml);
    assert_eq!(expected, result.unwrap_err().to_string());

    let mut deserializer = Deserializer::from_str(yaml);
    if let Some(first_document) = deserializer.next() {
        if deserializer.next().is_none() {
            let result = T::deserialize(first_document);
            assert_eq!(expected, result.unwrap_err().to_string());
        }
    }
}

fn test_scan_error() {
    let yaml = ">\n@";
    let expected = "found character that cannot start any token at line 2 column 1, while scanning for the next token";
    test_error::<Value>(yaml, expected);
}

fn test_incorrect_type() {
    let yaml = "---\nstr\n";
    let expected = "invalid type: string \"str\", expected i16 at line 2 column 1";
    test_error::<i16>(yaml, expected);
}

fn test_incorrect_nested_type() {
    #[derive(Deserialize, Debug)]
    pub struct A {
        #[allow(dead_code)]
        pub b: Vec<B>,
    }
    #[derive(Deserialize, Debug)]
    pub enum B {
        C(#[allow(dead_code)] C),
    }
    #[derive(Deserialize, Debug)]
    pub struct C {
        #[allow(dead_code)]
        pub d: bool,
    }
    let yaml = "b:\n  - !C\n    d: fase\n";
    let expected = "b[0].d: invalid type: string \"fase\", expected a boolean at line 3 column 8";
    test_error::<A>(yaml, expected);
}

fn test_empty() {
    let expected = "EOF while parsing a value";
    test_error::<String>("", expected);
}

fn test_missing_field() {
    #[derive(Deserialize, Debug)]
    pub struct Basic {
        #[allow(dead_code)]
        pub v: bool,
        #[allow(dead_code)]
        pub w: bool,
    }
    let yaml = "---\nv: true\n";
    let expected = "missing field `w` at line 2 column 1";
    test_error::<Basic>(yaml, expected);
}

fn test_unknown_anchor() {
    let yaml = "---\n*some\n";
    let expected = "unknown anchor at line 2 column 1";
    test_error::<String>(yaml, expected);
}

fn test_ignored_unknown_anchor() {
    #[derive(Deserialize, Debug)]
    pub struct Wrapper {
        #[allow(dead_code)]
        pub c: (),
    }
    let yaml = "b: [*a]\nc: ~\n";
    let expected = "unknown anchor at line 1 column 5";
    test_error::<Wrapper>(yaml, expected);
}

fn test_bytes() {
    let expected = "serialization and deserialization of bytes in YAML is not implemented";
    test_error::<&[u8]>("...", expected);
}

fn test_two_documents() {
    let yaml = "---\n0\n---\n1\n";
    let expected = "deserializing from YAML containing more than one document is not supported";
    test_error::<usize>(yaml, expected);
}

fn test_second_document_syntax_error() {
    let yaml = "---\n0\n---\n]\n";

    let mut de = Deserializer::from_str(yaml);
    let first_doc = de.next().unwrap();
    let result = <usize as serde::Deserialize>::deserialize(first_doc);
    assert_eq!(0, result.unwrap());

    let second_doc = de.next().unwrap();
    let result = <usize as serde::Deserialize>::deserialize(second_doc);
    let expected =
        "did not find expected node content at line 4 column 1, while parsing a block node";
    assert_eq!(expected, result.unwrap_err().to_string());
}

fn test_missing_enum_tag() {
    #[derive(Deserialize, Debug)]
    pub enum E {
        V(#[allow(dead_code)] usize),
    }
    let yaml = r#""V": 16
"other": 32
"#;
    let expected = "invalid type: map, expected a YAML tag starting with '!'";
    test_error::<E>(yaml, expected);
}

fn test_serialize_nested_enum() {
    #[derive(Serialize, Debug)]
    pub enum Outer {
        Inner(Inner),
    }
    #[derive(Serialize, Debug)]
    pub enum Inner {
        Newtype(usize),
        Tuple(usize, usize),
        Struct { x: usize },
    }

    let expected = "serializing nested enums in YAML is not supported yet";

    let e = Outer::Inner(Inner::Newtype(0));
    let error = nikaia_yaml::to_string(&e).unwrap_err();
    assert_eq!(error.to_string(), expected);

    let e = Outer::Inner(Inner::Tuple(0, 0));
    let error = nikaia_yaml::to_string(&e).unwrap_err();
    assert_eq!(error.to_string(), expected);

    let e = Outer::Inner(Inner::Struct { x: 0 });
    let error = nikaia_yaml::to_string(&e).unwrap_err();
    assert_eq!(error.to_string(), expected);

    let e = Value::Tagged(Box::new(TaggedValue {
        tag: Tag::new("Outer"),
        value: Value::Tagged(Box::new(TaggedValue {
            tag: Tag::new("Inner"),
            value: Value::Null,
        })),
    }));
    let error = nikaia_yaml::to_string(&e).unwrap_err();
    assert_eq!(error.to_string(), expected);
}

fn test_deserialize_nested_enum() {
    #[derive(Deserialize, Debug)]
    pub enum Outer {
        Inner(#[allow(dead_code)] Inner),
    }
    #[derive(Deserialize, Debug)]
    pub enum Inner {
        Variant(#[allow(dead_code)] Vec<usize>),
    }

    let yaml = "---\n!Inner []\n";
    let expected = "deserializing nested enum in Outer::Inner from YAML is not supported yet at line 2 column 1";
    test_error::<Outer>(yaml, expected);

    let yaml = "---\n!Variant []\n";
    let expected = "unknown variant `Variant`, expected `Inner`";
    test_error::<Outer>(yaml, expected);

    let yaml = "---\n!Inner !Variant []\n";
    let expected = "deserializing nested enum in Outer::Inner from YAML is not supported yet at line 2 column 1";
    test_error::<Outer>(yaml, expected);
}

fn test_variant_not_a_seq() {
    #[derive(Deserialize, Debug)]
    pub enum E {
        V(#[allow(dead_code)] usize),
    }
    let yaml = "---\n!V\nvalue: 0\n";
    let expected = "invalid type: map, expected usize at line 2 column 1";
    test_error::<E>(yaml, expected);
}

fn test_struct_from_sequence() {
    #[derive(Deserialize, Debug)]
    pub struct Struct {
        #[allow(dead_code)]
        pub x: usize,
        #[allow(dead_code)]
        pub y: usize,
    }
    let yaml = "[0, 0]\n";
    let expected = "invalid type: sequence, expected struct Struct";
    test_error::<Struct>(yaml, expected);
}

fn test_bad_bool() {
    let yaml = "---\n!!bool str\n";
    let expected = "invalid value: string \"str\", expected a boolean at line 2 column 1";
    test_error::<bool>(yaml, expected);
}

fn test_bad_int() {
    let yaml = "---\n!!int str\n";
    let expected = "invalid value: string \"str\", expected an integer at line 2 column 1";
    test_error::<i64>(yaml, expected);
}

fn test_bad_float() {
    let yaml = "---\n!!float str\n";
    let expected = "invalid value: string \"str\", expected a float at line 2 column 1";
    test_error::<f64>(yaml, expected);
}

fn test_bad_null() {
    let yaml = "---\n!!null str\n";
    let expected = "invalid value: string \"str\", expected null at line 2 column 1";
    test_error::<()>(yaml, expected);
}

fn test_short_tuple() {
    let yaml = "---\n[0, 0]\n";
    let expected = "invalid length 2, expected a tuple of size 3 at line 2 column 1";
    test_error::<(u8, u8, u8)>(yaml, expected);
}

fn test_long_tuple() {
    let yaml = "---\n[0, 0, 0]\n";
    let expected = "invalid length 3, expected sequence of 2 elements at line 2 column 1";
    test_error::<(u8, u8)>(yaml, expected);
}

fn test_invalid_scalar_type() {
    #[derive(Deserialize, Debug)]
    pub struct S {
        #[allow(dead_code)]
        pub x: [i32; 1],
    }

    let yaml = "x: ''\n";
    let expected = "x: invalid type: string \"\", expected an array of length 1 at line 1 column 4";
    test_error::<S>(yaml, expected);
}

fn test_infinite_recursion_objects() {
    #[derive(Deserialize, Debug)]
    pub struct S {
        #[allow(dead_code)]
        pub x: Option<Box<S>>,
    }

    let yaml = "&a {'x': *a}";
    let expected = "recursion limit exceeded";
    test_error::<S>(yaml, expected);
}

fn test_infinite_recursion_arrays() {
    #[derive(Deserialize, Debug)]
    pub struct S(
        #[allow(dead_code)] pub usize,
        #[allow(dead_code)] pub Option<Box<S>>,
    );

    let yaml = "&a [0, *a]";
    let expected = "recursion limit exceeded";
    test_error::<S>(yaml, expected);
}

fn test_infinite_recursion_newtype() {
    #[derive(Deserialize, Debug)]
    pub struct S(#[allow(dead_code)] pub Option<Box<S>>);

    let yaml = "&a [*a]";
    let expected = "recursion limit exceeded";
    test_error::<S>(yaml, expected);
}

fn test_finite_recursion_objects() {
    #[derive(Deserialize, Debug)]
    pub struct S {
        #[allow(dead_code)]
        pub x: Option<Box<S>>,
    }

    let yaml = "{'x':".repeat(1_000) + &"}".repeat(1_000);
    let expected = "recursion limit exceeded at line 1 column 641";
    test_error::<S>(&yaml, expected);
}

fn test_finite_recursion_arrays() {
    #[derive(Deserialize, Debug)]
    pub struct S(
        #[allow(dead_code)] pub usize,
        #[allow(dead_code)] pub Option<Box<S>>,
    );

    let yaml = "[0, ".repeat(1_000) + &"]".repeat(1_000);
    let expected = "recursion limit exceeded at line 1 column 513";
    test_error::<S>(&yaml, expected);
}

fn test_billion_laughs() {
    #[derive(Debug)]
    struct X;

    impl<'de> Visitor<'de> for X {
        type Value = X;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("exponential blowup")
        }

        fn visit_unit<E>(self) -> Result<X, E> {
            Ok(X)
        }

        fn visit_seq<S>(self, mut seq: S) -> Result<X, S::Error>
        where
            S: SeqAccess<'de>,
        {
            while let Some(X) = seq.next_element()? {}
            Ok(X)
        }
    }

    impl<'de> Deserialize<'de> for X {
        fn deserialize<D>(deserializer: D) -> Result<X, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(X)
        }
    }

    let yaml = "a: &a ~\nb: &b [*a,*a,*a,*a,*a,*a,*a,*a,*a]\nc: &c [*b,*b,*b,*b,*b,*b,*b,*b,*b]\nd: &d [*c,*c,*c,*c,*c,*c,*c,*c,*c]\ne: &e [*d,*d,*d,*d,*d,*d,*d,*d,*d]\nf: &f [*e,*e,*e,*e,*e,*e,*e,*e,*e]\ng: &g [*f,*f,*f,*f,*f,*f,*f,*f,*f]\nh: &h [*g,*g,*g,*g,*g,*g,*g,*g,*g]\ni: &i [*h,*h,*h,*h,*h,*h,*h,*h,*h]\n";
    let expected = "repetition limit exceeded";
    test_error::<BTreeMap<String, X>>(yaml, expected);
}

fn test_duplicate_keys() {
    let yaml = "---\nthing: true\nthing: false\n";
    let expected = "duplicate entry with key \"thing\" at line 2 column 1";
    test_error::<Value>(yaml, expected);

    let yaml = "---\nnull: true\n~: false\n";
    let expected = "duplicate entry with null key at line 2 column 1";
    test_error::<Value>(yaml, expected);

    let yaml = "---\n99: true\n99: false\n";
    let expected = "duplicate entry with key 99 at line 2 column 1";
    test_error::<Value>(yaml, expected);

    let yaml = "---\n{}: true\n{}: false\n";
    let expected = "duplicate entry in YAML map at line 2 column 1";
    test_error::<Value>(yaml, expected);
}
