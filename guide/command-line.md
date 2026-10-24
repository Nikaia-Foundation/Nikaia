# The command line: what `main` takes

This page shows how a Nikaia program gets the words it is started with. In short: you
write them as `main`'s parameters, and the program reads them, converts them to the
parameter types and checks them before the first line of `main` runs.

If you know Rust, it is close to what `clap` with `#[derive(Parser)]` gives you, without
the struct and the attributes. If you know Python, it is close to `typer`: the function's
parameters are the command line.

Each program below was run with today's compiler, and the output is pasted as it came. To
try one, save it as a file and run `nikaia run file.nika -- words…` (see [Passing the
words](#passing-the-words) below). The rules are in the specification, [Part I
5.5](../docs/specification/10-nikaia-light.md#55-mains-parameters-are-the-command-line),
and the reasons in [ADR-360](../docs/specification/adr/adr-360.md).

## A first example

```nika
/// Greets someone, a few times.
fn main(name: String; times: int = 1, loud: bool = false) {
    for _ in 0..<times {
        if loud {
            println(f"HELLO, {name}!")
        } else {
            println(f"Hello, {name}.")
        }
    }
}
```

The `;` splits the parameters in two, as it does for every function
([syntax tour](syntax.md#functions)). What stands before it, `name`, is a word the program
needs, given by position. What stands after it, `times` and `loud`, are options with a
default, given by name:

```text
$ nikaia run greet.nika -- Ada
Hello, Ada.
$ nikaia run greet.nika -- Ada --times 2
Hello, Ada.
Hello, Ada.
$ nikaia run greet.nika -- --times=3 Ada --loud
HELLO, Ada!
HELLO, Ada!
HELLO, Ada!
```

A word that does not fit ends the program before `main` runs. It prints one line that says
what is wrong, then the short usage, on standard error, and exits with status 2:

```text
$ nikaia run greet.nika --
error: `name` is missing: the program takes a `String` there.
usage: greet [--times <int>] [--loud] <name>
For more, try `greet --help`.
$ echo $?
2
```

## Reading the words by hand: `fn main()` and `cli::args()`

`main` does not have to take parameters. `cli::args()` returns every word the program was
started with, the program's own path first, and a program may read them itself:

```nika
use std::cli

fn main() {
    let count = cli::args().len()
    println(f"{count} word(s), the program's name first")
    for i in 0..<count {
        let word = cli::args().nth(i) ?? ""
        println(f"{i}: {word}")
    }
}
```

```text
$ nikaia run args.nika -- one "two words" --x
4 word(s), the program's name first
0: /root/.cache/nikaia/run/args-643f1d66b116db14/target/nikaia/bin/args
1: one
2: two words
3: --x
```

Nothing is converted or checked here: every word is a `String`, and `--x` is just a word.
`cli::args().nth(1) ?? "default"` is the short form for one optional word. `cli::args()`
also works in a `main` that has parameters, and returns the same words. Use it for what
the parameters cannot say (see [What is not supported yet](#what-is-not-supported-yet)).

## Positional words: the parameters before `;`

The parameters before the `;` take the words in order. Each is required, with two
exceptions.

**A `T?` parameter may be left out.** It is then `null`. It has to stand after every
required one:

```nika
fn main(greeting: String, name: String?) {
    let who = name ?? "world"
    println(f"{greeting}, {who}!")
}
```

```text
$ nikaia run hello.nika -- Hi
Hi, world!
$ nikaia run hello.nika -- Hi Ada
Hi, Ada!
```

**The last parameter may be a `Vec[T]`.** It takes every word that is left, also none:

```nika
/// Copies files into a directory (only prints what it would do).
fn main(target: String, files: Vec[String]; max_depth: int = 3, dry_run: bool = false) {
    for f in files {
        println(f"{f} -> {target} (depth {max_depth}, dry run {dry_run})")
    }
    println(f"{files.len()} file(s)")
}
```

```text
$ nikaia run copy.nika -- out a.txt b.txt --max-depth 5
a.txt -> out (depth 5, dry run false)
b.txt -> out (depth 5, dry run false)
2 file(s)
$ nikaia run copy.nika -- out --dry-run
0 file(s)
```

A word too many is an error:

```text
$ nikaia run greet.nika -- Ada Bob
error: `Bob` is one word too many.
usage: greet [--times <int>] [--loud] <name>
For more, try `greet --help`.
```

## Options: the parameters after `;`

Each parameter after the `;` is an option. It always has a default, which it takes when the
option is left out.

* `--name value` and `--name=value` both work.
* An `_` in the name is written `-`: `max_depth` is `--max-depth`. `--max_depth` is not
  accepted.
* Options may stand anywhere among the positional words: before, between or after them.
* A `bool` option is a bare flag: `--loud` sets it to `true`. `--loud=false` sets it to
  `false`. `--loud false` does not: there `false` is a positional word.
* `--` ends the options. Every word after it is positional, even one that starts with `--`.
* A word that is `-` followed by a digit is a value, never an option: `-3` is the number -3.
* An option of type `T?` may have the default `null`: `name: String? = null`.

```text
$ nikaia run copy.nika -- out a.txt --max_depth 2
error: `--max_depth` names no option.
usage: copy [--max-depth <int>] [--dry-run] <target> [<files>...]
For more, try `copy --help`.
$ nikaia run greet.nika -- Ada --times
error: `--times` needs a value: an `int`.
usage: greet [--times <int>] [--loud] <name>
For more, try `greet --help`.
$ nikaia run greet.nika -- -- --loud
Hello, --loud.
```

In the last line the first `--` belongs to `nikaia run` (see [Passing the
words](#passing-the-words)) and the second to the program, so `--loud` is taken as the
positional word `name`.

## Which types a parameter can have

A parameter of `main` is one of:

* `int`, a fixed-width integer (`i32`, `i64`, `u8`, `u32`, `u64`), `f64`, `bool`, `String`;
* one of these behind `?` (`String?`, `int?`, …);
* for the last positional parameter only: `Vec[T]` of one of these.

A number is read as `std` reads one from text: `-3` and `+5` are numbers, `0x10` and
`1_000` are not. It must fit its type. A `bool` is `true` or `false`.

```nika
fn main(x: int, scale: f64; offset: i32 = 0, level: u8 = 1) {
    println(f"x = {x}, scale = {scale}, offset = {offset}, level = {level}")
}
```

```text
$ nikaia run num.nika -- -3 2.5 --offset -7
x = -3, scale = 2.5, offset = -7, level = 1
$ nikaia run num.nika -- 3 1e3 --offset=-1 --level 9
x = 3, scale = 1000, offset = -1, level = 9
$ nikaia run num.nika -- 3 x
error: `scale` is a `f64`, and `x` is not one.
usage: num [--offset <i32>] [--level <u8>] <x> <scale>
For more, try `num --help`.
$ nikaia run num.nika -- 3 1 --level 300
error: `--level` is an `u8`, and `300` is not one.
usage: num [--offset <i32>] [--level <u8>] <x> <scale>
For more, try `num --help`.
```

Anything else does not build. A struct, a `ref` parameter, a `Vec` that is not the last
positional parameter, or a required positional parameter after a `T?` one is `NK1273`:

```text
$ nikaia check r2.nika
error[NK1273]: `count` of `main` is a required word after `name`, which may be left out.
  --> r2.nika:1:1
   |
 1 | fn main(name: String?, count: int) {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: A subject of type `T?` may be left out at the end; it stands after every required subject and before a last `Vec[T]` (Part I 5.5).
   = help: Move `count` before `name`, or make it a `T?` too.
1 type error
```

An option named `help` is `NK1274`, because `--help` is taken (see [`--help`](#--help-and--h)):

```text
$ nikaia check r3.nika
error[NK1274]: `main` has an option named `help`, and `--help` prints the program's help.
  --> r3.nika:1:1
   |
 1 | fn main(a: int; help: bool = false) {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `--help` and `-h` are the program's own: they print the `///` lines above `main` and its parameters (Part I 5.5).
   = help: Name the option differently.
1 type error
```

Where a word needs a shape these types do not have, take it as a `String` and convert it in
the body.

## Checking the words: `assert` on a parameter

In any Nikaia function, an `assert` about a parameter is a precondition: every caller has
to meet it ([Contracts](contracts.md)). `main`'s caller is the person at the terminal, so
the program checks `main`'s preconditions itself, right after reading the words and before
the body runs. A word that breaks one is a usage error like a word of the wrong type: one
line, the usage, exit status 2. It is not a panic (status 101), and the compiler does not
refuse it as an unchecked claim about outside data (`NK1202`).

The precondition does not have to be written in `main`. Here `share` asks for `people > 0`,
and `main` passes `people` on unchanged, so that claim becomes a precondition of `main` too:

```nika
fn share(total: int, people: int) -> int {
    assert(people > 0)
    return total / people
}

/// Splits a bill between people.
fn main(total: int, people: int; tip: int = 0) {
    assert(total >= 0)
    assert(tip <= 30)
    let each = share(total + total * tip / 100, people)
    println(f"each pays {each}")
}
```

```text
$ nikaia run split.nika -- 120 4 --tip 10
each pays 33
$ nikaia run split.nika -- -5 2
error: `total` is -5, which breaks `main`'s precondition `total >= 0`.
usage: split [--tip <int>] <total> <people>
For more, try `split --help`.
$ nikaia run split.nika -- 120 0
error: `people` is 0, which breaks `main`'s precondition `people > 0`, from `assert(people > 0)` in `share`.
usage: split [--tip <int>] <total> <people>
For more, try `split --help`.
$ echo $?
2
$ nikaia run split.nika -- 120 4 --tip 50
error: `tip` is 50, which breaks `main`'s precondition `tip <= 30`.
usage: split [--tip <int>] <total> <people>
For more, try `split --help`.
```

An option is a parameter like the others: an `assert` about `tip` is a precondition of
`main` and is checked with the words, as one about `total` is. The same holds in every
function: if `fn round(n: int; step: int = 1)` starts with `assert(step > 0)`, the call
`round(7; step: 0)` is refused while the program is built, and `round(7)` is proved with
the default.

The line names the parameter, the value given, the claim, and for a claim from another
function the `assert` it came from. If `main` had called `share(total, extra + 1)`, the
precondition would read `extra + 1 > 0`. Inside the body, every precondition is a fact the
compiler uses, as in any function. `nikaia build --asserts` lists them:

```text
$ nikaia build --asserts
asserts in …/src/main.nika: 4 - not proved 0 (refused 0, checked at run time 0, checked by a test 0); checked where the words arrive 3; proved 1 (proved 0, preconditions proved at every call 1)
  …/src/main.nika:8  assert(total >= 0)  precondition of `main`, checked where the words arrive
  …/src/main.nika:9  assert(tip <= 30)  precondition of `main`, checked where the words arrive
  …/src/main.nika  people > 0  precondition of `main` carried back from `assert(people > 0)`, checked where the words arrive
  …/src/main.nika:2  assert(people > 0)  precondition of `share`: proved at every call the compiler sees; any other reaches the entry that checks it
```

They are also written into `nikaia.contracts`, under `[fn."main"]`, so a change to what the
program accepts shows up there in review:

```toml
[fn."main"]
sync = "inferred"
locks = true
touches = ["stdout write"]
requires = ["total >= 0", "tip <= 30", "people > 0"]
from = ["assert(total >= 0)", "assert(tip <= 30)", "assert(people > 0)"]
signature = "(total: i64, people: i64; tip: i64 = 0)"
```

Words read from standard input or from a file are a different matter: they are not trusted,
and an `assert` about them is refused. That is in [Contracts](contracts.md#data-from-outside-a-guard-not-an-assert).

## `--help` and `-h`

Every program whose `main` takes parameters answers `--help` and `-h`. The help is built
from the `///` lines above `main`, the parameters with their types and defaults, and the
preconditions. It goes to standard output, and the program ends with status 0:

```text
$ nikaia run split.nika -- --help
split
Splits a bill between people.

usage: split [--tip <int>] <total> <people>

arguments:
  <total>              int; requires `total >= 0`
  <people>             int; requires `people > 0`, from `assert(people > 0)` in `share`

options:
  --tip <int>          int, default 0; requires `tip <= 30`
  -h, --help           prints this help
```

An optional word is shown in brackets, a `Vec[T]` with `...`:

```text
usage: hello <greeting> [<name>]
usage: copy [--max-depth <int>] [--dry-run] <target> [<files>...]
```

After `--`, `--help` and `-h` are ordinary positional words.

## Passing the words

**One file:** everything after `--` goes to the program.

```text
$ nikaia run greet.nika -- Ada --times 2
```

Without the `--`, `nikaia` reads the words as its own and refuses them
(`error: unexpected argument 'Ada' found`).

**A project** (a `nikaia.toml` and `src/main.nika`, see [Getting
started](getting-started.md)): the same, without the file name.

```text
$ nikaia run -- 90 3 --tip 10
each pays 33
```

**A built binary:** `nikaia build` writes it to `target/nikaia/bin/<name>`, and it takes the
words directly:

```text
$ nikaia build            # its progress lines left out here
$ ./target/nikaia/bin/split 90 3
each pays 30
$ ./target/nikaia/bin/split 90 0
error: `people` is 0, which breaks `main`'s precondition `people > 0`, from `assert(people > 0)` in `share`.
usage: split [--tip <int>] <total> <people>
For more, try `split --help`.
$ echo $?
2
```

## Claims the build does not prove: `unproved-claims` and `refuted-claims`

The compiler proves every `assert` ([Contracts](contracts.md)). It refuses a claim it does
not prove (`NK1300`), and it refuses a claim it shows false with values (`NK1207`). Two build
settings in `nikaia.toml` change that, and each has a flag that overrides it for one build:

```toml
[build]
unproved-claims = "warn"   # default "error"
refuted-claims = "warn"    # default "error"
```

```
$ nikaia build --unproved-claims warn
```

* **`unproved-claims`** ([ADR-380](../docs/specification/adr/adr-380.md) D2): at `warn` each
  `NK1300` is a warning, and the claim is checked where it is reached and stops the program
  when it is false. Nothing in the program has to change: the compiler writes the check.
* **`refuted-claims`** ([ADR-269](../docs/specification/adr/adr-269.md) D8): at `warn` a claim
  shown false, and an index or an arithmetic operation shown to fail for a value the program
  reaches, is a warning and a check where it is reached. It is the bypass for the day a better
  prover refutes what an older one built, for that emergency only: a claim newly refuted is
  fixed in the code.

`nikaia check` reports every claim it did not prove either way; the setting only decides
whether the build stops. Both are part of the build's cache key.

## What is not supported yet

These are not part of the language yet. A program that needs one reads `cli::args()` itself:

* short flags such as `-v` or `-n 3` (only `-h` exists, for the help);
* subcommands, as in `git commit`;
* an `enum` as a choice of words (`--mode fast`): take a `String` and match on it;
* environment variables as defaults;
* a `--version` flag.
