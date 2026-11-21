# ADR-cli-v1: `main`'s Parameters Are the Command Line; a `Command[f, g]` Parameter Picks a Function by the First Word, and a Name `main` Declares Is `main`'s

**Status:** Accepted
**Date:** November 21, 2026 (D1-D7 from October 17, 2026)
**Answers:** whether `main` may take parameters, how the words a program is
started with reach them, and what an `assert` about them is (issue
[#635](https://github.com/Nikaia-Foundation/Nikaia/issues/635)); how a program
with several commands (`drive run`, `drive focus-next FILE`) declares them,
how options every command takes are shared, and what a name declared twice
means (issue [#449](https://github.com/Nikaia-Foundation/Nikaia/issues/449))
**History:** the record this one replaces is archived in [`docs/history/adr`](../../history/adr/README.md); read it only for how the decision was reached
**Supersedes / Related:** consolidates the archived record
(D1-D7 kept with their numbers, D2 and D5 extended; D1, D2 *changed by
[ADR-399](adr-399.md) D1, D2: `main`'s parameters are untrusted, their
preconditions still checked where the words arrive*);
Part I 5.1 (subject and options; an option's default);
[ADR-call-spread-v1](adr-call-spread-v1.md) (a call spreads a tuple and a
struct); [ADR-269](adr-269.md) D5, D6, D14-D21 (a claim about parameters is
the caller's; data from outside is checked where it arrives; a claim is
carried back to the entry); [ADR-318](adr-318.md) (a default is a build-time
value); [ADR-010](adr-010.md) (`cli::args`; *changed by
[ADR-399](adr-399.md): untrusted*); [ADR-285](adr-285.md) D36 (`int`);
Part III 14.2, 17.1 (`std::cli`, the shape of a number in text), A.2 (how the
program ends)

## 1. Question

`fn main(a: int) { assert(a > 0) … }` reads as *the program takes a number,
and it must be above 0*. For any other function that is what it means: the
`assert` is a precondition, written in the ledger, proved by every caller and a
fact in the body after it. `main`'s caller is the person at the command line,
who is not in the program (D1-D7).

A tool such as `scripts/drive.nika` has several commands: `drive run`,
`drive focus-next areas.md parser`. Each takes its own words, and some options
(`--verbose`) every command takes. Today a program reads `cli::args()`,
compares the first word by hand and writes its help by hand. The question is
how the signature says *which commands there are*, how each command's
function receives the shared options, and what happens when `main` and a
command both declare `--verbose` (D8-D12).

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| Rust with clap (derive) | a `struct` with attributes, `Args::parse()`; `#[command(subcommand)]` an `enum`, `#[command(flatten)]` a shared struct; a word declared twice panics when the parser is built | the program matches on the enum and passes the shared struct by hand; the duplicate is found when the program first runs | clap docs, *Derive tutorial*, `flatten`, `Subcommand` |
| Swift ArgumentParser | a `struct` with `@Argument`, `@Option`, `@Flag`; `subcommands:` a list of types; `@OptionGroup` a shared struct | each subcommand is its own type with `run()`; status 64 for a wrong word | ArgumentParser docs, *Commands and Subcommands* |
| Python typer | the function's parameters: positional without a default, `--name` with one; `@app.command()` per function, `@app.callback()` for the shared options | shared options reach a command through a context object, not its parameters | typer docs, *Commands*, *Callback* |
| Symfony Console | a class per command; global options on the application | a global option is read from the input object, not a parameter | Symfony docs, *Console* |
| Go `flag` | variables registered, `flag.Parse()` | subcommands by hand with `FlagSet` | Go docs, `flag` |
| C; Nikaia before D1 | a list of strings | written by hand | - |

None of them makes the check a fact for a prover, and none lets the command
be an ordinary function a test calls with the shared options as a parameter.

## 3. Options, weighed

D1-D7 were weighed in the archived record's §3 (parameters
refused, `Vec[String]`, a panic on a breach, only an `assert` at the top, a
required `T?`, status 64, short flags and enums from the start); this record
keeps them. The options here are for several commands and shared options.

### Option A — `Command[f, g]` as `main`'s subject, shared options a struct of `main`; a name `main` declares is `main`'s (chosen)

```nika
/// Options every command takes.
struct Global {
	/// Prints each step.
	verbose: bool = false,
}

/// Builds and pushes the lane's issues.
fn main(command: Command[run, focus_next]; global: Global = Global {}) throws {
	command.run(..command.args; ..command.options, global: global)
}

/// Builds the queue's next issue.
fn run(; global: Global = Global {}) throws { … }

/// Picks the next focus area from FILE, skipping the areas named.
fn focus_next(file: String, skipped: Vec[String]; global: Global = Global {}) throws { … }
```

`drive focus-next areas.md parser --verbose` calls `focus_next("areas.md",
["parser"]; global: Global { verbose: true })`. A test calls exactly that.

### Option B — the same, but a name declared by `main` and by a command is refused

`focus_next(...; global: Global = Global {})` beside `main(...; global: Global)`
would be `NK1357`; each command reads the shared options some other way.

### Option C — an `enum` of commands and a `match` (clap's form)

```nika
enum Cmd { Run, FocusNext(String, Vec[String]) }
fn main(cmd: Cmd; verbose: bool = false) throws {
	match cmd { Cmd::Run => run(verbose: verbose), Cmd::FocusNext(f, s) => focus_next(f, s; verbose: verbose) }
}
```

### Option D — the command implied, `@command` on each function

```nika
@command
fn focus_next(file: String, skipped: Vec[String]; verbose: bool = false) throws { … }
```

| | A | B | C | D |
| :--- | :--- | :--- | :--- | :--- |
| D1 one meaning per form (ADR-258) | `Command[...]` is one type; `..` is the general spread | as A | an enum that only parses words | an attribute that changes what `main` is |
| D2 a reader sees what runs | `main`'s signature lists the commands; the call is written | as A | the `match` repeats every parameter | `main` does not show it |
| D3 checked while built | the spread is checked as the written-out call; a name shared without being passed is refused (D11) | as A, but the shared struct cannot be a parameter of the command | the enum and the functions can drift | as A |
| D4 smallest | one type, the general spread | as A plus a rule that forbids the natural form | a type per program and a `match` | an attribute and an implied `main` |
| If it is wrong | a program writes the `match` itself; `Command` is refused again | widening to A later is free | - | - |

Also weighed and dropped: `Global` as the first subject of every command (a
positional word would be a struct); `command(verbose: verbose)` and calling
`command(...)` instead of `command.run(...)` (a value that is called hides
which function runs); a magic parameter named `options` (a name with a
meaning only in `main`); an anonymous `{verbose: false}` (no type to document
the fields); a silent rule that passes `main`'s options to the command without
a written argument (a reader does not see it); `..` only for a `Command`'s
values and `...args` (refused by the owner for ADR-call-spread-v1: a form that
works in one place only); `Command[...](args)` and a parameter pattern
`(command, args): Command[...]` (the owner refused it: the values belong to
one picked command and should not stand apart from it).

**Option A** is chosen (the owner, November 21, 2026, #449).

## 4. Decision

### D1 — `main`'s parameters are the command line.

`main`'s subject parameters (Part I 5.1) are the program's **positional
arguments**, in order, each required unless its type is `T?`: such a
subject may be left out at the end and is then `null`. Its options are **named arguments**,
`--name value` or `--name=value`, each taking its default when left out; an
`_` in a name is written `-` there (`max_depth` is `--max-depth`). An option of
type `bool` is a bare `--name`, which sets it to `true`; `--name=false` sets it
to `false`. Named arguments may stand anywhere among the positional ones, and
`--` ends them: every word after it is positional. A word that is `-` followed by a digit is a value, never a name, so `prog -3` passes -3. A word that names no option
is a wrong word (D4).

### D2 — What a word can be.

A parameter of `main` is `int`, a fixed-width integer, `f64`, `bool`,
`String`, or one of these behind `?`; the last subject parameter may be
`Vec[T]` of one of them, which takes every positional word left, none
included. An option of `main` may also be a struct whose fields are each one
of these, each with a default (D8); a subject of `main` may also be a
`Command[f, g, …]` (D9), then its only subject. A number is read in the shape
`std` reads one from text (Part III 17.1) and must fit its type. A subject of
type `T?` stands after every required subject and before a last `Vec[T]`. Any
other type, a `ref` parameter, a required subject after a `T?` one, and a
parameter of `main` on a target that has no command line (`wasm32-unknown`)
are refused while the program is built (`NK1273`); so is an option named
`help` (`NK1274`). The same holds for each function a `Command` names: its
parameters are what `main`'s may be, without a `Command` of its own.

### D3 — `main`'s preconditions are derived as every function's are.

What becomes a precondition of `main` is what ADR-269 D5 and D14-D21 make one
for any function: a claim carried back to the entry - through `let`s,
branches, guards and the preconditions of the calls it makes - that then reads
only parameters, at least one of them, with its path; one that reads none the
prover decides, proved or refuted as any claim is. `assert(a > 0)` at the top is one;
`positive(10 + a)`'s `assert(whole > 0)`, carried back through `example`, is
`10 + a > 0`. In the body each is a fact, as it is in any function.

### D4 — They are checked where the words arrive; a breach is a usage error.

Before `main`'s body runs, the words are read into the parameters and every
precondition of `main` is checked; this is the check where outside data
arrives that ADR-269 D6 asks for, so a claim about `main`'s parameters is not
refused with `NK1202`. A word missing, a word left over, a word that is not of
its type, or a precondition that does not hold, prints one line to standard
error in the program's terms - the parameter, the claim as written, the value
given, and for a claim carried back the `assert` it came from - then the short
usage, and ends the program with **exit status 2**. Nothing of the body has
run: no cleanup is owed and none runs. With a `Command`, the picked
function's preconditions are checked there too, before `main`'s body runs.

### D5 — The help is the interface.

`--help` and `-h` print the program's name, the `///` lines above `main`, and
each parameter with its type, its default and its preconditions, then end the
program with status 0. `--help` after `--` is a positional word. A struct
option is listed by its fields, each with the `///` lines above the field
(D8); a `Command` lists its functions (D12).

### D6 — The interface is in the ledger.

`main`'s parameters, defaults and preconditions are written in
`nikaia.contracts` as any function's are, so a change to what a program
accepts is a ledger diff in review.

### D7 — A call to `main` inside the program is an ordinary call.

A test or another function that calls `main(5)` proves its preconditions as
it proves any callee's. `cli::args()` still returns every word the program was
started with, so a program may read them itself, with or without parameters on
`main`. How a call to a `main` that takes a `Command` builds one is not
decided (§6); such a program's test calls the command's function itself.

### D8 — A struct option is flattened on the command line.

An option of `main` (or of a function a `Command` names) whose type is a
struct is not one word: each field is a named argument of its own, by the
field's name (`global: Global` with a field `verbose: bool = false` is
`--verbose`), with the field's default, read as D1 reads an option of that
type. The option's own default is the struct with every field at its
default; another default is refused (`NK1273`). The fields are checked for
D2's types (`NK1273`).

### D9 — `Command[f, g, …]` picks a function by the first word.

`Command[f, g, …]` is a type of the prelude that names functions of the
program, and is allowed only as the type of `main`'s one subject parameter
(elsewhere `NK1273`). The first positional word picks the function whose name,
with `_` written `-`, it is (`focus-next` is `focus_next`); the words after it
are read into that function's parameters as D1-D2 read `main`'s. A first word
that names no function is a wrong word (D4). The value has three fields:

* `run`, the function picked;
* `args`, a tuple of its subject values, in order;
* `options`, a struct of its option values, one field per option.

`command.run(..command.args; ..command.options)` calls the picked function
with the words given (ADR-call-spread-v1 D1, D2); each function's arguments
are checked as that written-out call.

### D10 — `args` and `options` fit only the `run` of the same value.

Because `run` is one of several functions, `args` and `options` have the type
of whichever was picked; they may be spread only into a call of `run` of the
same, unchanged `command` parameter, taken directly from it:
`command.run(..command.args; ..command.options, global: global)`. A copy
(`let a = command.args; command.run(..a)`), a spread into a named function
(`focus_next(..command.args)`) and a `mut command` are refused while the
program is built (`NK1358`).

### D11 — A name `main` declares is `main`'s.

After D8 flattens the struct options, every named argument on the command
line belongs to one parameter. A name `main` declares is `main`'s: the word
`--verbose` is read into `main`'s `global`. A function a `Command` names may
declare an option of the same name and type, for a test or another caller to
pass; it receives `main`'s value only through the call in `main`'s body,
which must name it (`command.run(..command.args; ..command.options, global:
global)`): a `command.run` call that leaves out an option `main` and the
picked function share is refused (`NK1356`), so a word given once is never
silently dropped. Two parameters that flatten to the same name inside `main`,
inside one function, or in `main` and a function with another type, are
refused (`NK1357`).

### D12 — The help lists the commands.

With a `Command`, `--help` prints the usage `prog [OPTIONS] COMMAND …`,
`main`'s options, and each function by its command-line name with the first
`///` line above it. `prog COMMAND --help` prints that function's help, as D5
prints `main`'s, followed by `main`'s options.

## 5. Reasons

* **The signature already is the shape of a command line.** Part I 5.1
  separates required, positional data from named options with defaults; D1
  reads that shape and adds no syntax.
* **One rule for contracts.** `main` is a function; its contract is derived,
  written and used as every function's is (D3, D6). Only its caller differs,
  and that caller is outside the program, so the check stands where the words
  arrive (ADR-269 D6) and fails as a message, not as a defect (D4).
* **Exit status 2** is the usage error scripts already test for.
* **A command is an ordinary function.** A test calls `focus_next(…; global:
  Global { verbose: true })`; nothing reads a context object. `main` shows the
  commands in its signature and the call that runs one (D9).
* **One owner per word.** A word on the command line belongs to one parameter
  (D11), and the hand-over to the command is written, so a reader sees where
  `--verbose` goes and the checker sees it is not lost.

## 6. Consequences

* **Part I 5.5** states D1-D5 and D7-D12; **Part III 14.2** says where `main`'s
  preconditions are checked, **17.1** that `std::cli` reads `main`'s
  parameters, **A.2** exit status 2; Part III's table carries `NK1273`,
  `NK1274`, `NK1356`, `NK1357` and `NK1358`.
* **What it costs.** A reader of the words and the check before `main`'s body,
  in the lowering of `main` and in `std::cli`; a help text; the report of
  `--asserts`. A program whose `main` takes nothing pays nothing. A
  `Command` lowers to a `match` on the first word, one arm per function.
* **What it risks.** Every rule of D1 is language now and stays; a later
  short flag can never be a digit (`-3` is a value). A first word that names
  a command can never be a positional word of `main`.

**Not decided here:** how a call to `main` inside the program builds a
`Command` (D7); `fs::Path` as a parameter type; short flags (`-v`); a
`Command` nested in a command; a data-less `enum` as a choice of words;
environment variables as defaults; a version flag; and whether `--help` lists
a default that is not a literal.

*Decided October 17, 2026 ([#635](https://github.com/Nikaia-Foundation/Nikaia/issues/635)), D8-D12 November 21, 2026 ([#449](https://github.com/Nikaia-Foundation/Nikaia/issues/449)).*
