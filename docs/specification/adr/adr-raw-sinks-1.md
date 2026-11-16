# ADR-raw-sinks-1: Data From Outside the Program at a Raw Sink Is Proved, or Allowed in the Manifest

**Status:** Accepted
**Date:** November 16, 2026 (owner's decision on #1253: alternative C)
**Answers:** what the compiler does when data from outside the program reaches
a place that reads text as code or as a location: a shell command, an SQL
text, markup written as is, a path with no root to keep it in
**Changes:** [ADR-010](adr-010.md) D8 (a path to `fs::Root::Anywhere` becomes
a consumer of provenance) and [ADR-108](adr-108.md) D5 (an `Untrusted` value
at such a path is refused while the program is built)
**Related:** [ADR-010](adr-010.md) D2, [ADR-365](adr-365.md) D1,
[ADR-397](adr-397.md), [ADR-399](adr-399.md) (which data is from outside);
[ADR-017](adr-017.md) and [ADR-398](adr-398.md) (escaping in templates and in
the log); [ADR-243](adr-243.md) (a process without a shell);
[ADR-299](adr-299.md) D5 (dynamic SQL); [ADR-380](adr-380.md) D2 and
[ADR-396](adr-396.md) D1, D2, D6 (a claim on outside data is proved, and the
way out is a guard)

## 1. Question

```nika
fn thumbnail(req: Request) -> String throws {
    let name = req.query("file") ?? ""
    let run = process::shell(f"convert {name} -resize 64x64 thumb.png")
    return run.stdout
}
```

A request with `file=x;rm -rf ~` runs a second command. The same holds for an
SQL text built with an f-string, for markup passed to `html::Raw`, and for a
path from a request opened under `fs::Root::Anywhere`. The language knows
where the value came from (ADR-010) and already proves claims about outside
data (ADR-396). The question is whether it refuses such a call, and how a
program says that it means it.

## 2. How others do it

* **Perl's taint mode** marks every value read from outside and stops the
  program at run time where a marked value reaches a system call; a regular
  expression match clears the mark, whatever it matched. ADR-108 rejected this
  for paths.
* **Ruby** had `$SAFE` levels with the same per-value mark and removed them in
  3.0: the mark travelled where nobody expected it and was cleared by habit.
* **Rust, Go, Python** offer the safe form (`Command` with arguments,
  prepared statements) beside the raw one and check nothing.
* **Static taint checkers** (CodeQL, Semgrep, the Checker Framework's tainting
  checker) report a flow from a source to a sink without a sanitizer as a
  finding beside the build.
* **Trusted Types** in browsers refuse a string at a DOM sink unless a policy
  the page registered made it; the policy is set per page, not per library.

## 3. Decisions

### D1 — Every boundary escapes by default.

The form a program writes first is the one that cannot be steered:

1. SQL takes its values as parameters (`execute(conn; parameters:)`, a `dsl`
   block, ADR-299);
2. a process takes its arguments as a list, with no shell between (ADR-243);
3. a path names its root, and `Dir` keeps it inside (ADR-108);
4. text written to the log has its control characters and terminal sequences
   escaped (ADR-398); so does text that `std::cli`'s styled output writes,
   when that is built. Standard output, files and sockets are data and are
   written as they are (ADR-398);
5. an encoder in `std` or an official package writes every value so that it
   reads back as the same value: `json`, `yaml`, `toml`, `html::escape`,
   `db::Value::text`, a template (ADR-017). An encoder has no raw mode.

What reaches past these forms is a raw sink (D2).

### D2 — A raw sink carries a precondition, and an outside value at it is proved.

The raw sinks are a fixed list in `std` and the official packages. Each states
its precondition on the outside parts of its text:

| Sink | Precondition on each outside part | `[allow]` key |
|---|---|---|
| `process::shell(command)` | `process::is_word(part)`: not empty, only `A-Z a-z 0-9 _ . / : = @ % + , -` | `untrusted-shell` |
| `db::raw(text)` | `db::is_identifier(part)`: `[A-Za-z_][A-Za-z0-9_]*` | `untrusted-sql` |
| `html::Raw(markup)` | `html::is_plain(part)`: none of `& < > " '` | `untrusted-markup` |
| a path under `fs::Root::Anywhere` or `Dir("/")`, in `std::fs` and `http::File` | none: only a value that is not outside passes | `untrusted-paths-anywhere` |

A call is accepted when each outside part either:

1. is not outside: a literal, a constant, a value from a source marked
   `trusted: true` (ADR-010 D3, ADR-365 D2, ADR-399);
2. has a type that cannot carry an attack: a whole number, a float, a `bool`
   or an `enum` (its text is a number or a variant name);
3. is proved to meet the precondition, from a guard where the data arrives,
   by the prover of ADR-380 and ADR-396.

The parts are what the compiler sees where the text is built in the function:
each hole of an f-string and each operand of `+` on `String`, judged on its
own as ADR-398 judges the log. A value built out of sight is one part. The
walk follows the value through every call whose body it reads, and a
private function's sink on a parameter is asked at the call (ADR-396 D1).

### D3 — Unproved and not allowed is refused, `NK1308`.

An outside part that no case of D2 covers, at a sink whose key the manifest
does not allow (D4), refuses the program, `NK1308`. The message names the
sink, the part and its source; the help writes the guard to put where the
value arrives (`throw BadInput(name) if !process::is_word(name)`), the escaping
form of D1, and the `[allow]` line.

`[build] raw-sinks = "warn"` in the application's manifest, or `--raw-sinks=warn`
for one build, builds such a program anyway: each site is a warning, and the
precondition is checked where the call is reached. A breach throws from the
call (D7). The default is `error`. `raw-sinks` is a cache-key dimension, as
`unproved-claims` is.

### D4 — An intended exception is written twice: the raw API in the code, the key in the manifest.

A program that means to pass outside text to a raw sink calls the raw API by
name, which it does already: `process::shell`, `db::raw`, `html::Raw`,
`fs::Root::Anywhere`. And its manifest allows it:

```toml
[allow]
untrusted-shell = true
untrusted-paths-anywhere = true
```

`--allow=KEY` allows one key for one build and may be given more than once.
An allowed sink builds with no check, and `--trust` lists every such call with
its place and its key. A key is all or nothing for the package: the manifest
does not name a call.

### D5 — A library's exception builds only where the application allows it, `NK1309`.

A package's `[allow]` lets its own `nikaia check` and `nikaia test` build. In a
program that uses the package, its allowed sinks build only where the
application's manifest names the key for that package:

```toml
[allow.packages.thumbs]
untrusted-shell = true
```

Otherwise the program is refused at the package's call, `NK1309`, and the
message names the line to add. `--allow=thumbs:untrusted-shell` is the same for
one build. `raw-sinks` is read from the application only: a package's own
`[build]` is ignored, as every other build option of a package is (Part III
13.3).

### D6 — A `pub` function states what its sink needs.

A `pub` function's parameter that reaches a raw sink is outside data, as for
an index (ADR-396 D6). The body proves the precondition with a guard, or
states it, `assert process::is_word(name)`, published as `requires`
(ADR-269 D18, D20), and the obligation goes to each caller; otherwise the
package needs the `[allow]` key (D4).

### D7 — The run time check throws the sink's own error.

Under `raw-sinks = "warn"` a breached precondition throws from the call:
`io::IoError::Outside` for a path, checked as under `Dir(".")`; for the other
sinks the error the call already throws, with the sink and the part that broke
it, the part escaped as the log escapes it. `process::shell`, every `fs` call
and `db::Raw::execute` throw already. `html::Raw` checks where its markup is
written, a call that throws.

### D8 — It is switched on when our own code builds under it.

As ADR-396 D4: the build that adds `NK1308` and `NK1309` also makes every site
in the compiler, `packages/`, `examples/`, `tools/` and `scripts/` build. A
path the operator typed is read with `cli::args(trusted: true)` (ADR-399),
which states who chose it. No `[allow]` line goes into our own code to get a
site through.

## 4. Not decided here

* **Names and codes** are confirmed by the supervisor (2026-11-16, #1253):
  the keys (`untrusted-shell`, `untrusted-sql`, `untrusted-markup`,
  `untrusted-paths-anywhere`), `[allow.packages.NAME]`, `--allow=KEY` and
  `--allow=PACKAGE:KEY`, the predicates of D2, and the codes `NK1308` and
  `NK1309`, reserved after `NK1303`.
* **`process::shell` is new API.** It runs `/bin/sh -c` on Unix; the shell on
  Windows is open.
* **A process's program from outside.** `process::run(program, args)` with an
  outside `program` runs a program a stranger names. Whether that is a raw
  sink, and with which precondition, is open.
* **An argument that starts with `-`.** An outside word passed as an argument
  can be read as an option, by the shell and by `process::run` alike. A
  precondition against it is open.
* **A response header.** `http`'s response has no header setter yet; when it
  gets one, a CR or LF in a header value is refused by the encoder (D1).
* **A library and `raw-sinks = "warn"`.** This record reads it from the
  application only (D5), for D5's reason: a dependency must not decide for the
  application.
* **The website and the guide.** Until it is built they say "specified, not
  built yet" and link the issue that builds it.

## 5. Considered and rejected

**A: do nothing.** The safe forms exist and the raw ones are visible by name.
Visible is not checked: a raw call with a stranger's text builds and runs.

**B: a mark on each value.** Perl's taint mode, rejected by ADR-108: the mark
travels with copies, any match clears it, and the check runs when the attack
already arrived. Provenance here is a property of sources, proved while the
program is built (ADR-010).

**D: the exception in the code only.** Calling `process::shell` would be
enough. Then a dependency decides for the application which strangers may run
commands on its machine. The manifest line keeps that decision with the
application (D5).

**A guard that clears a value for every sink.** A check written by the program
that marks a value safe is Perl's regular expression again. Each sink has its
own precondition, and only a proof of that one admits the value.

## 6. Consequences

* Part III 13.3 names `raw-sinks`, `--raw-sinks`, `[allow]` and `--allow`.
  14.2 states the rule beside `NK1302`. Part III 17 gives each sink its
  precondition where its module is described (`std::fs`, `std::process`,
  `html`, `http::File`, `db`). C.3 lists `NK1308` and `NK1309`.
* Part I 1.3 says that outside data at a raw sink is proved or allowed.
* ADR-010 D8 and ADR-108 D5 carry a note.
* Built by [#1253](https://github.com/Nikaia-Foundation/Nikaia/issues/1253).
