//! The event stream of each file named on the command line, through libyaml
//! (`unsafe-libyaml`, serde_yaml 0.9's parser), in the bench's event form
//! (README, *The output*).
use std::collections::HashMap;
use std::ffi::CStr;
use std::fmt::Write as _;
use std::io::Write as _;
use std::mem::MaybeUninit;
use unsafe_libyaml as sys;

fn escaped(out: &mut String, t: &[u8]) {
    for c in String::from_utf8_lossy(t).chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
}

unsafe fn text(p: *const u8) -> Option<&'static [u8]> {
    if p.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(p.cast()) }.to_bytes())
    }
}

struct Ids {
    ids: HashMap<Vec<u8>, usize>,
    next: usize,
}

impl Ids {
    fn props(&mut self, out: &mut String, anchor: *const u8, tag: *const u8) {
        if let Some(a) = unsafe { text(anchor) } {
            self.ids.insert(a.to_vec(), self.next);
            let _ = write!(out, " &{}", self.next);
            self.next += 1;
        }
        if let Some(t) = unsafe { text(tag) } {
            out.push_str(" <");
            out.push_str(&String::from_utf8_lossy(t));
            out.push('>');
        }
    }
}

/// The file's events appended to `out`, or `Err` where libyaml refuses it.
fn events(input: &[u8], out: &mut String) -> Result<(), ()> {
    let mut ids = Ids {
        ids: HashMap::new(),
        next: 1,
    };
    let mut parser = MaybeUninit::<sys::yaml_parser_t>::uninit();
    let parser = parser.as_mut_ptr();
    unsafe {
        if sys::yaml_parser_initialize(parser).fail {
            return Err(());
        }
        sys::yaml_parser_set_encoding(parser, sys::YAML_UTF8_ENCODING);
        sys::yaml_parser_set_input_string(parser, input.as_ptr(), input.len() as u64);
    }
    let mut result = Ok(());
    loop {
        let mut event = MaybeUninit::<sys::yaml_event_t>::uninit();
        let event = event.as_mut_ptr();
        if unsafe { sys::yaml_parser_parse(parser, event) }.fail {
            result = Err(());
            break;
        }
        let e = unsafe { &*event };
        let mut done = false;
        unsafe {
            match e.type_ {
                sys::YAML_STREAM_START_EVENT => out.push_str("+STR"),
                sys::YAML_STREAM_END_EVENT => {
                    out.push_str("-STR");
                    done = true;
                }
                sys::YAML_DOCUMENT_START_EVENT => out.push_str(if e.data.document_start.implicit {
                    "+DOC"
                } else {
                    "+DOC ---"
                }),
                sys::YAML_DOCUMENT_END_EVENT => out.push_str("-DOC"),
                sys::YAML_SEQUENCE_START_EVENT => {
                    out.push_str("+SEQ");
                    let d = &e.data.sequence_start;
                    ids.props(out, d.anchor, d.tag);
                }
                sys::YAML_SEQUENCE_END_EVENT => out.push_str("-SEQ"),
                sys::YAML_MAPPING_START_EVENT => {
                    out.push_str("+MAP");
                    let d = &e.data.mapping_start;
                    ids.props(out, d.anchor, d.tag);
                }
                sys::YAML_MAPPING_END_EVENT => out.push_str("-MAP"),
                sys::YAML_ALIAS_EVENT => {
                    let a = text(e.data.alias.anchor).unwrap_or_default();
                    let _ = write!(out, "=ALI *{}", ids.ids.get(a).copied().unwrap_or(0));
                }
                sys::YAML_SCALAR_EVENT => {
                    let d = &e.data.scalar;
                    out.push_str("=VAL");
                    ids.props(out, d.anchor, d.tag);
                    out.push_str(match d.style {
                        sys::YAML_SINGLE_QUOTED_SCALAR_STYLE => " '",
                        sys::YAML_DOUBLE_QUOTED_SCALAR_STYLE => " \"",
                        sys::YAML_LITERAL_SCALAR_STYLE => " |",
                        sys::YAML_FOLDED_SCALAR_STYLE => " >",
                        _ => " :",
                    });
                    escaped(out, std::slice::from_raw_parts(d.value, d.length as usize));
                }
                _ => {}
            }
            sys::yaml_event_delete(event);
        }
        out.push('\n');
        if done {
            break;
        }
    }
    unsafe { sys::yaml_parser_delete(parser) };
    result
}

fn main() {
    let mut out = String::new();
    for path in std::env::args().skip(1) {
        let input = std::fs::read(&path).expect("a readable file");
        let mut one = String::new();
        match events(&input, &mut one) {
            Ok(()) => out.push_str(&one),
            Err(()) => out.push_str("error\n"),
        }
    }
    std::io::stdout().write_all(out.as_bytes()).unwrap();
}
