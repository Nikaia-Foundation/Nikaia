# ADR-raw-sinks-1: Data From Outside the Program at a Raw Sink Is Proved, or Allowed in the Manifest

**Status:** Accepted
**Date:** November 16, 2026 (owner's decision on #1253: alternative C); D9
(no shell in `std`) added November 19, 2026 (owner's decision on
[#1937](https://github.com/Nikaia-Foundation/Nikaia/issues/1937)); D10
(a program from outside at `process::run`) added November 19, 2026 (owner's
decision on [#1938](https://github.com/Nikaia-Foundation/Nikaia/issues/1938)); D11-D16 (proof in
place of `trusted:`) added November 19, 2026 (owner's decision on
[#2242](https://github.com/Nikaia-Foundation/Nikaia/issues/2242)), which change D8;
D17 (the compiler's own build paths) added November 21, 2026 (owner's decision on
[#2442](https://github.com/Nikaia-Foundation/Nikaia/issues/2442)); D17 items 6-8
added the same day (supervisor's decision on
[#2460](https://github.com/Nikaia-Foundation/Nikaia/issues/2460)); D7's
`html::Raw` check added the same day (supervisor's decision on
[#2650](https://github.com/Nikaia-Foundation/Nikaia/issues/2650)); D18 (an
argument of `process::run` read as an option) added November 22, 2026
(owner's decision on
[#1939](https://github.com/Nikaia-Foundation/Nikaia/issues/1939),
alternative C), which changes D9 and D10; D18 item 6 (a leading `/`) added
the same day (owner's decision on
[#2718](https://github.com/Nikaia-Foundation/Nikaia/issues/2718),
alternative C); D19 (a shell or interpreter named as the program) added the
same day (owner's decision on
[#2717](https://github.com/Nikaia-Foundation/Nikaia/issues/2717),
alternative B), which changes D9 item 3
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
[ADR-database-v1](adr-database-v1.md) D5 (dynamic SQL); [ADR-380](adr-380.md) D2 and
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
   block, ADR-database-v1);
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
| `db::raw(text)` | `db::is_identifier(part)`: `[A-Za-z_][A-Za-z0-9_]*` | `untrusted-sql` |
| `html::Raw(markup)` | `html::is_plain(part)`: none of `& < > " '` | `untrusted-markup` |
| a path under `fs::Root::Anywhere` or `Dir("/")`, in `std::fs` and `http::File` | none: only a value that is not outside passes | `untrusted-paths-anywhere` |
| the `program` of `process::run` (D10) | none: only a value that is not outside passes | `untrusted-program` |
| the start of an element of `process::run`'s `args` (D18) | `process::is_operand(part)`: does not start with `-` or `/`; or a literal `"--"` earlier in the list, which proves only the `-` part | `untrusted-option` |
| an element of `args` with an outside part, when the program names a shell or interpreter (D19) | none: only the manifest key lets it build | `untrusted-shell` |

A call is accepted when each outside part either:

1. is not outside: a literal, a constant, a value from a source marked
   `trusted: true` (ADR-010 D3, ADR-365 D2, ADR-399), a value the operator
   chose (the environment, ADR-397 D1);
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
`untrusted-shell` names no sink since D9; it is kept for the shell library a
later decision brings (D9).
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
it, the part escaped as the log escapes it. Every `fs` call and
`db::Raw::execute` throw already. `html::Raw` checks where its markup is
written, a call that throws.

`html::Raw` is checked where it is made (supervisor's decision on
[#2650](https://github.com/Nikaia-Foundation/Nikaia/issues/2650), November 21,
2026). The template that writes the markup and `html::Render` stay as they
are: a template does not throw, and a value does not carry a breach past the
line that made it. Under `warn`, each part from outside that the compiler did
not prove at an `html::Raw(...)` site is checked with `html::is_plain` where
the value is made, and a part that fails throws
`html::HtmlError::NotPlain(part)` from that site, the part escaped as the log
escapes it. `std::html` declares that error. The site throws only in a
function that throws already: in one that does not, `warn` does not lower the
site, and it stays `NK1308`, because a site whose check cannot throw is a
breach with no check at all.

`db::raw` is checked where it is made, too (supervisor's decision on
[#2705](https://github.com/Nikaia-Foundation/Nikaia/issues/2705), November 22,
2026). The `Raw` value keeps only its joined text, so by `execute` the parts
are gone; the rule for `html::Raw` above applies to `db::raw(...)` as it
stands: each unproved outside part is checked with `db::is_identifier` at the
site, and a part that fails throws `db::NotIdentifier { part }` from it, the
part escaped as the log escapes it. The `db` package declares that error, as
`NullInField` is declared. The site throws only in a function that throws
already, else it stays `NK1308`. `execute` stays as it is.

The thrown error is in the function's throw set before the prover has run
(supervisor's decision on
[#2704](https://github.com/Nikaia-Foundation/Nikaia/issues/2704), November 22,
2026). The throw set is inferred before the claim prover decides which sites
it proves, so under `warn` the inference counts every `html::Raw(...)` and
`db::raw(...)` site with a part that is not a literal or a constant as
throwing the sink's error. The set may then be wider than what the program
can throw where the prover later proves the part; it is never narrower. The
prover's proof removes only the check at the site, not the entry in the set.
Under `error` nothing changes: a site is proved or refused, and no sink error
enters any set.

A part that reaches a site through a parameter is checked at that site, in
the function that makes the value, for every call (supervisor's decision on
[#2773](https://github.com/Nikaia-Foundation/Nikaia/issues/2773), November 22,
2026). The caller only passes the value on; the breach is the line that makes
the markup or the SQL, so the check and the error belong there, as for any
other part. Where the prover proves the parameter at every call of the
function (ADR-396 D1), the check goes; where one call leaves it unproved, the
check stays and runs for every call. A finding the prover reports at the call
is not a check, and a site that is neither proved nor checked does not build
under `warn`: it stays `NK1308`, as in a function that does not throw.

### D8 — It is switched on when our own code builds under it.

As ADR-396 D4: the build that adds `NK1308` and `NK1309` also makes every site
in the compiler, `packages/`, `examples/`, `tools/` and `scripts/` build.
Changed by D16: a site builds by a proof of D11-D15, not by `trusted:`;
the only `[allow]` lines in our own code are the narrow ones of D15, where
taking a path from outside is what the tool is for.

### D9 — `std` runs no shell; `process::shell` is refused, `NK1314`.

The owner's decision on [#1937](https://github.com/Nikaia-Foundation/Nikaia/issues/1937), for every platform alike, Unix
included:

1. `std` has no `process::shell`. A call to it is refused while the program
   is built, `NK1314` ("no shell is configured"). The help writes the same
   command as `process::run(program, args)` (ADR-243), whose arguments reach
   the program one by one and are never read as code.
2. The precondition `process::is_word` leaves `std` with it. It was written
   for `/bin/sh`; under `cmd.exe` a `%` expands a variable, and no predicate
   was checked against PowerShell. A precondition not checked against the
   shell that reads the text is worse than no shell.
3. What a shell is wanted for (a pipe, a redirection, `&&` and `||`, a glob)
   comes later as a structure, not as a text: a list of commands, each a
   program with its arguments, and where each stream goes. A value from
   outside is always one argument, so nothing is escaped and nothing has to
   be proved. A library may run such a structure without a shell. A library
   that does hand a text to `sh` or `cmd.exe` is the shell library this
   record keeps `untrusted-shell` for: its escaping is its own, and it builds
   in a program only where the application allows it (D5). D19 makes the
   compiler see it when the program is named in `process::run`.

`process::run` stays as it is. Its program is a raw sink by D10, and the
start of each argument by D18, and every argument to a shell by D19.

### D10 — The program of `process::run` is a raw sink; an outside one is refused, `NK1308`.

The owner's decision on [#1938](https://github.com/Nikaia-Foundation/Nikaia/issues/1938):

```nika
let tool = req.query("tool") ?? "convert"
process::run(tool, [name, "thumb.png"])
```

runs whatever program a stranger names. No precondition makes a stranger's
choice of program safe: a name without a separator is looked up on `PATH`, a
name with one runs that file. So the `program` of `process::run` is a raw sink
with no precondition, as a path under `fs::Root::Anywhere` is (D2): a part that
is not outside passes, any other is refused, `NK1308`, unless the manifest
allows `untrusted-program` (D4, D5). The arguments are not this sink: each
reaches the program as one word and is never read as code (ADR-243). Changed
by D18: one word can still be read as an option, so the start of each
argument is a sink of its own.

A value is not outside when it is a literal, a constant, or chosen by the
operator: the environment is the operator's except a CGI host's names
(ADR-397 D1, D2), so `env::var("EDITOR") ?? "vi"` passes in a command-line
tool. Where the environment is untrusted, by a host's name or by
`[build] environment = "untrusted"` (ADR-397 D4), the help writes the choice
as a `match` over the programs the program means, whose arms are literals and
so not outside:

```nika
let editor = match env::var("EDITOR") ?? "" {
    "vim" => "vim",
    "nano" => "nano",
    else => "vi",
}
```

A privileged start of a program that has not declared its environment
untrusted ends before `main` (ADR-397 D3), so an operator's `EDITOR` never
reaches a setuid program unchecked.

### D11 — The root of `Dir` is a part of the path.

`fs::Root::Dir(dir)` keeps a name inside `dir` (ADR-108 D3), but it keeps it
inside a directory a stranger may have chosen. So `dir` is judged as a part
of the path under D2: a `dir` from outside makes the call a raw sink with no
precondition, as `Anywhere` does, `NK1308` under `untrusted-paths-anywhere`
or the operation's key of D15. A literal, a constant, a directory the
operator chose (the environment, ADR-397), a root `std` made (D12) and a
root built by `Root::join` from one of these (D13) pass.

### D12 — A directory `std` made is a root, and it replaces `mktemp`.

`fs::scratch()` (ADR-247 D5) makes a fresh, empty directory and hands it
back as a `Root`. Nobody but the program chose it, so it is not outside and
what is written under it with names that are not outside is proved. It is
the form for a temporary directory: `process::run("mktemp", ["-d"])` hands
back standard output, which is outside, and a path joined to it under
`Anywhere` is refused. Where the directory's name is needed as text, for a
program's argument, `root.path()` gives it (a method is called on its value;
`Root::path(root)` is refused, supervisor, 2026-11-19, #2336); an argument is
not a raw sink (D10). `Root::path` returns `fs::Path`, never `null` (supervisor,
2026-11-19, #2326): for `Dir(d)` it is `d`; for `Anywhere`, which opens a
relative path as given, against the working directory, it is `"."`, so
`f"{root.path()}/x"` names the file `root` opens for `"x"` under either
root. A test of `fs::scratch()` thus writes no fallback for a case that
cannot occur.

### D13 — A name from `fs::read_dir` is opened under its directory.

`fs::read_dir(path, root)` hands back names the file system chose: outside
data. Each is one component, with no separator, never `.` or `..`, so it
cannot climb, but it may be a symlink to anywhere, planted by whoever can
write into the directory. The form that proves it is a root for the
directory, `root.join(path)`, which is a `Root::Dir` under the same check,
and the name opened under it:

```nika
let packages = root.join("packages")
for name in fs::read_dir(".", packages) {
    let manifest = fs::read_to_string(name.join("Cargo.toml"), packages)
}
```

`Dir`'s check resolves symlinks (ADR-108 D3), so a planted link out of the
directory is `io::IoError::Outside` at the call. The same name joined to a
text and opened under `Anywhere` stays outside, `NK1308`: it follows the
link wherever it points.

### D14 — A path from outside is proved by a property of its own.

Where a program does take a name from outside and opens it, the guard where
it arrives proves one of two predicates of `std::path`, and the proof holds
after the guard as `html::is_plain`'s does (ADR-396 D2):

| Predicate | Holds when | What it admits |
|---|---|---|
| `path::is_component(p)` | one component, no separator, not `.` or `..` | `p` under a `Dir` whose root passes D11, or joined to a part that is not outside |
| `path::is_below(p)` | relative, no root, no drive, no `..` component | the same |

"Joined to a part that is not outside" (supervisor, 2026-11-20, the
question of #2358): the guarded name with a literal or another clean part
before it, as in `f"{store}/{key}/bin"`, passes under `Anywhere`; the bare
name under `Anywhere` does not, since it names a file wherever the program
runs. The proof holds for the joined text as for the name: a binding made
from it (`let kept = f"{store}/{key}/bin"`) carries it to the sink.

The same join admits the program `process::run` runs (D10; supervisor,
2026-11-20, the question of #2369): the guard keeps the name inside a
directory the program chose, as it does for a file it opens. A whole
program name from outside, with no clean part before it, stays `NK1308`:
it is looked up on `PATH` or names any file.

A function hands back its proof (supervisor, 2026-11-20, #2375): a call is
not outside where every value its function returns is proved, a literal (D10)
or a guarded name joined to a clean part. A parameter the function hands back
unguarded is not proved there: it carries what the caller passes, so the call
is outside exactly where the argument at that place is outside, and a clean or
guarded argument keeps it clean. One returned value from outside makes the call
outside, as one arm does for a `match`.

Neither proves where a symlink points; under `Dir` the run time check does
(ADR-108 D3), under `Anywhere` nothing does, as for a literal. A check of the
file system (`fs::exists`, a `metadata` call) proves nothing: the answer is
about the moment of the call, not the moment the file is opened.

### D15 — The sink is the operation, and an exception names its sources.

A path's danger depends on what is done with it. A path in `std::fs` and
`http::File` is one of three sinks, each with its own key:

| Operation | Calls | `[allow]` key |
|---|---|---|
| read | `read_to_string`, `read`, `map`, `open` for reading, `read_dir`, `metadata`, `exists`, `canonical`, the source of `copy`, `http::File` | `untrusted-paths-read` |
| write | `write`, `create`, `open` for writing, `create_dir`, the target of `copy` and `rename` | `untrusted-paths-write` |
| remove | `remove`, the source of `rename` | `untrusted-paths-remove` |

`untrusted-paths-anywhere` is kept and means all three. A key takes `true`,
every source, or the list of sources it admits, named as `--trust` names
them (ADR-010 D7): `args`, `stdin`, `env` (an untrusted environment,
ADR-397 D4), `request`, `file`, `process`, `network`.

```toml
[allow]
untrusted-paths-read = ["args"]
```

A checksum tool built with this line reads every file named on its command
line. The same name at `fs::write` or `fs::remove` is still `NK1308`, and a
file name read from standard input is still `NK1308` at `fs::read`: each
operation and each source is admitted on its own. D5 holds per key and per
source: `[allow.packages.NAME]` names the same keys with the same lists.

### D16 — `trusted:` is not the way a raw sink is passed.

The owner's direction on #2242: an option that says *trusted* at the source
is boilerplate where it is needed and dangerous where it is written by
habit; the program proves what it does instead. So:

1. A value marked `trusted: true` at its source still counts as not outside
   under D2 (ADR-010 D3, ADR-399), but no further source gets a `trusted:`
   option for a raw sink's sake. The `trusted:` of `io::read_line` and
   `io::read_exact`, added by #2242, is taken out again.
2. Our own code passes its raw sinks by D11-D15: `fs::scratch()` for a
   temporary directory, a root per directory it walks, the predicates of
   D14 where a name arrives, and a D15 line in the manifest of a tool whose
   job is to read the files its command line names (`nikaia` itself reads
   the program it is given). The line names the operation and the sources,
   never `true`, and the manifest says in a comment why.
3. A site none of these covers is a finding: it is filed and decided, not
   written around (CLAUDE.md, *Fix, do not work around*).

### D17: The compiler proves its own build paths, and states its allowances in a manifest

Added November 21, 2026 (owner's decision on
[#2442](https://github.com/Nikaia-Foundation/Nikaia/issues/2442), alternative
C). The compiler's rustc wrapper (`project_drive.nika`, run by Cargo in
rustc's place) reads paths it did not choose: the input files a Cargo
dep-info file names, the files of the compiler's own record, and the
`--out-dir` Cargo passes on the command line. They are data from outside
(`file` and `args`), and under D15 each would need an `[allow]` line. They get
none:

1. **Reads are kept below the generated sources.** Before the wrapper reuses
   a cached run, every path the dep-info file and the record name must be
   proved below the directory in `NIKAIA_GEN_DIR` (`path::is_below`, D14),
   and is read through a root on that directory (D12).
2. **Writes are kept below the target directory.** `--out-dir` must be proved
   below the directory in `CARGO_TARGET_DIR`, and outputs are written through
   a root on it.
3. **A path outside is not an error.** The wrapper then caches nothing for
   that run and lets rustc run as it would without the wrapper. A build is
   never refused because of where Cargo put a file; it only loses the cache.
4. **The limit, stated.** Both roots come from the environment, which the
   outer `nikaia build` sets before it starts Cargo; the wrapper has no other
   channel from the build. Whoever sets that environment chooses the roots.
   The proof keeps every read and write inside the directories the build
   named; it does not prove that the build named the right ones.
5. **The compiler has a manifest.** `crates/nikaia-std/nikaia.toml` is the
   compiler's own manifest, with an `[allow]` section that is empty. The
   self-check of the raw sinks in `std` (`project_check.nika`) reads its
   allowances from it instead of a list written into the code, and `--trust`
   shows them, so a later allowance is a visible line in a reviewed file,
   never a change inside the checker. Its place is fixed by item 8.
6. **The write root is `NIKAIA_TARGET_DIR`.** Cargo does not pass
   `CARGO_TARGET_DIR` to the wrapper when the build names `--target-dir`, and
   exporting it would make the compiler's nested Cargo runs share the outer
   target directory and wait on its lock (#1774). The outer build therefore
   sets `NIKAIA_TARGET_DIR` to the `--target-dir` it hands Cargo, beside
   `NIKAIA_GEN_DIR` (`project_drive_cargo_env`); item 2 reads that variable,
   not `CARGO_TARGET_DIR`. Without it the wrapper keeps no store (item 3).
   The copy out of the store into the target directory is
   `fs::copy_between`, which keeps the file's mode, and the variable is
   handed through by the Rust host; both are platform primitives of
   [ADR-platform-layer-v1](adr-platform-layer-v1.md) D7, so the choice
   around them stays in Nikaia.
7. **The outer build's record has the outer build's roots.** The record that
   lets a build skip Cargo (#1750, `project_drive_fresh` and
   `project_drive_record`) is kept by the outer build, not the wrapper, and
   the dep-info file it reads names the program's sources and `std`'s, not
   only generated ones. Every path in it, and the binary and the record
   themselves, must be proved below one of the roots the outer build already
   holds: the project's root, the `std` sources of the sysroot and the target
   directory, and is read through a root on it. A path below none of them
   means no record and Cargo runs (item 3).
   The host hands those three roots through as accessors on
   `ProjectDriveHost` (ADR-platform-layer-v1 D5, D7); choosing them stays
   in the Rust host until the outer build moves to Nikaia (#125).
   What the roots are (supervisor's decisions on
   [#2700](https://github.com/Nikaia-Foundation/Nikaia/issues/2700) and
   [#2699](https://github.com/Nikaia-Foundation/Nikaia/issues/2699),
   November 22, 2026):
   * **The `std` sources are the sysroot's `crates/` directory**
     (`Sysroot::resolve().root()`), not `std_dir()`: `std` builds with its
     path dependencies under `crates/unsafe/`, and their files are in every
     program's dep-info.
   * **The project's root includes its members.** Each member's root
     (`ProjectMembersMember.root`, a path dependency the manifest names) is
     a root of the record as the project's root is: the manifest the operator
     wrote chose it.
   * **A test batch adds its own directory.** For a `nikaia test` batch the
     directory of the batch, which holds its carrier project and its
     programs, is a root: `nikaia test` made it (D12), and it names the
     programs in `NIKAIA_TEST_BATCH`.
   No other root is added; a path below none of these still means no record.
8. **The manifest lies beside the sources, not above them.** The manifest of
   item 5 is `crates/nikaia-std/trust/nikaia.toml`. A `nikaia.toml` in a
   directory above the `std` sources would make every one of them a member of
   a project (`Layout::resolve` takes the nearest `nikaia.toml` above a
   file), so `nikaia check` of a compiler file would become a project build.
   A directory beside them is above none of them; the file keeps the name and
   form of every manifest, and `project_check.nika` reads it by its path.

### D18 — An argument of `process::run` that may be read as an option is a raw sink, `NK1308`.

Added November 22, 2026 (owner's decision on
[#1939](https://github.com/Nikaia-Foundation/Nikaia/issues/1939),
alternative C).

```nika
let name = args[1]
process::run("rm", [name])
```

removes one file, until a stranger names it `-rf`, or `tar` is handed
`--checkpoint-action=exec=...`. The argument reaches the program as one word
(ADR-243), but the program reads a word that starts with `-` as an option, and
an option can name a program to run or a file to write. So:

1. **The start of each element of `process::run`'s `args` is a raw sink.** An
   element whose first part is not outside passes, whatever follows it:
   `"--output=" + name`, `"./" + name` and `f"-I{dir}"` pass, because the
   option or the path was written by the program. An element whose first
   part is outside passes only when it is proved (2) or (3); otherwise it is
   refused, `NK1308`, unless the manifest allows `untrusted-option` (D4, D5).
2. **The predicate is `process::is_operand(part)`**: the text does not start
   with `-`. Like every predicate of D2 and D14 it is a guard, and a function
   that checked it hands back its proof (D14). The empty text holds it.
3. **A literal `"--"` earlier in the list proves every element after it.** It
   is the end of the options for every program that follows POSIX's utility
   conventions, so `process::run("rm", ["--", name])` passes. A `"--"` that is
   not a literal, or that stands after the element, proves nothing.
4. **A value made under a root passes.** `Root::path` and `Root::join` (D11,
   D14) start with the root's own text, which is not outside, so a path of a
   `Dir` passes as an argument without a guard.
5. **Option B of #1939 is moot.** It made `process::is_word` refuse a leading
   `-`; D9 removed `is_word` and `process::shell` with it. What a shell
   library escapes is its own (D9 item 3).
6. **A leading `/` is refused as well, on every target** (owner's decision on
   [#2718](https://github.com/Nikaia-Foundation/Nikaia/issues/2718),
   alternative C). Windows programs read a word that starts with `/` as an
   option (`robocopy /PURGE`, `findstr /G:file`), and a program is built once
   for every target, so `process::is_operand` refuses a leading `/` too. A
   literal `"--"` ends only the `-` options, so it proves only the `-` part:
   an outside element after it still needs `is_operand` against `/`. A path
   that is meant passes as `"./" + name` or under a root (item 4); an outside
   absolute path is refused on Unix too, which is D14's rule anyway: a path
   from outside goes through a root.

The run time check under `raw-sinks = "warn"` (D7) throws `process::run`'s own
error, `io::IoError::Other`, naming the argument, before anything starts.

### D19 — A shell or interpreter named as the program makes every outside argument a raw sink, `NK1308`.

Added November 22, 2026 (owner's decision on
[#2717](https://github.com/Nikaia-Foundation/Nikaia/issues/2717),
alternative B).

```nika
let name = req.query("file") ?? ""
process::run("sh", ["-c", f"convert {name} thumb.png"])     // name = "x; rm -rf ~"
process::run("cmd.exe", ["/c", f"type {name}"])              // name = "x & del /q *"
```

passes D10 (the program is a literal) and D18 (the element starts with a
literal), and the text runs as shell code, which D9 forbids. So:

1. **`std` keeps a list of shells and interpreters.** The shells `sh`,
   `bash`, `zsh`, `dash`, `ksh`, `fish`, `cmd`, `powershell` and `pwsh`; the
   interpreters `python`, `python3`, `perl`, `ruby` and `node`; and `ssh`,
   whose remote side runs its text in a shell. A literal program matches on
   its base name (`/bin/sh`, `sh.exe`), without case on Windows. The list
   lives in one place in `std`, and the help names it.
2. **When the program is on the list, every element of `args` with an outside
   part is a raw sink**, wherever the part stands, under the key
   `untrusted-shell` (D4, D5), which D9 kept for this case. Not only the
   element after a code flag: `sh -ec`, `bash --norc -c` and `cmd /s /c`
   would slip past a rule that looks for one flag.
3. **No predicate proves it**, for D9 item 2's reason: no escaping has been
   checked against every shell. Only the manifest key lets it build. A
   program that wants a pipe or a glob of its own writes it with literals or
   with values that are not outside, or runs the commands one by one with
   `process::run`; it does not allow the key for its own convenience.
4. **A runner passes the check on to its first operand.** `env`, `sudo`,
   `nohup`, `timeout`, `nice` and `xargs` start a program named among their
   arguments, so `process::run("env", ["sh", "-c", text])` counts as
   `process::run("sh", ["-c", text])`. The first operand is the first
   element that does not start with `-` (for `timeout` and `nice`, after
   their number). That operand is a program, so D10's rule holds for it: an
   outside one is refused under `untrusted-program` unless proved.

   *Amendment (supervisor, 2026-11-22, #2723, by the obvious-decision rule
   of `docs/supervisor.md`):* "does not start with `-`" alone lets
   `process::run("sudo", ["-u", "root", "sh", "-c", text])` through, because
   `root`, the value of `-u`, is taken for the program. So each runner
   carries a table of the options it knows, and which of them take a value:
   the value is skipped with its option, `env`'s `NAME=VALUE` operands are
   skipped, and `--` ends the options. An option that is not in the runner's
   table, an option the table marks as running a command line of its own
   (`env -S`/`--split-string`, `sudo -s`/`-i`), a non-literal element before
   the first operand, or no operand at all leaves the program unknown, and
   the call is checked as a shell (this D19): refused under `untrusted-shell`
   unless every argument is proved. The check fails closed: what the table
   does not know is never taken for safe.
5. **A program that is not a literal** is refused by D10 unless proved, so
   the list only has to match literals; a program proved by a `match` over
   literals (D10) is checked for each literal of the match.

The run time check under `raw-sinks = "warn"` (D7) throws
`io::IoError::Other`, naming the program and the argument, before the
process starts.

## 4. Not decided here

* **Names and codes** are confirmed by the supervisor (2026-11-16, #1253):
  the keys (`untrusted-shell`, `untrusted-sql`, `untrusted-markup`,
  `untrusted-paths-anywhere`, `untrusted-program` of D10, confirmed
  2026-11-19, and `untrusted-option` of D18, confirmed 2026-11-22),
  `[allow.packages.NAME]`, `--allow=KEY` and
  `--allow=PACKAGE:KEY`, the predicates of D2, and the codes `NK1308` and
  `NK1309`, reserved after `NK1303`.
* **A tool list in the manifest** for `process::run`'s program. A `match`
  over literals (D10) names the programs where they are used; a list beside
  it would be a second place. It is taken up if a case shows the need.
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
  precondition where its module is described (`std::fs`, `html`,
  `http::File`, `db`), says that `std::process` runs no shell (D9), and gives
  `process::run`'s program its rule (D10), and its arguments theirs with
  `process::is_operand` (D18), and the list of shells and interpreters (D19). Part III 13.3 names the keys
  `untrusted-program` and `untrusted-option`. C.3
  lists `NK1308`, `NK1309` and `NK1314`.
* Part I 1.3 says that outside data at a raw sink is proved or allowed.
* ADR-010 D8 and ADR-108 D5 carry a note.
* Built by [#1253](https://github.com/Nikaia-Foundation/Nikaia/issues/1253); D9 by
  [#2197](https://github.com/Nikaia-Foundation/Nikaia/issues/2197); D10 by
  [#2217](https://github.com/Nikaia-Foundation/Nikaia/issues/2217); the environment by
  [#2216](https://github.com/Nikaia-Foundation/Nikaia/issues/2216); D11-D16 by the issues
  listed on [#2242](https://github.com/Nikaia-Foundation/Nikaia/issues/2242); D17 by
  [#2442](https://github.com/Nikaia-Foundation/Nikaia/issues/2442) and
  [#2460](https://github.com/Nikaia-Foundation/Nikaia/issues/2460); D18 by
  [#2694](https://github.com/Nikaia-Foundation/Nikaia/issues/2694); D18 item 6 and D19 by the
  issues listed on [#2718](https://github.com/Nikaia-Foundation/Nikaia/issues/2718) and
  [#2717](https://github.com/Nikaia-Foundation/Nikaia/issues/2717).
* Part III 13.3 names the keys of D15 and their source lists; Part III 17
  (`std::fs`, `std::path`) gives `Root::join`, `Root::path`,
  `path::is_component` and `path::is_below`.
