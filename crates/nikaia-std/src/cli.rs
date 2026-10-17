//! `std::cli` - the command line.

/// The arguments the program was started with, the program's own name first,
/// exactly as Part III describes it.
pub fn args() -> Args {
    Args(std::env::args())
}

/// The arguments, read in order. **A position is an `i64`**, as every count
/// in the language is: `cli::args().nth(at)` with `at: i64` is what the ledger
/// describes (`Args::nth`), and Rust's own `nth` takes a `usize`.
pub struct Args(std::env::Args);

impl Args {
    /// The argument at a position, or nothing where there is none (or the
    /// position is negative).
    // Not `Iterator::nth`: that one counts in `usize`.
    #[allow(clippy::should_implement_trait)]
    pub fn nth(&mut self, at: i64) -> Option<String> {
        let at = usize::try_from(at).ok()?;
        self.0.nth(at)
    }

    /// How many arguments are left to read, the program's name among them
    /// until it is read. A count is an `i64`, as every count is.
    pub fn len(&self) -> i64 {
        self.0.len() as i64
    }

    /// Whether no argument is left to read.
    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }
}

impl Iterator for Args {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

/// **What `main`'s parameters make of the words** (Part I 5.5,
/// [ADR-360](../../../docs/specification/adr/adr-360.md)): the lowering of a
/// `main` with parameters describes them here, reads the words with
/// [`Words::read`] before the body runs, and checks `main`'s preconditions
/// with [`Words::breach`] where one does not hold.
pub struct Interface {
    /// The `///` lines above `main`, for `--help`.
    pub about: &'static str,
    pub params: &'static [Param],
}

/// One parameter of `main`.
pub struct Param {
    /// As the program wrote it; an option's word writes `_` as `-`.
    pub name: &'static str,
    /// The type as the program wrote it, for the messages and the help.
    pub ty: &'static str,
    pub kind: Kind,
    /// An option's default as the help shows it; empty for a subject.
    pub default: &'static str,
    /// The preconditions of `main` that read it, as written.
    pub pre: &'static [&'static str],
}

/// How a parameter takes its words (ADR-360 D1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Positional and required.
    Subject,
    /// Positional, of type `T?`: after every required subject, and `null`
    /// where it is left out at the end.
    Maybe,
    /// The last subject, a `Vec[T]`: every positional word left.
    Rest,
    /// `--name value` or `--name=value`.
    Option,
    /// A `bool` option: a bare `--name` is `true`.
    Flag,
}

/// The words, sorted into the parameters, before any is converted.
pub struct Words {
    interface: &'static Interface,
    program: String,
    positional: Vec<String>,
    /// By parameter: the word an option was given, the last one where it was
    /// given twice.
    named: Vec<Option<String>>,
}

/// Why the words do not reach the body.
#[derive(Debug, PartialEq, Eq)]
pub enum Stop {
    /// `--help` or `-h`: the help, status 0.
    Help,
    /// A wrong word: the line, then the usage, status 2.
    Wrong(String),
}

impl Words {
    /// The words the program was started with, read into `interface`'s
    /// parameters. `--help` prints the help and ends the program with status
    /// 0; a wrong word prints one line and the usage to standard error and
    /// ends it with status 2 (ADR-360 D4, D5).
    pub fn read(interface: &'static Interface) -> Words {
        let mut args = std::env::args();
        let program = args.next().unwrap_or_default();
        let program = std::path::Path::new(&program)
            .file_name()
            .map_or(program.clone(), |name| name.to_string_lossy().into_owned());
        match Words::sort(interface, program.clone(), args) {
            Ok(words) => words,
            Err(Stop::Help) => {
                print!("{}", help(interface, &program));
                std::process::exit(0)
            }
            Err(Stop::Wrong(line)) => stop(interface, &program, &line),
        }
    }

    /// The words sorted into the parameters: D1's rules, and the counts.
    pub fn sort(
        interface: &'static Interface,
        program: String,
        args: impl Iterator<Item = String>,
    ) -> Result<Words, Stop> {
        let params = interface.params;
        let mut named: Vec<Option<String>> = vec![None; params.len()];
        let mut positional = Vec::new();
        let mut ended = false;
        let mut args = args.peekable();
        while let Some(word) = args.next() {
            if ended {
                positional.push(word);
                continue;
            }
            if word == "--" {
                ended = true;
                continue;
            }
            if word == "--help" || word == "-h" {
                return Err(Stop::Help);
            }
            if let Some(rest) = word.strip_prefix("--") {
                let (name, given) = match rest.split_once('=') {
                    Some((name, value)) => (name, Some(value.to_string())),
                    None => (rest, None),
                };
                let Some(at) = params.iter().position(|p| {
                    matches!(p.kind, Kind::Option | Kind::Flag) && p.name.replace('_', "-") == name
                }) else {
                    return Err(Stop::Wrong(format!("`--{name}` names no option.")));
                };
                let value = match (given, params[at].kind) {
                    (Some(value), _) => value,
                    (None, Kind::Flag) => "true".to_string(),
                    (None, _) => match args.next() {
                        Some(value) => value,
                        None => {
                            return Err(Stop::Wrong(format!(
                                "`--{name}` needs a value: {}.",
                                an(params[at].ty)
                            )));
                        }
                    },
                };
                named[at] = Some(value);
                continue;
            }
            // `-` and a digit is a value; another word that starts with `-`
            // would be a short flag, which no parameter is.
            if word.len() > 1
                && word.starts_with('-')
                && !word[1..].starts_with(|c: char| c.is_ascii_digit())
            {
                return Err(Stop::Wrong(format!("`{word}` names no option.")));
            }
            positional.push(word);
        }
        let subjects: Vec<&Param> = params.iter().filter(|p| p.kind == Kind::Subject).collect();
        if let Some(missing) = subjects.get(positional.len()) {
            return Err(Stop::Wrong(format!(
                "`{}` is missing: the program takes {} there.",
                missing.name,
                an(missing.ty)
            )));
        }
        let places = params
            .iter()
            .filter(|p| matches!(p.kind, Kind::Subject | Kind::Maybe))
            .count();
        if positional.len() > places && !params.iter().any(|p| p.kind == Kind::Rest) {
            return Err(Stop::Wrong(format!(
                "`{}` is one word too many.",
                positional[places]
            )));
        }
        Ok(Words {
            interface,
            program,
            positional,
            named,
        })
    }

    /// The `at`-th subject, converted.
    pub fn subject<T: Word>(&self, at: usize) -> T {
        let param = self.subject_param(at);
        self.converted(&self.positional[at], param.name, param.ty)
    }

    /// The `at`-th subject, a `T?`, converted, or nothing where it was left
    /// out.
    pub fn maybe<T: Word>(&self, at: usize) -> Option<T> {
        let param = self.subject_param(at);
        let word = self.positional.get(at)?;
        let ty = param.ty.strip_suffix('?').unwrap_or(param.ty);
        Some(self.converted(word, param.name, ty))
    }

    /// Every positional word from `from` on, each converted.
    pub fn rest<T: Word>(&self, from: usize) -> Vec<T> {
        let param = self.subject_param(from);
        let item = param
            .ty
            .strip_prefix("Vec[")
            .and_then(|t| t.strip_suffix(']'))
            .unwrap_or(param.ty);
        self.positional
            .iter()
            .skip(from)
            .map(|word| self.converted(word, param.name, item))
            .collect()
    }

    /// The option at parameter `at`, converted, or nothing where it was left
    /// out.
    pub fn option<T: Word>(&self, at: usize) -> Option<T> {
        let param = &self.interface.params[at];
        let word = self.named[at].as_ref()?;
        let shown = format!("--{}", param.name.replace('_', "-"));
        Some(self.converted(word, &shown, param.ty))
    }

    /// **A precondition of `main` that does not hold** (ADR-360 D4): the
    /// claim as written, with where it came from, and the values given.
    pub fn breach(&self, claim: &str, operands: &[(&str, Option<String>)]) -> ! {
        let values: Vec<String> = operands
            .iter()
            .map(|(name, value)| match value {
                Some(value) => format!("`{name}` is {value}"),
                None => format!("`{name}`"),
            })
            .collect();
        let line = format!(
            "{}, which breaks `main`'s precondition {claim}.",
            values.join(" and ")
        );
        stop(self.interface, &self.program, &line)
    }

    fn subject_param(&self, at: usize) -> &'static Param {
        self.interface
            .params
            .iter()
            .filter(|p| matches!(p.kind, Kind::Subject | Kind::Maybe | Kind::Rest))
            .nth(at)
            .expect("the lowering asks for a subject `main` has")
    }

    fn converted<T: Word>(&self, word: &str, name: &str, ty: &str) -> T {
        match T::from_word(word) {
            Some(value) => value,
            None => stop(
                self.interface,
                &self.program,
                &format!("`{name}` is {}, and `{word}` is not one.", an(ty)),
            ),
        }
    }
}

/// The line, the usage, status 2.
fn stop(interface: &Interface, program: &str, line: &str) -> ! {
    eprintln!("error: {line}");
    eprintln!("{}", usage(interface, program));
    eprintln!("For more, try `{program} --help`.");
    std::process::exit(2)
}

/// `an \`int\``, `a \`String\``.
fn an(ty: &str) -> String {
    let base = ty.trim_start_matches("Vec[");
    let article = match base.chars().next() {
        Some('a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U') => "an",
        _ => "a",
    };
    format!("{article} `{ty}`")
}

/// The short usage: `usage: prog [--times <int>] [--verbose] <a>`.
pub fn usage(interface: &Interface, program: &str) -> String {
    let mut text = format!("usage: {program}");
    for p in interface.params {
        let word = p.name.replace('_', "-");
        match p.kind {
            Kind::Option => text.push_str(&format!(" [--{word} <{}>]", p.ty)),
            Kind::Flag => text.push_str(&format!(" [--{word}]")),
            _ => {}
        }
    }
    for p in interface.params {
        match p.kind {
            Kind::Subject => text.push_str(&format!(" <{}>", p.name)),
            Kind::Maybe => text.push_str(&format!(" [<{}>]", p.name)),
            Kind::Rest => text.push_str(&format!(" [<{}>...]", p.name)),
            _ => {}
        }
    }
    text
}

/// **The help is the interface** (ADR-360 D5): the program's name, the `///`
/// lines above `main`, and each parameter with its type, default and
/// preconditions.
pub fn help(interface: &Interface, program: &str) -> String {
    let mut text = format!("{program}\n");
    if !interface.about.is_empty() {
        text.push_str(&format!("{}\n", interface.about.trim_end()));
    }
    text.push_str(&format!("\n{}\n", usage(interface, program)));
    let line = |p: &Param, head: String| {
        let mut line = format!("  {head:<20} {}", p.ty);
        if !p.default.is_empty() {
            line.push_str(&format!(", default {}", p.default));
        }
        for pre in p.pre {
            line.push_str(&format!("; requires {pre}"));
        }
        line.push('\n');
        line
    };
    let subjects: Vec<&Param> = interface
        .params
        .iter()
        .filter(|p| matches!(p.kind, Kind::Subject | Kind::Maybe | Kind::Rest))
        .collect();
    if !subjects.is_empty() {
        text.push_str("\narguments:\n");
        for p in subjects {
            let head = match p.kind {
                Kind::Maybe => format!("[<{}>]", p.name),
                _ => format!("<{}>", p.name),
            };
            text.push_str(&line(p, head));
        }
    }
    text.push_str("\noptions:\n");
    for p in interface.params {
        let word = p.name.replace('_', "-");
        match p.kind {
            Kind::Option => text.push_str(&line(p, format!("--{word} <{}>", p.ty))),
            Kind::Flag => text.push_str(&line(p, format!("--{word}"))),
            _ => {}
        }
    }
    text.push_str(&format!("  {:<20} prints this help\n", "-h, --help"));
    text
}

/// **What a word can be** (ADR-360 D2): a number in the shape `std` reads one
/// from text (Part III 17.1) that fits its type, `true` or `false`, or the
/// word itself.
pub trait Word: Sized {
    fn from_word(word: &str) -> Option<Self>;
}

/// An optional sign and decimal digits, as `text::parse_i64` reads them; an
/// unsigned type takes no `-`.
fn whole(word: &str, signed: bool) -> Option<i128> {
    let (negative, digits) = match word.as_bytes().first() {
        Some(b'-') if signed => (true, &word[1..]),
        Some(b'+') => (false, &word[1..]),
        _ => (false, word),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut value: i128 = 0;
    for b in digits.bytes() {
        value = value.checked_mul(10)?.checked_sub(i128::from(b - b'0'))?;
    }
    if negative {
        Some(value)
    } else {
        value.checked_neg()
    }
}

macro_rules! whole_word {
    ($($t:ty: $signed:expr),*) => {$(
        impl Word for $t {
            fn from_word(word: &str) -> Option<Self> {
                <$t>::try_from(whole(word, $signed)?).ok()
            }
        }
    )*};
}

whole_word!(i8: true, i16: true, i32: true, i64: true, i128: true, isize: true,
    u8: false, u16: false, u32: false, u64: false, usize: false);

impl Word for u128 {
    fn from_word(word: &str) -> Option<Self> {
        let digits = word.strip_prefix('+').unwrap_or(word);
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        digits.parse().ok()
    }
}

impl Word for f64 {
    fn from_word(word: &str) -> Option<Self> {
        crate::text::parse_f64(word)
    }
}

impl Word for bool {
    fn from_word(word: &str) -> Option<Self> {
        match word {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }
}

impl Word for String {
    fn from_word(word: &str) -> Option<Self> {
        Some(word.to_string())
    }
}

impl<T: Word> Word for Option<T> {
    fn from_word(word: &str) -> Option<Self> {
        T::from_word(word).map(Some)
    }
}

#[cfg(test)]
mod tests {
    static PARAMS: [super::Param; 4] = [
        super::Param {
            name: "a",
            ty: "i64",
            kind: super::Kind::Subject,
            default: "",
            pre: &["`a > 0`"],
        },
        super::Param {
            name: "times",
            ty: "i64",
            kind: super::Kind::Option,
            default: "4",
            pre: &[],
        },
        super::Param {
            name: "be_loud",
            ty: "bool",
            kind: super::Kind::Flag,
            default: "false",
            pre: &[],
        },
        super::Param {
            name: "rest",
            ty: "Vec[String]",
            kind: super::Kind::Rest,
            default: "",
            pre: &[],
        },
    ];
    static INTERFACE: super::Interface = super::Interface {
        about: "Multiplies.",
        params: &PARAMS,
    };

    fn sorted(words: &[&str]) -> Result<super::Words, super::Stop> {
        super::Words::sort(
            &INTERFACE,
            "prog".to_string(),
            words.iter().map(|w| w.to_string()),
        )
    }

    /// ADR-360 D1: positional and named words, `--`, `-3`.
    #[test]
    fn words_are_sorted_into_the_parameters() {
        let w = sorted(&["--times=3", "5", "--be-loud", "x"]).unwrap();
        assert_eq!(w.subject::<i64>(0), 5);
        assert_eq!(w.option::<i64>(1), Some(3));
        assert_eq!(w.option::<bool>(2), Some(true));
        assert_eq!(w.rest::<String>(1), vec!["x".to_string()]);
        let w = sorted(&["--times", "7", "--", "-3", "--be-loud"]).unwrap();
        assert_eq!(w.subject::<i64>(0), -3);
        assert_eq!(w.option::<i64>(1), Some(7));
        assert_eq!(w.option::<bool>(2), None);
        assert_eq!(w.rest::<String>(1), vec!["--be-loud".to_string()]);
        assert_eq!(sorted(&["-3"]).unwrap().subject::<i64>(0), -3);
    }

    /// ADR-360 D4, D5: a wrong word, and the help.
    #[test]
    fn wrong_words_stop_before_the_body() {
        assert!(
            matches!(sorted(&[]), Err(super::Stop::Wrong(line)) if line.contains("`a` is missing"))
        );
        assert!(
            matches!(sorted(&["--nope", "1"]), Err(super::Stop::Wrong(line)) if line.contains("`--nope` names no option"))
        );
        assert!(matches!(sorted(&["-v", "1"]), Err(super::Stop::Wrong(_))));
        assert!(
            matches!(sorted(&["1", "--times"]), Err(super::Stop::Wrong(line)) if line.contains("needs a value"))
        );
        assert_eq!(sorted(&["1", "-h"]).err(), Some(super::Stop::Help));
        let help = super::help(&INTERFACE, "prog");
        assert!(help.contains("Multiplies."), "{help}");
        assert!(help.contains("--be-loud"), "{help}");
        assert!(help.contains("requires `a > 0`"), "{help}");
        assert!(help.contains("default 4"), "{help}");
    }

    /// ADR-360 D2: a number in `std`'s shape that fits its type.
    #[test]
    fn a_word_is_a_value_of_its_type() {
        use super::Word;
        assert_eq!(i64::from_word("-12"), Some(-12));
        assert_eq!(i64::from_word("+12"), Some(12));
        assert_eq!(i64::from_word("1_2"), None);
        assert_eq!(u8::from_word("256"), None);
        assert_eq!(u8::from_word("-1"), None);
        assert_eq!(i64::from_word("-9223372036854775808"), Some(i64::MIN));
        assert_eq!(
            u128::from_word("340282366920938463463374607431768211455"),
            Some(u128::MAX)
        );
        assert_eq!(f64::from_word("2.5"), Some(2.5));
        assert_eq!(bool::from_word("yes"), None);
        assert_eq!(<Option<i32>>::from_word("4"), Some(Some(4)));
    }

    /// `cli::args().nth(at)` takes the `i64` the language counts in.
    #[test]
    fn a_position_is_an_i64() {
        let at: i64 = 0;
        assert!(super::args().nth(at).is_some());
        assert_eq!(super::args().nth(-1), None);
        assert_eq!(super::args().nth(i64::MAX), None);
    }
}
