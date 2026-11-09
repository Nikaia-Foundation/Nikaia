//! serde_yaml 0.9's `tests/test_value.rs`, then the issue's own cases of
//! `Value` indexing, `Mapping` and `with::singleton_map`.

use crate::Case;
use nikaia_yaml::value::{Tag, TaggedValue};
use nikaia_yaml::{Mapping, Number, Value};
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};

pub const CASES: &[Case] = &[
    ("value::test_nan", test_nan),
    ("value::test_digits", test_digits),
    ("value::test_into_deserializer", test_into_deserializer),
    ("value::test_merge", test_merge),
    ("value::test_debug", test_debug),
    ("value::test_tagged", test_tagged),
    ("value::test_index", test_index),
    ("value::test_mapping", test_mapping),
    ("value::test_tagged_value", test_tagged_value),
    ("value::test_singleton_map", test_singleton_map),
    (
        "value::test_singleton_map_recursive",
        test_singleton_map_recursive,
    ),
];

fn test_nan() {
    let pos_nan = nikaia_yaml::from_str::<Value>(".nan").unwrap();
    assert!(pos_nan.is_f64());
    assert_eq!(pos_nan, pos_nan);

    let neg_fake_nan = nikaia_yaml::from_str::<Value>("-.nan").unwrap();
    assert!(neg_fake_nan.is_string());

    let significand_mask = 0xF_FFFF_FFFF_FFFF;
    let bits = (f64::NAN.copysign(1.0).to_bits() ^ significand_mask) | 1;
    let different_pos_nan = Value::Number(Number::from(f64::from_bits(bits)));
    assert_eq!(pos_nan, different_pos_nan);
}

fn test_digits() {
    let num_string = nikaia_yaml::from_str::<Value>("01").unwrap();
    assert!(num_string.is_string());
}

fn test_into_deserializer() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Test {
        first: String,
        second: u32,
    }

    let value = nikaia_yaml::from_str::<Value>("xyz").unwrap();
    let s = String::deserialize(value.into_deserializer()).unwrap();
    assert_eq!(s, "xyz");

    let value = nikaia_yaml::from_str::<Value>("- first\n- second\n- third").unwrap();
    let arr = Vec::<String>::deserialize(value.into_deserializer()).unwrap();
    assert_eq!(arr, &["first", "second", "third"]);

    let value = nikaia_yaml::from_str::<Value>("first: abc\nsecond: 99").unwrap();
    let test = Test::deserialize(value.into_deserializer()).unwrap();
    assert_eq!(
        test,
        Test {
            first: "abc".to_string(),
            second: 99
        }
    );
}

fn test_merge() {
    // From https://yaml.org/type/merge.html.
    let yaml = "---\n\
        - &CENTER { x: 1, y: 2 }\n\
        - &LEFT { x: 0, y: 2 }\n\
        - &BIG { r: 10 }\n\
        - &SMALL { r: 1 }\n\
        \n\
        # All the following maps are equal:\n\
        \n\
        - # Explicit keys\n  x: 1\n  y: 2\n  r: 10\n  label: center/big\n\
        \n\
        - # Merge one map\n  << : *CENTER\n  r: 10\n  label: center/big\n\
        \n\
        - # Merge multiple maps\n  << : [ *CENTER, *BIG ]\n  label: center/big\n\
        \n\
        - # Override\n  << : [ *BIG, *LEFT, *SMALL ]\n  x: 1\n  label: center/big\n";

    let mut value: Value = nikaia_yaml::from_str(yaml).unwrap();
    value.apply_merge().unwrap();
    for i in 5..=7 {
        assert_eq!(value[4], value[i]);
    }
}

fn test_debug() {
    let yaml = "'Null': ~\nBool: true\nNumber: 1\nString: ...\nSequence:\n  - true\n\
        EmptySequence: []\nEmptyMapping: {}\nTagged: !tag true\n";

    let value: Value = nikaia_yaml::from_str(yaml).unwrap();
    let debug = format!("{:#?}", value);

    let expected = r#"Mapping {
    "Null": Null,
    "Bool": Bool(true),
    "Number": Number(1),
    "String": String("..."),
    "Sequence": Sequence [
        Bool(true),
    ],
    "EmptySequence": Sequence [],
    "EmptyMapping": Mapping {},
    "Tagged": TaggedValue {
        tag: !tag,
        value: Bool(true),
    },
}"#;

    assert_eq!(debug, expected);
}

fn test_tagged() {
    #[derive(Serialize)]
    enum Enum {
        Variant(usize),
    }

    let value = nikaia_yaml::to_value(&Enum::Variant(0)).unwrap();

    let deserialized: nikaia_yaml::Value = nikaia_yaml::from_value(value.clone()).unwrap();
    assert_eq!(value, deserialized);

    let serialized = nikaia_yaml::to_value(&value).unwrap();
    assert_eq!(value, serialized);
}

/// Not in `test_value.rs`: `Index` by `usize`, `str` and `Value`, through
/// a tag, and `IndexMut` inserting a key.
fn test_index() {
    let mut value: Value = nikaia_yaml::from_str("a:\n  - x\n  - !t {b: 1}\n2: two\n").unwrap();
    assert_eq!(value["a"][0], Value::String("x".to_owned()));
    assert_eq!(value["a"][1]["b"], Value::Number(Number::from(1)));
    assert_eq!(value[Value::Number(Number::from(2))].as_str(), Some("two"));
    assert_eq!(value["missing"], Value::Null);
    assert_eq!(value["a"][9], Value::Null);
    assert!(value.get("a").unwrap().is_sequence());
    assert!(value.get(0).is_none());
    value["c"] = Value::Bool(true);
    assert_eq!(value["c"].as_bool(), Some(true));
}

fn test_mapping() {
    let mut mapping: Mapping = nikaia_yaml::from_str("b: 2\na: 1\n").unwrap();
    assert_eq!(mapping.len(), 2);
    assert!(mapping.contains_key("a"));
    assert_eq!(mapping.get("b").and_then(Value::as_u64), Some(2));
    let keys: Vec<&str> = mapping.keys().filter_map(Value::as_str).collect();
    assert_eq!(keys, ["b", "a"]);
    mapping.insert("c".into(), 3.into());
    *mapping.entry("a".into()).or_insert(Value::Null) = 10.into();
    assert_eq!(mapping["a"], Value::from(10));
    // `remove` is `IndexMap::swap_remove`, as in serde_yaml: `c` takes `b`'s place.
    assert_eq!(mapping.remove("b"), Some(Value::from(2)));
    let pairs: Vec<(String, u64)> = mapping
        .iter()
        .map(|(k, v)| (k.as_str().unwrap().to_owned(), v.as_u64().unwrap()))
        .collect();
    assert_eq!(pairs, [("c".to_owned(), 3), ("a".to_owned(), 10)]);
    mapping.retain(|k, _| k != "a");
    assert_eq!(mapping.len(), 1);
}

fn test_tagged_value() {
    let value: Value = nikaia_yaml::from_str("!Thing [1]\n").unwrap();
    let tagged = match &value {
        Value::Tagged(tagged) => tagged,
        other => panic!("not tagged: {:?}", other),
    };
    assert_eq!(tagged.tag, Tag::new("Thing"));
    assert_eq!(tagged.tag, "!Thing");
    assert_eq!(tagged.tag.to_string(), "!Thing");
    assert!(value.is_sequence());
    let built = Value::Tagged(Box::new(TaggedValue {
        tag: Tag::new("Thing"),
        value: Value::Sequence(vec![1.into()]),
    }));
    assert_eq!(value, built);

    #[derive(Deserialize, PartialEq, Debug)]
    enum E {
        Thing(Vec<u8>),
    }
    let e: E = nikaia_yaml::from_value(value).unwrap();
    assert_eq!(e, E::Thing(vec![1]));
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum Shape {
    Unit,
    Circle { r: u8 },
    Pair(u8, u8),
}

/// serde_yaml's doc example of `with::singleton_map`, both ways.
fn test_singleton_map() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Outer {
        #[serde(with = "nikaia_yaml::with::singleton_map")]
        shape: Shape,
    }

    let outer: Outer = nikaia_yaml::from_str("shape:\n  Circle:\n    r: 3\n").unwrap();
    assert_eq!(
        outer,
        Outer {
            shape: Shape::Circle { r: 3 }
        }
    );
    let outer: Outer = nikaia_yaml::from_str("shape: Unit\n").unwrap();
    assert_eq!(outer, Outer { shape: Shape::Unit });

    let value = nikaia_yaml::to_value(&Outer {
        shape: Shape::Pair(1, 2),
    })
    .unwrap();
    let expected: Value = nikaia_yaml::from_str("shape:\n  Pair: [1, 2]\n").unwrap();
    assert_eq!(value, expected);
    let back: Outer = nikaia_yaml::from_value(value).unwrap();
    assert_eq!(
        back,
        Outer {
            shape: Shape::Pair(1, 2)
        }
    );
}

fn test_singleton_map_recursive() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Inner {
        shapes: Vec<Shape>,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Outer {
        #[serde(with = "nikaia_yaml::with::singleton_map_recursive")]
        inner: Inner,
    }

    let yaml = "inner:\n  shapes:\n    - Unit\n    - Circle: {r: 1}\n";
    let outer: Outer = nikaia_yaml::from_str(yaml).unwrap();
    let expected = Outer {
        inner: Inner {
            shapes: vec![Shape::Unit, Shape::Circle { r: 1 }],
        },
    };
    assert_eq!(outer, expected);
    let value = nikaia_yaml::to_value(&expected).unwrap();
    assert_eq!(value, nikaia_yaml::from_str::<Value>(yaml).unwrap());

    let inner: Inner =
        nikaia_yaml::with::singleton_map_recursive::deserialize(value["inner"].clone()).unwrap();
    assert_eq!(inner, expected.inner);
}
