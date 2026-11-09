//! serde_yaml 0.9's `tests/test_de.rs`, `test_value.rs` and
//! `test_error.rs` over `nikaia_yaml` (the modules `value` and `error`), with
//! the strings of `indoc!` written out, and the issue's own cases. Only
//! `test_serialize_nested_enum` is left out: it needs `to_string` (#1515).
//! Each case prints `ok NAME` or `FAILED NAME: why`.

mod error;
mod value;

use nikaia_yaml::{Deserializer, Mapping, Number, Value};
use serde::Deserialize;
use serde::de::IgnoredAny;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::panic;

pub type Case = (&'static str, fn());

fn test_de<T>(yaml: &str, expected: &T)
where
    T: serde::de::DeserializeOwned + PartialEq + Debug,
{
    let deserialized: T = nikaia_yaml::from_str(yaml).unwrap();
    assert_eq!(*expected, deserialized);

    let value: Value = nikaia_yaml::from_str(yaml).unwrap();
    let deserialized = T::deserialize(&value).unwrap();
    assert_eq!(*expected, deserialized);

    let deserialized: T = nikaia_yaml::from_value(value).unwrap();
    assert_eq!(*expected, deserialized);

    nikaia_yaml::from_str::<IgnoredAny>(yaml).unwrap();

    let mut deserializer = Deserializer::from_str(yaml);
    let document = deserializer.next().unwrap();
    let deserialized = T::deserialize(document).unwrap();
    assert_eq!(*expected, deserialized);
    assert!(deserializer.next().is_none());
}

fn test_de_no_value<'de, T>(yaml: &'de str, expected: &T)
where
    T: Deserialize<'de> + PartialEq + Debug,
{
    let deserialized: T = nikaia_yaml::from_str(yaml).unwrap();
    assert_eq!(*expected, deserialized);
    nikaia_yaml::from_str::<Value>(yaml).unwrap();
    nikaia_yaml::from_str::<IgnoredAny>(yaml).unwrap();
}

fn test_de_seed<'de, T, S>(yaml: &'de str, seed: S, expected: &T)
where
    T: PartialEq + Debug,
    S: serde::de::DeserializeSeed<'de, Value = T>,
{
    let deserialized: T = seed.deserialize(Deserializer::from_str(yaml)).unwrap();
    assert_eq!(*expected, deserialized);

    nikaia_yaml::from_str::<Value>(yaml).unwrap();
    nikaia_yaml::from_str::<IgnoredAny>(yaml).unwrap();
}

fn test_borrowed() {
    let yaml = "- plain nonàscii\n- 'single quoted'\n- \"double quoted\"\n";
    let expected = vec!["plain nonàscii", "single quoted", "double quoted"];
    test_de_no_value(yaml, &expected);
}

fn test_alias() {
    let yaml = "first:\n  &alias\n  1\nsecond:\n  *alias\nthird: 3\n";
    let mut expected = BTreeMap::new();
    expected.insert("first".to_owned(), 1);
    expected.insert("second".to_owned(), 1);
    expected.insert("third".to_owned(), 3);
    test_de(yaml, &expected);
}

fn test_option() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Data {
        a: Option<f64>,
        b: Option<String>,
        c: Option<bool>,
    }
    let yaml = "b:\nc: true\n";
    let expected = Data {
        a: None,
        b: None,
        c: Some(true),
    };
    test_de(yaml, &expected);
}

fn test_option_alias() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Data {
        a: Option<f64>,
        b: Option<String>,
        c: Option<bool>,
        d: Option<f64>,
        e: Option<String>,
        f: Option<bool>,
    }
    let yaml = "none_f:\n  &none_f\n  ~\nnone_s:\n  &none_s\n  ~\nnone_b:\n  &none_b\n  ~\n\n\
        some_f:\n  &some_f\n  1.0\nsome_s:\n  &some_s\n  x\nsome_b:\n  &some_b\n  true\n\n\
        a: *none_f\nb: *none_s\nc: *none_b\nd: *some_f\ne: *some_s\nf: *some_b\n";
    let expected = Data {
        a: None,
        b: None,
        c: None,
        d: Some(1.0),
        e: Some("x".to_owned()),
        f: Some(true),
    };
    test_de(yaml, &expected);
}

fn test_enum_alias() {
    #[derive(Deserialize, PartialEq, Debug)]
    enum E {
        A,
        B(u8, u8),
    }
    #[derive(Deserialize, PartialEq, Debug)]
    struct Data {
        a: E,
        b: E,
    }
    let yaml =
        "aref:\n  &aref\n  A\nbref:\n  &bref\n  !B\n    - 1\n    - 2\n\na: *aref\nb: *bref\n";
    let expected = Data {
        a: E::A,
        b: E::B(1, 2),
    };
    test_de(yaml, &expected);
}

fn test_enum_representations() {
    #[derive(Deserialize, PartialEq, Debug)]
    enum Enum {
        Unit,
        Tuple(i32, i32),
        Struct { x: i32, y: i32 },
        String(String),
        Number(f64),
    }
    let yaml = "- Unit\n- 'Unit'\n- !Unit\n- !Unit ~\n- !Unit null\n- !Tuple [0, 0]\n\
        - !Tuple\n  - 0\n  - 0\n- !Struct {x: 0, y: 0}\n- !Struct\n  x: 0\n  y: 0\n\
        - !String '...'\n- !String ...\n- !Number 0\n";
    let expected = vec![
        Enum::Unit,
        Enum::Unit,
        Enum::Unit,
        Enum::Unit,
        Enum::Unit,
        Enum::Tuple(0, 0),
        Enum::Tuple(0, 0),
        Enum::Struct { x: 0, y: 0 },
        Enum::Struct { x: 0, y: 0 },
        Enum::String("...".to_owned()),
        Enum::String("...".to_owned()),
        Enum::Number(0.0),
    ];
    test_de(yaml, &expected);

    let yaml = "- !String\n";
    let expected = vec![Enum::String(String::new())];
    test_de_no_value(yaml, &expected);
}

fn test_number_as_string() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Num {
        value: String,
    }
    let yaml = "# Cannot be represented as u128\nvalue: 340282366920938463463374607431768211457\n";
    let expected = Num {
        value: "340282366920938463463374607431768211457".to_owned(),
    };
    test_de_no_value(yaml, &expected);
}

fn test_empty_string() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Struct {
        empty: String,
        tilde: String,
    }
    let yaml = "empty:\ntilde: ~\n";
    let expected = Struct {
        empty: String::new(),
        tilde: "~".to_owned(),
    };
    test_de_no_value(yaml, &expected);
}

fn test_i128_big() {
    let expected: i128 = i64::MIN as i128 - 1;
    assert_eq!(
        expected,
        nikaia_yaml::from_str::<i128>("-9223372036854775809\n").unwrap()
    );
    assert_eq!(
        expected,
        nikaia_yaml::from_str::<i128>("-0o1000000000000000000001\n").unwrap()
    );
}

fn test_u128_big() {
    let expected: u128 = u64::MAX as u128 + 1;
    assert_eq!(
        expected,
        nikaia_yaml::from_str::<u128>("18446744073709551616\n").unwrap()
    );
    assert_eq!(
        expected,
        nikaia_yaml::from_str::<u128>("0o2000000000000000000000\n").unwrap()
    );
}

fn test_number_alias_as_string() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Num {
        version: String,
        value: String,
    }
    let yaml = "version: &a 1.10\nvalue: *a\n";
    let expected = Num {
        version: "1.10".to_owned(),
        value: "1.10".to_owned(),
    };
    test_de_no_value(yaml, &expected);
}

fn test_de_mapping() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Data {
        pub substructure: Mapping,
    }
    let yaml = "substructure:\n  a: 'foo'\n  b: 'bar'\n";
    let mut expected = Data {
        substructure: Mapping::new(),
    };
    expected.substructure.insert(
        Value::String("a".to_owned()),
        Value::String("foo".to_owned()),
    );
    expected.substructure.insert(
        Value::String("b".to_owned()),
        Value::String("bar".to_owned()),
    );
    test_de(yaml, &expected);
}

fn test_byte_order_mark() {
    let yaml = "\u{feff}- 0\n";
    let expected = vec![0];
    test_de(yaml, &expected);
}

fn test_bomb() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Data {
        expected: String,
    }
    // This would deserialize an astronomical number of elements if we were
    // vulnerable.
    let mut yaml = String::from("a: &a ~\n");
    let names: Vec<char> = ('a'..='z').collect();
    for pair in names.windows(2) {
        let (before, name) = (pair[0], pair[1]);
        let aliases = vec![format!("*{}", before); 9].join(",");
        yaml.push_str(&format!("{}: &{} [{}]\n", name, name, aliases));
    }
    yaml.push_str("expected: string\n");
    let expected = Data {
        expected: "string".to_owned(),
    };
    assert_eq!(expected, nikaia_yaml::from_str::<Data>(&yaml).unwrap());
}

fn test_numbers() {
    let cases = [
        ("0xF0", "240"),
        ("+0xF0", "240"),
        ("-0xF0", "-240"),
        ("0o70", "56"),
        ("+0o70", "56"),
        ("-0o70", "-56"),
        ("0b10", "2"),
        ("+0b10", "2"),
        ("-0b10", "-2"),
        ("127", "127"),
        ("+127", "127"),
        ("-127", "-127"),
        (".inf", ".inf"),
        (".Inf", ".inf"),
        (".INF", ".inf"),
        ("-.inf", "-.inf"),
        ("-.Inf", "-.inf"),
        ("-.INF", "-.inf"),
        (".nan", ".nan"),
        (".NaN", ".nan"),
        (".NAN", ".nan"),
        ("0.1", "0.1"),
    ];
    for &(yaml, expected) in &cases {
        let value = nikaia_yaml::from_str::<Value>(yaml).unwrap();
        match value {
            Value::Number(number) => assert_eq!(number.to_string(), expected),
            _ => panic!("expected number. input={:?}, result={:?}", yaml, value),
        }
    }

    // NOT numbers.
    let cases = [
        "0127", "+0127", "-0127", "++.inf", "+-.inf", "++1", "+-1", "-+1", "--1", "0x+1", "0x-1",
        "-0x+1", "-0x-1", "++0x1", "+-0x1", "-+0x1", "--0x1",
    ];
    for yaml in &cases {
        let value = nikaia_yaml::from_str::<Value>(yaml).unwrap();
        match value {
            Value::String(string) => assert_eq!(string, *yaml),
            _ => panic!("expected string. input={:?}, result={:?}", yaml, value),
        }
    }
}

fn test_nan() {
    // There is no negative NaN in YAML.
    assert!(
        nikaia_yaml::from_str::<f32>(".nan")
            .unwrap()
            .is_sign_positive()
    );
    assert!(
        nikaia_yaml::from_str::<f64>(".nan")
            .unwrap()
            .is_sign_positive()
    );
}

fn test_stateful() {
    struct Seed(i64);

    impl<'de> serde::de::DeserializeSeed<'de> for Seed {
        type Value = i64;
        fn deserialize<D>(self, deserializer: D) -> Result<i64, D::Error>
        where
            D: serde::de::Deserializer<'de>,
        {
            struct Visitor(i64);
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = i64;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    write!(formatter, "an integer")
                }

                fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<i64, E> {
                    Ok(v * self.0)
                }

                fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<i64, E> {
                    Ok(v as i64 * self.0)
                }
            }

            deserializer.deserialize_any(Visitor(self.0))
        }
    }

    let cases = [("3", 5, 15), ("6", 7, 42), ("-5", 9, -45)];
    for &(yaml, seed, expected) in &cases {
        test_de_seed(yaml, Seed(seed), &expected);
    }
}

fn test_ignore_tag() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Data {
        struc: Struc,
        tuple: Tuple,
        newtype: Newtype,
        map: BTreeMap<char, usize>,
        vec: Vec<usize>,
    }
    #[derive(Deserialize, Debug, PartialEq)]
    struct Struc {
        x: usize,
    }
    #[derive(Deserialize, Debug, PartialEq)]
    struct Tuple(usize, usize);
    #[derive(Deserialize, Debug, PartialEq)]
    struct Newtype(usize);
    let yaml = "struc: !wat\n  x: 0\ntuple: !wat\n  - 0\n  - 0\nnewtype: !wat 0\n\
        map: !wat\n  x: 0\nvec: !wat\n  - 0\n";
    let expected = Data {
        struc: Struc { x: 0 },
        tuple: Tuple(0, 0),
        newtype: Newtype(0),
        map: BTreeMap::from([('x', 0)]),
        vec: vec![0],
    };
    test_de(yaml, &expected);
}

fn test_no_required_fields() {
    #[derive(Deserialize, PartialEq, Debug)]
    pub struct NoRequiredFields {
        optional: Option<usize>,
    }
    for document in ["", "# comment\n"] {
        let expected = NoRequiredFields { optional: None };
        let deserialized: NoRequiredFields = nikaia_yaml::from_str(document).unwrap();
        assert_eq!(expected, deserialized);

        let deserialized: Vec<String> = nikaia_yaml::from_str(document).unwrap();
        assert_eq!(Vec::<String>::new(), deserialized);

        let deserialized: BTreeMap<char, usize> = nikaia_yaml::from_str(document).unwrap();
        assert_eq!(BTreeMap::new(), deserialized);

        let deserialized: Option<String> = nikaia_yaml::from_str(document).unwrap();
        assert_eq!(None, deserialized);

        let deserialized: Value = nikaia_yaml::from_str(document).unwrap();
        assert_eq!(Value::Null, deserialized);
    }
}

fn test_empty_scalar() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Struct<T> {
        thing: T,
    }
    let yaml = "thing:\n";
    test_de(
        yaml,
        &Struct {
            thing: nikaia_yaml::Sequence::new(),
        },
    );
    test_de(
        yaml,
        &Struct {
            thing: Mapping::new(),
        },
    );
}

fn test_python_safe_dump() {
    #[derive(Deserialize, PartialEq, Debug)]
    struct Frob {
        foo: u32,
    }
    // PyYAML's `yaml.safe_dump({"foo": 7200}, default_style="|")`.
    let yaml = "\"foo\": !!int |-\n    7200\n";
    test_de(yaml, &Frob { foo: 7200 });
}

fn test_tag_resolution() {
    let yaml = "- null\n- Null\n- NULL\n- ~\n-\n- true\n- True\n- TRUE\n- false\n- False\n\
        - FALSE\n- y\n- Y\n- yes\n- Yes\n- YES\n- n\n- N\n- no\n- No\n- NO\n- on\n- On\n\
        - ON\n- off\n- Off\n- OFF\n";
    let mut expected = vec![Value::Null; 5];
    expected.extend([true, true, true, false, false, false].map(Value::Bool));
    for s in [
        "y", "Y", "yes", "Yes", "YES", "n", "N", "no", "No", "NO", "on", "On", "ON", "off", "Off",
        "OFF",
    ] {
        expected.push(Value::String(s.to_owned()));
    }
    test_de(yaml, &expected);
}

fn test_parse_number() {
    let n = "111".parse::<Number>().unwrap();
    assert_eq!(n, Number::from(111));
    let n = "-111".parse::<Number>().unwrap();
    assert_eq!(n, Number::from(-111));
    let n = "-1.1".parse::<Number>().unwrap();
    assert_eq!(n, Number::from(-1.1));
    let n = ".nan".parse::<Number>().unwrap();
    assert_eq!(n, Number::from(f64::NAN));
    assert!(n.as_f64().unwrap().is_sign_positive());
    let n = ".inf".parse::<Number>().unwrap();
    assert_eq!(n, Number::from(f64::INFINITY));
    let n = "-.inf".parse::<Number>().unwrap();
    assert_eq!(n, Number::from(f64::NEG_INFINITY));
    let err = "null".parse::<Number>().unwrap_err();
    assert_eq!(err.to_string(), "failed to parse YAML number");
    let err = " 1 ".parse::<Number>().unwrap_err();
    assert_eq!(err.to_string(), "failed to parse YAML number");
}

/// Not in `test_de.rs`: the issue's own cases, a stream that is not one
/// document and `Error::location`.
fn test_one_document() {
    let err = nikaia_yaml::from_str::<Value>("---\na: 1\n---\nb: 2\n").unwrap_err();
    assert_eq!(
        err.to_string(),
        "deserializing from YAML containing more than one document is not supported"
    );
    assert!(err.location().is_none());
}

fn test_location() {
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Data {
        a: u8,
        b: u8,
    }
    let err = nikaia_yaml::from_str::<Data>("a: 1\nb: x\n").unwrap_err();
    let location = err.location().unwrap();
    assert_eq!((location.line(), location.column()), (2, 4));
    println!("  {}", err);
    let err = nikaia_yaml::from_str::<Value>("a: [1\n").unwrap_err();
    let location = err.location().unwrap();
    println!("  {} ({}:{})", err, location.line(), location.column());
}

fn main() {
    panic::set_hook(Box::new(|_| {}));
    let de: &[Case] = &[
        ("test_borrowed", test_borrowed),
        ("test_alias", test_alias),
        ("test_option", test_option),
        ("test_option_alias", test_option_alias),
        ("test_enum_alias", test_enum_alias),
        ("test_enum_representations", test_enum_representations),
        ("test_number_as_string", test_number_as_string),
        ("test_empty_string", test_empty_string),
        ("test_i128_big", test_i128_big),
        ("test_u128_big", test_u128_big),
        ("test_number_alias_as_string", test_number_alias_as_string),
        ("test_de_mapping", test_de_mapping),
        ("test_byte_order_mark", test_byte_order_mark),
        ("test_bomb", test_bomb),
        ("test_numbers", test_numbers),
        ("test_nan", test_nan),
        ("test_stateful", test_stateful),
        ("test_ignore_tag", test_ignore_tag),
        ("test_no_required_fields", test_no_required_fields),
        ("test_empty_scalar", test_empty_scalar),
        ("test_python_safe_dump", test_python_safe_dump),
        ("test_tag_resolution", test_tag_resolution),
        ("test_parse_number", test_parse_number),
        ("test_one_document", test_one_document),
        ("test_location", test_location),
    ];
    let cases = de.iter().chain(value::CASES).chain(error::CASES);
    for (name, case) in cases {
        match panic::catch_unwind(case) {
            Ok(()) => println!("ok {}", name),
            Err(payload) => {
                let why = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                println!("FAILED {}: {}", name, why.replace('\n', " "));
            }
        }
    }
}
