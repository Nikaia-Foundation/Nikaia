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

## Building a calculator, step by step

The second half of this chapter builds a complete parser: the four-function
calculator of [`examples/calc`](../examples/calc/src/main.nika). It reads
expressions such as `2 + 3 * (10 - 4) / 2`, with the usual precedence, brackets and
negative numbers, and computes their value while it parses. The blocks in this
section are pieces of that file, shown in the order they are best read; the file
itself is the complete program, and `nikaia test` in its directory runs the tests
at its end.

### The numbers

Start with the smallest piece. A number is a token, so it gets an uppercase rule,
and `dec[i64]` reads its digits:

```nika
rule NUMBER -> i64 =
    n:dec[i64](digit+) -> { n }
```

Because `NUMBER` is lexical, `1 2` is two numbers and not one. Because of
`dec[i64]`, a number too large for an `i64` is a parse error.

### Precedence is the order of the rules

`2 + 3 * 4` is 14, because `*` binds tighter than `+`. A grammar says that without a
precedence table: it gives each level its own rule, and the rule for the tighter
level sits below the looser one. An expression is a sum of *terms*, and a term is a
product of *factors*:

```nika
entry rule expr -> i64 =
    head:term tail:add_tail* -> {
        let mut value = head
        for t in tail { value = value + t }
        value
    }

rule add_tail -> i64 =
      "+" t:term -> { t }
    | "-" t:term -> { -t }
```

`expr` reads one term and then any number of `+ term` or `- term`. Each tail hands
back its term, negated after a `-`, and the action adds them up from left to right.
Folding from the left is what makes `10 - 4 - 3` mean `(10 - 4) - 3`, which is 3.

The tails are rules of their own because of how labels work: a label inside
`("+" t:term)*` would not reach the action of `expr`, while `tail:add_tail*` binds a
list of what each `add_tail` built.

A term works the same way one level down. Division cannot be turned into an
addition the way subtraction can, so each operand travels with its operator, as a
pair of an `Op` and the value:

```nika
enum Op {
    Times,
    Divide,
}

rule term -> i64 =
    head:factor tail:mul_tail* -> {
        let mut value = head
        for step in tail {
            match step {
                (Op::Times, n) => { value = value * n }
                (Op::Divide, n) => { value = value / n }
            }
        }
        value
    }

rule mul_tail -> (Op, i64) =
    op:mul_op f:factor -> { (op, f) }

rule mul_op -> Op =
      "*" -> { Op::Times }
    | "/" -> { Op::Divide }
```

`Op` has two variants, so the `match` in the action needs exactly two arms, and the
compiler checks that it has them.

### Recursion and brackets

A factor is a number, a negated factor, or a whole expression in brackets. The
brackets are where the grammar calls `expr` again, so `(10 - 4)` is a factor and
multiplies like one:

```nika
rule factor -> i64 # "expression" =
      "(" e:expr ")" -> { e }
    | "-" f:factor -> { -f }
    | n:NUMBER -> { n }
```

The `# "expression"` after the result type names the rule for error messages. Where
a factor is missing, the message says `expected expression` instead of listing the
three ways a factor can begin. The name is used only where the rule failed at its
own start; a factor that got further reports what it was in the middle of, such as
the missing `)` in `(1`.

### Calling it

`main` takes the expression from the command line and handles a parse that fails
with `catch`:

```nika
fn main() throws {
    let source = cli::args().nth(1) ?? "2 + 3 * (10 - 4) / 2"

    let value = Calc::expr(source) catch {
        eprintln(f"{source}: {error}")
        return
    }

    println(f"{source} = {value}")
}
```

Run in `examples/calc`:

```text
$ nikaia run -- "2 + 3 * (10 - 4) / 2"
2 + 3 * (10 - 4) / 2 = 11
$ nikaia run -- "10 - 4 - 3"
10 - 4 - 3 = 3
$ nikaia run -- "2 +"
2 +: unexpected end of input, expected expression at line 1, column 4
   1 | 2 +
          ^
note: also possible here: a digit
in factor
in term
in add_tail
in item 1
in expr
$ nikaia run -- "(1"
(1: unexpected end of input, expected `)` at line 1, column 3
   1 | (1
         ^
note: also possible here: `*`, `+`, `-`, `/`, a digit
in factor
in term
in expr
```

The grammar's promises are written as tests at the end of the file, so `nikaia
test` checks that `2 + 3 * 4` is 14 and that `2 +` is refused.

One case is still open in this calculator: `1 / 0` divides by zero inside an
action, and that stops the program, as a division by zero does anywhere else. The
next section shows how an action reports such a case as a parse error instead.

### Left recursion

Many grammar books write the sum as `expr = expr "+" term | term`, a rule that
begins with itself. The specification does not describe what such a rule means
yet, so this chapter, and the calculator, write a head and a list of tails
instead. That form is specified, and its action states the order of evaluation
in plain code. Left recursion through a rule that takes values is listed as not
decided in [ADR-grammar-1](../docs/specification/adr/adr-grammar-1.md).

**Specification:** [Part II, §10.6 Advanced parser features](../docs/specification/20-nikaia-advance.md#106-advanced-parser-features),
[§10.8](../docs/specification/20-nikaia-advance.md#108-the-grammars-vocabulary)

## Commit points

When an alternative fails, the parser tries the next one. That is what a choice
needs, but it also hides the real mistake: if `(3 4)` fails inside the first
alternative and the parser moves on, the message is about the alternatives it
tried last, not about the missing comma. A *commit point*, written `=>`, says that
once the part before it matched, this alternative is the only one, and a failure
after it is the error:

```nika
grammar Point {
    entry rule point -> (i64, i64) =
        "(" => x:NUM "," y:NUM ")" -> { (x, y) }
      | "origin" -> { (0, 0) }
    rule NUM -> i64 # "number" = n:dec[i64]("-"? digit+) -> { n }
}

fn show(input: String) {
    let p = Point::point(input) catch {
        println(f"{input}: {error}")
        return
    }
    let (x, y) = p
    println(f"{input}: x {x}, y {y}")
}

fn main() {
    show("(3, -4)")
    show("origin")
    show("(3 4)")
    show("(3,")
    show("north")
}
```

```text
(3, -4): x 3, y -4
origin: x 0, y 0
(3 4): expected `,`; found unexpected token `4` at line 1, column 4
   1 | (3 4)
          ^
in point
(3,: unexpected end of input, expected number at line 1, column 4
   1 | (3,
          ^
note: also possible here: `-`
in NUM
in point
north: expected one of: `(`, `origin`; found unexpected token `north` at line 1, column 1
   1 | north
       ^^^^^
in point
```

Once `(` matched, the comma is what the message asks for. Where nothing of an
alternative matched, as with `north`, the message lists what each alternative could
have begun with.

**Specification:** [Part II, §10.1](../docs/specification/20-nikaia-advance.md#101-parsing-with-grammar-scannerless),
[§10.8](../docs/specification/20-nikaia-advance.md#108-the-grammars-vocabulary)

## Actions that refuse a value

Some checks need code: a colour component must fit in a byte, a date must exist.
An action may call a function that throws, and the failure becomes a failure of the
parse, reported at the place of the value with the function's message:

```nika
struct TooBig {
    n: i64,
}

impl Error for TooBig {
    fn message(ref self) -> String {
        return f"{self.n} does not fit in a byte"
    }
}

fn byte(n: i64) -> i64 throws {
    if n > 255 {
        throw TooBig { n: n }
    }
    return n
}

grammar Rgb {
    entry rule color -> (i64, i64, i64) =
        "rgb" => "(" r:BYTE "," g:BYTE "," b:BYTE ")" -> { (r, g, b) }
    rule BYTE -> i64 = n:dec[i64](digit+) -> { byte(n) }
}

fn main() {
    for input in ["rgb(255, 128, 0)", "rgb(255, 300, 0)"] {
        let (r, g, b) = Rgb::color(input) catch {
            println(f"{input}: {error}")
            continue
        }
        println(f"{input}: {r} {g} {b}")
    }
}
```

```text
rgb(255, 128, 0): 255 128 0
rgb(255, 300, 0): 300 does not fit in a byte at line 1, column 13
   1 | rgb(255, 300, 0)
                   ^
note: also possible here: a digit
in item 4
in BYTE
in color
```

The commit point decides what such a failure means. Before a `=>`, a failing action
fails its alternative, and the parser tries the next one; after a `=>`, as here, it
is the error of the whole parse. An action may not pause, for example to wait on the
network: a parse runs from start to end without waiting, and the compiler refuses an
action whose call can pause.

**Specification:** [Part II, §10.8, *An action may not pause*](../docs/specification/20-nikaia-advance.md#108-the-grammars-vocabulary)

## Your own whitespace, with comments

A lowercase rule skips whatever the rule `WS` matches, and by default that is any
run of whitespace. A grammar that declares its own `WS` decides what is skipped.
Here it is whitespace or a `#` comment up to the end of the line, so a comment is
allowed wherever a blank is, and no other rule mentions comments:

```nika
grammar Kv {
    rule BLANK = multispace1 -> { }
    rule COMMENT = "#" until(line_ending) -> { }
    rule WS = (BLANK | COMMENT)* -> { }

    entry rule file -> Vec[(String, i64)] = xs:pair* eof -> { xs }
    rule pair -> (String, i64) = k:NAME "=" v:NUM -> { (k, v) }
    rule NAME -> String = s:raw_ident -> { s }
    rule NUM -> i64 = n:dec[i64](digit+) -> { n }
}

fn main() throws {
    let xs = Kv::file("# screen size\nwidth = 80   # columns\nheight = 24\n")
    for (k, v) in xs {
        println(f"{k} -> {v}")
    }
}
```

```text
width -> 80
height -> 24
```

A rule written without a result type, such as `BLANK`, builds nothing. `BLANK`
exists because `multispace1` on its own yields text and `COMMENT` yields nothing,
and a group whose alternatives yield different things does not compile yet
([#1001](https://github.com/Nikaia-Foundation/Nikaia/issues/1001)). `rule WS = "" -> { }`
skips nothing at all, for formats where blanks matter.
[`examples/config`](../examples/config/src/main.nika) uses this pattern for an
INI-style file.

**Specification:** [Part II, §10.8, *Lexical and syntactic*](../docs/specification/20-nikaia-advance.md#108-the-grammars-vocabulary)

## Rules that take values, and guards

A rule may take values, declared like a function's parameters, and a pattern may
use them. A guard, `if(cond)`, lets an alternative go on only where a condition
holds; its second argument names what was expected, for the message. This format
starts with a field width and then holds two fields, the first of that width:

```nika
grammar Record {
    entry rule record -> (String, String) =
        w:dec[i64](digit) if(w > 0, "a width above 0") ":" a:Field(w) b:Field(2) eof -> { (a, b) }
    rule Field(width: i64) -> String = s:text(any{width}) -> { s }
}

fn main() {
    for input in ["4:AbcdEf", "2:AbCd", "0:Ab"] {
        let (a, b) = Record::record(input) catch {
            println(f"{input}: {error}")
            continue
        }
        println(f"{input}: {a} | {b}")
    }
}
```

```text
4:AbcdEf: Abcd | Ef
2:AbCd: Ab | Cd
0:Ab: expected a width above 0; found unexpected token `:` at line 1, column 2
   1 | 0:Ab
        ^
in record
```

`Field(w)` hands the value bound before it to the rule, and `any{width}` repeats
`any` that many times. `count(p)` is the companion for formats that depend on
indentation: it matches `p` as often as it can and binds how often, as an `i64`, so
`m:count(" ")` measures the indentation of a line and a guard compares it. A rule's
values may also have defaults and be passed by name after a `;`, as a function's
can; §10.8 has the details and an indentation-based example.

**Specification:** [Part II, §10.8, *A rule may take values*](../docs/specification/20-nikaia-advance.md#108-the-grammars-vocabulary)

## Error recovery

A parser for a programmer's tool should not stop at the first mistake: the errors
after it are worth reporting too. `recover(p, sync)` parses `p`, and where `p`
fails it skips ahead to `sync` and carries on. A repeated `recover` gives a list
with one entry per item, `null` for each item that failed:

```nika
grammar Stmts {
    entry rule block -> Vec[i64?] =
        "{" => xs:recover(stmt, ";")* "}" -> { xs }
    rule stmt -> i64 = "let" n:NUM ";" -> { n }
    rule NUM -> i64 = n:dec[i64](digit+) -> { n }
}

fn main() throws {
    let xs = Stmts::block("{ let 1; let x; let 3; }") catch {
        println(f"{error}")
        return
    }
    for x in xs {
        println(f"{x ?? -1}")
    }
}
```

```text
1
-1
3
```

The commit point after `{` and the recovery work together: the commit point decides
that a failure inside the block is reported rather than a reason to try another
alternative, and the recovery decides where parsing resumes. Two parts of the
specified behaviour are not available yet. The program receives `null` for a failed
item, but not the message the failure would have produced. And a commit point
inside the recovered rule itself, such as `"let" => n:NUM ";"`, makes its failure
end the whole parse instead of being recovered.

**Specification:** [Part II, §10.6, *Fault tolerance*](../docs/specification/20-nikaia-advance.md#106-advanced-parser-features)

## Positions for messages

A tool that reports problems needs to say where they are. `span(p)` hands back the
range of the input that `p` covered, and `here` the current position, consuming
nothing. Today both work over a `Bytes` input, where a span is a `text::Span` of two
byte offsets:

```nika
use std::text

grammar Words {
    entry rule words -> Vec[(String, text::Span)] = xs:word* eof -> { xs }
    rule word -> (String, text::Span) = s:span(w:WORD) -> { (w, s) }
    rule WORD -> String = w:alpha1 -> { w }
}

fn main() throws {
    let source = "one  two three"
    let input = Bytes(source.bytes().collect())
    for (w, s) in Words::words(input) {
        println(f"{w} at {s.start}..{s.end}")
    }
}
```

```text
one at 0..3
two at 5..8
three at 9..14
```

A span ends where its pattern ends, not after the whitespace skipped behind it.
Over a `Bytes`, an element that yields text checks that the bytes it covered are
UTF-8, and a byte that is not ends the parse with an error that names its offset.
Over a `String`, `span` and `here` are specified to give a `text::Match` and a
`text::Pos`, which are not built yet
([#743](https://github.com/Nikaia-Foundation/Nikaia/issues/743)); the compiler refuses
such a call and suggests passing a `Bytes`.

**Specification:** [Part II, §10.8, *Text or bytes*](../docs/specification/20-nikaia-advance.md#108-the-grammars-vocabulary)

## Parsing while the program is built

The same grammar runs while the program is built. In a `comptime` binding the call
happens during the build, the result is part of the program, and input the grammar
refuses stops the build with the parser's message:

```nika
grammar Size {
    entry rule size -> i64 = n:NUM u:unit -> { n * u }
    rule unit -> i64 =
          "KiB" -> { 1024 }
        | "MiB" -> { 1024 * 1024 }
        | "" -> { 1 }
    rule NUM -> i64 = n:dec[i64](digit+) -> { n }
}

comptime LIMIT: i64 = Size::size("4 MiB")

fn main() {
    println(f"{LIMIT}")
}
```

```text
4194304
```

A build reads a file only where the program names it with `asset("…")` and the
build is given an allowlist that names it as well, so parsing a file at build time
does not let the build read anything else.

**Specification:** [Part II, §10.2 Dual-mode parsing](../docs/specification/20-nikaia-advance.md#102-dual-mode-parsing-static-vs-dynamic)

## Further features

A few features go beyond a first parser, and each has a complete program that shows
it working.

A rule may take types as well as values, and `members[S]` reads the members of an
object in any order into the fields of a struct `S`, with one mapping rule for
missing, repeated and unknown members. A JSON reader written that way builds any
struct the program declares;
[`tests/compiler/tests/grammar_members_builds_a_struct.nika`](../tests/compiler/tests/grammar_members_builds_a_struct.nika)
is a short one, and [`examples/json`](../examples/json/src/main.nika) a complete
JSON parser.

For large inputs, a rule marked `@frame` says where the input may be cut, and
`par_fold` parses the pieces in parallel and merges the results;
[`examples/1brc.nika`](../examples/1brc.nika) reads a file of measurements that way.
A `dsl` block hands text written inside a Nikaia file to a grammar while the
program is built; [`examples/report`](../examples/report/src/main.nika) uses one.
Identifiers can be interned into a table the caller owns, so comparing two names
compares two integers (§10.6).

**Specification:** [Part II, §10.5 Using DSLs](../docs/specification/20-nikaia-advance.md#105-using-dsls-the-dsl-keyword),
[§10.6](../docs/specification/20-nikaia-advance.md#106-advanced-parser-features),
[§10.7 Parallel parsing](../docs/specification/20-nikaia-advance.md#107-parallel-parsing),
[§10.8](../docs/specification/20-nikaia-advance.md#108-the-grammars-vocabulary)

---

## Where next

* [A tour of the syntax](syntax.md): the rest of the language on one page.
* [The prover](prover.md): what the compiler proves today, where it stops, and what is planned.
* [`examples/calc`](../examples/calc/src/main.nika): a calculator in about 100 lines.
* [`examples/config`](../examples/config/src/main.nika): an INI-style configuration file.
* [`examples/json`](../examples/json/src/main.nika): a complete JSON parser.
* [Specification, Part II, Chapter 10](../docs/specification/20-nikaia-advance.md#chapter-10-metaprogramming-code-that-writes-code):
  the full rules for grammars.
