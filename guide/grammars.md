# Grammars and Parsers

Sooner or later most programs read text that someone else wrote: a configuration
file, a log line, a command typed by a user, a message from the network. That
text is input from outside, and a program that reads it by hand, with indexes and
`split`, tends to accept things it should refuse and to stop on things it should
report. Nikaia lets you describe the format instead, as a `grammar`, and the
compiler builds the parser from that description. The parser hands back values of
your own types, and input it cannot read is an ordinary error that the compiler
asks you to handle, like any other call that can fail.

This chapter teaches grammars from a three-line example up to a complete
calculator. Every program in a `nika` block is complete and was run with the
current compiler; the output under it is what that run printed. To try one, save it
as a file and run `nikaia run file.nika` (see [Getting started](getting-started.md)).
A few blocks are pieces of a larger program, and the text says so where they appear.

The full rules are in the specification,
[Part II, Chapter 10](../docs/specification/20-nikaia-advance.md#chapter-10-metaprogramming-code-that-writes-code).
Each section ends with a **Specification** link to the part it comes from.

---

## What a grammar is

A grammar is a set of named rules. Each rule says what text it matches and which
value it builds from that text. One rule, or several, is marked `entry`, and the
program calls it like a function.

Many parser tools split the work in two: a tokenizer first cuts the text into
words, and a parser then arranges the words. A Nikaia grammar is *scannerless*: it
reads the characters directly, and the same rules describe both the small pieces
(a number, a name) and the structure around them (a list of settings, an
expression in brackets). There is one description to read and one to keep up to
date.

**Specification:** [Part II, §10.1 Parsing with `grammar`](../docs/specification/20-nikaia-advance.md#101-parsing-with-grammar-scannerless)

## A first grammar

The format here is a single setting, a name, an equals sign and a number, such as
`width = 80`. The program parses it into a `Pair`:

```nika
grammar Pairs {
    entry rule pair -> Pair =
        key:NAME "=" value:NUMBER -> { Pair { key, value } }

    rule NAME -> String = s:alpha1 -> { s }
    rule NUMBER -> i64 = n:dec[i64](digit+) -> { n }
}

struct Pair {
    key: String,
    value: i64,
}

fn main() throws {
    let p = Pairs::pair("width = 80")
    println(f"{p.key} is {p.value}")
    let bad = Pairs::pair("width = eighty") catch {
        println(f"refused: {error}")
        return
    }
    println(f"{bad.key}")
}
```

```text
width is 80
refused: expected a digit; found unexpected token `eighty` at line 1, column 9
   1 | width = eighty
               ^^^^^^
in item 1
in NUMBER
in pair
```

Read a rule from left to right. `rule NUMBER -> i64` names the rule and the type it
builds. After the `=` comes the pattern: `digit+` is one or more decimal digits, and
`dec[i64](…)` reads the text they cover as an `i64`. The label `n:` binds that
value, and the block after the second arrow, `-> { n }`, is the *action*: ordinary
Nikaia code that builds the rule's result from its labels. The rule `pair` uses the
two smaller rules and the literal `"="`, and its action builds a `Pair`.

`Pairs::pair(…)` is the call. It is written with `::`, as every qualified name is,
and it runs the entry rule `pair` over the whole string. A rule without `entry`
belongs to the grammar alone, and a call to it from outside is refused.

**Specification:** [Part II, §10.2 Dual-mode parsing](../docs/specification/20-nikaia-advance.md#102-dual-mode-parsing-static-vs-dynamic)

## When the input does not fit

The second call above hands the grammar `width = eighty`. The parse fails, and the
failure is an error of the call, which `catch` handles as it handles a file that
cannot be opened. The message says what the parser expected, what it found, and
where, and it shows the line with the place marked. The `in …` lines below it are
the rules the parser was inside, innermost first.

An entry rule *throws*: the compiler knows that the call can fail, so a function
that calls it either handles the error with `catch` or is marked `throws` itself
and passes the error on. A program cannot use a parse result without deciding what
happens when there is none. A `main` marked `throws` that lets the error through
stops with the same message:

```nika
grammar Point {
    entry rule point -> i64 = "(" x:NUM ")" -> { x }
    rule NUM -> i64 = n:dec[i64](digit+) -> { n }
}

fn main() throws {
    let x = Point::point("(12")
    println(f"{x}")
}
```

```text
Error: unexpected end of input, expected `)` at line 1, column 4
   1 | (12
          ^
note: also possible here: a digit
in item 3
in NUM
in point
  (raised below Nikaia; no site recorded)
  (no trace; set NIKAIA_TRACE=1 to capture one)
```

The `note` lists what else could have continued the input at that place: here
another digit of the number. The call also fails where text is left over after the
entry rule is done, so `12x` is refused rather than read as `12`.

Numbers get the same care. `dec[i64](…)` refuses digits whose value does not fit
in an `i64`, so a number too large is a parse error and not a value that quietly
wrapped around.

**Specification:** [Part II, §10.8 The grammar's vocabulary](../docs/specification/20-nikaia-advance.md#108-the-grammars-vocabulary)

## Uppercase rules and lowercase rules

The case of a rule's first letter decides how it treats whitespace. A lowercase
rule is *syntactic*: whitespace between its parts is skipped, so `width = 80`,
`width=80` and `width   =   80` all match `pair` above, and no rule has to mention
blanks. An uppercase rule is *lexical*: nothing is skipped inside it, so it is
where tokens are described, the words and numbers of the format.

The difference shows when the same pattern stands in both kinds of rule:

```nika
grammar Words {
    entry rule loose -> String = s:text(digit+) -> { s }
    entry rule tight -> String = s:DIGITS -> { s }
    rule DIGITS -> String = s:text(digit+) -> { s }
}

fn main() {
    println(Words::loose("1 2") catch { "refused" })
    println(Words::tight("1 2") catch { "refused" })
}
```

```text
1 2
refused
```

In the lowercase rule the blank between the digits was skipped, so `1 2` came out
as one run of digits. The uppercase `DIGITS` stops at the blank, and the call then
refuses the ` 2` that is left over. A good habit follows: describe each token in an
uppercase rule of its own, and build the structure from lowercase rules that use
them. All the examples in this chapter are written that way.

**Specification:** [Part II, §10.8, *Lexical and syntactic*](../docs/specification/20-nikaia-advance.md#108-the-grammars-vocabulary)

## Alternatives, repetition and optional parts

A larger format needs choices and repeats. This one is a list of settings, each
ending in `;`, whose values are numbers, `true` or `false`, or quoted text, and
which may carry a comment in brackets:

```nika
enum Value {
    Number(i64),
    Flag(bool),
    Text(String),
}

struct Setting {
    key: String,
    value: Value,
    comment: String?,
}

grammar Settings {
    entry rule file -> Vec[Setting] = s:setting* eof -> { s }

    rule setting -> Setting =
        key:NAME "=" value:value c:comment? ";" -> { Setting { key, value, comment: c } }

    rule value -> Value =
          "true" -> { Value::Flag(true) }
        | "false" -> { Value::Flag(false) }
        | n:dec[i64](digit+) -> { Value::Number(n) }
        | s:string -> { Value::Text(s) }

    rule comment -> String = "(" t:until(")") ")" -> { t }

    rule NAME -> String = s:text(("_" | alpha1)+) -> { s }
}

fn main() throws {
    let input = "width = 80; title = \"Report\" (shown on top); draft_mode = true;"
    let settings = Settings::file(input)
    for s in settings {
        let shown = match s.value {
            Value::Number(n) => f"number {n}"
            Value::Flag(b) => f"flag {b}"
            Value::Text(t) => f"text {t}"
        }
        println(f"{s.key}: {shown}, comment {s.comment ?? "-"}")
    }
}
```

```text
width: number 80, comment -
title: text Report, comment shown on top
draft_mode: flag true, comment -
```

`|` separates alternatives. The parser tries them in the order written and takes
the first that matches, and each alternative has its own action. `value` therefore
reads `true` before it tries a number.

`*` repeats a pattern zero or more times and `+` one or more times. A labelled
repetition, `s:setting*`, binds a list of what each repeat built, here a
`Vec[Setting]`. The repetition stops by itself where the next setting cannot begin.
`eof` matches the end of the input and makes the end part of the description.

`?` makes a part optional. `c:comment?` binds a `String?`: the comment's text where
there is one, and `null` where there is none, which is why the field is declared
`String?` and the program reads it with `?? "-"`.

`text(…)` hands back the text a pattern covered instead of its value. `NAME` uses
it to turn a run of letters and underscores into one `String`. Put the group in
front, as `NAME` does: a rule name written directly before `(` is read as a call of
that rule with values (see *Rules that take values* below), so `alpha1 ("_" | …)`
is not what it looks like.

**Specification:** [Part II, §10.8 The grammar's vocabulary](../docs/specification/20-nikaia-advance.md#108-the-grammars-vocabulary)

## Literals and character classes

The small pieces a pattern is made of are literals and built-in classes. A literal
in quotes, `"="`, matches exactly that text. The classes used most often:

| element | matches | a label binds |
| :--- | :--- | :--- |
| `"text"` | that text, exactly | the text matched |
| `digit` | one decimal digit; `digit+` is a run of them | a `scalar`; text for the run |
| `alpha1` | a run of letters | text |
| `hex_digit` | one hexadecimal digit | a `scalar` |
| `raw_ident` | an identifier | text |
| `string` | a quoted string, quotes excluded | text |
| `any` | one character | a `scalar` |
| `multispace0`, `multispace1` | zero or more, one or more whitespace characters | text |
| `line_ending` | `\n` or `\r\n` | nothing |
| `until(p)` | everything up to `p`, leaving `p` in place | text |
| `not(p)` | nothing; succeeds where `p` does not match | nothing |
| `peek(p)` | `p`, without consuming it | as `p` |
| `p{n}`, `p{n,m}` | `p` exactly `n` times, or between `n` and `m` times | a list, or the text for a class |
| `list(item, sep)` | `item`s separated by `sep`, zero or more | a list |
| `fold(p, init, step)` | `p` repeated, each value folded into an accumulator | the accumulator |

A few of them at work, each as an entry of its own:

```nika
grammar Lists {
    entry rule numbers -> Vec[i64] = "[" xs:list(NUM, ",") "]" eof -> { xs }
    entry rule sum -> i64 = s:fold(NUM, 0, fn(acc, n) { acc + n }) eof -> { s }
    entry rule color -> (String, String, String) = "#" r:BYTE g:BYTE b:BYTE eof -> { (r, g, b) }
    entry rule word -> String = not("let") w:raw_ident -> { w }
    rule BYTE -> String = h:text(hex_digit{2}) -> { h }
    rule NUM -> i64 = n:dec[i64](digit+) -> { n }
}

fn main() throws {
    let xs = Lists::numbers("[1, 2, 3]")
    println(xs.iter().map(fn(n) { f"{n}" }).join(", "))
    println(f"{Lists::numbers("[]").len()}")
    println(f"{Lists::sum("1 2 3 4")}")
    let (r, g, b) = Lists::color("#ff8000")
    println(f"{r} {g} {b}")
    println(Lists::word("total"))
    let _w = Lists::word("let") catch { println(f"word: {error}"); "" }
}
```

```text
1, 2, 3
0
10
ff 80 00
total
word: unexpected token `let` at line 1, column 1
   1 | let
       ^^^
in word
```

`list` reads the separators and keeps only the items. `fold` adds the numbers up
as it reads them and collects nothing. `not("let")` refuses input that begins with
`let`; it compares text, so it refuses `letter` as well, and a keyword check that
must not do that writes what may follow the word.

The full list, with every element the language has, is the table in §10.8. A few
spellings from other parser libraries are refused with a message that names the
one to write: `tag("x")` is `"x"`, and `digit1` is `digit+`.

**Specification:** [Part II, §10.8 The grammar's vocabulary](../docs/specification/20-nikaia-advance.md#108-the-grammars-vocabulary)

## How results become Nikaia types

Each rule declares the type it builds, and the compiler checks every action against
it. What a label binds follows from the pattern it stands on:

| pattern | a label on it binds |
| :--- | :--- |
| a rule `r -> T` | a `T` |
| `p?` | a `T?`, `null` where `p` is absent |
| `p*`, `p+`, `p{n}` | a `Vec[T]`, or the text matched where `p` is a character class |
| `text(p)`, `until(p)`, a literal | text |
| `dec[T](p)` | a `T` |

A rule may build any type the program declares: a number, a `String`, a tuple, a
`struct` or an `enum`, as the examples above do. The text a rule hands back is a
view of the input, so a parser copies none of it. A result that holds such text may
not outlive the input it came from; the compiler works that out from the types,
and the program declares nothing for it.

Labels belong to the alternative they are written in. A label inside a repetition
or a choice is not visible to the action outside it: in `xs:(d:NUM ",")*` the
action sees `xs`, not `d`. A rule that needs both writes a smaller rule for the
repeated part, as the calculator below does with `add_tail` and `mul_tail`.

**Specification:** [Part II, §10.8](../docs/specification/20-nikaia-advance.md#108-the-grammars-vocabulary)

---

## Where next

* [A tour of the syntax](syntax.md): the rest of the language on one page.
* [`examples/calc`](../examples/calc/src/main.nika): a calculator in about 100 lines.
* [`examples/config`](../examples/config/src/main.nika): an INI-style configuration file.
* [Specification, Part II, Chapter 10](../docs/specification/20-nikaia-advance.md#chapter-10-metaprogramming-code-that-writes-code):
  the full rules for grammars.
