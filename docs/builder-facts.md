# Builder facts

Fixed facts a builder would otherwise look up itself. `scripts/agent.nika`
puts *Core* into every builder's prompt, and after it the sections the brief
names (`Facts: Probes, Grammar`; a `##` heading brings its `###` sections
along), taken from `docs/builder-facts-lookup.md`; a brief without that line
gets the sections whose `Paths:` line matches its text, at most three (#2002).
*Core* holds only what every run needs, at most 120 lines: a trap of one topic
goes into its section of the lookup. That file is not read
whole: `nikaia run scripts/syntax.nika -- KEYWORD` prints its lines on a topic
together with the specification section and an example test.

## Core

### Where things live

| What | Where |
| :--- | :--- |
| Checker, moved parts | `crates/nikaia-std/src/compiler/check_*.nika`; Rust adapter in `crates/nikaia/src/check/` |
| Emitter | `crates/nikaia/src/emit/mod.rs`, `emit/host.rs`; moved parts `tools/emit_*.nika` |
| `std` methods by receiver | `std_receivers` in `tools/check_core_method.nika` |
| Generated Rust of a tools file | `compiler/lowered/<file>.rs`, index `compiler/lowered.rs` (never edit, `lower-std` writes them) |
| Program tests (new tests) | `tests/compiler/tests/NAME.nika` + `.stdout` / `.stderr` |
| Rust tests by NK code | `grep -rln NK1234 crates/nikaia/tests/it/` |
| Specification | `docs/specification/10-nikaia-light.md`, `20-nikaia-advance.md`, `30-nikaia-tooling.md` (code table) |
| A name, Rust or Nikaia | `nikaia run scripts/where.nika -- NAME...`: definitions with file:line and the files that call them; for an `NKnnnn` the tests and the spec row (a field is not found: grep it); `-- --outline FILE` lists each item of a file with its line range |
| A snippet through the built compiler | `nikaia run scripts/probe.nika -- 'let x = 1\nprintln(x)'` (`--check`, `--file F.nika`): runs it with `target/fast/nikaia`, prints at most 60 lines and `exit N`; a probe for a report is the snippet and these lines. A first `note: target/fast/nikaia is older than ...` line means the binary may predate HEAD: build it before reading a refusal as a defect (#1866) |
| How a form is written | `nikaia run scripts/syntax.nika -- KEYWORD`: the specification section, an example test and the facts lines on it (e.g. `slice`, `view`, `throws`) |

### Every run

* **Check the binary against HEAD before the first `nikaia test` or probe**: `git log --oneline --since=@$(stat -c %Y target/fast/nikaia) -- crates/` lists the compiler commits `target/fast/nikaia` predates; any line means build it first (`cargo build --profile fast -p nikaia`). A stale binary shows as a refusal in code you did not touch, often the `tests/compiler` library refusing a test before it runs. (#1893)
* **Copy `target/fast/nikaia` to `$S/old/nikaia` before your first build** (its own directory, named exactly `nikaia`: script tests check the basename and otherwise fall back to `target/fast/nikaia` under their tmp dir, *no such file ... target/fast/nikaia*, #1868): it is the before binary for the fails-before run and the binary for long test runs, since another agent may rebuild `target/fast/nikaia` mid-run. It may predate HEAD; when a fails-before result surprises, build the before binary from HEAD's sources first. `target/release/nikaia` is older than the views syntax. (issue-1483)
* **A touched test that fails on `$S/old/nikaia` too is red on `develop`, not yours**: one run there settles it; `git fetch origin develop && git log --oneline HEAD..origin/develop -- tests/compiler/tests/NAME.*` shows whether a re-record landed. Name it under `Tests:` and go on. (#1902)
* **Long runs go in the background** (`run_in_background`, `timeout: 3600000`), and you wait for the wake-up, never a `sleep` or `pgrep` loop; `&` on one command, never on a chain. **One `lower-std` or build at a time**: two `lower-std` runs write the same lowered files and one silently undoes the other. A cycle costs 10-15 minutes: find every change it needs with probes on the old binary first. (issue-1711)
* **Solver work (`packages/logic`) keeps each try short** (#2012's run spent 93 minutes on tools, half in a single callgrind run and repeated builds): callgrind only on an instance that runs under 1 s natively, a large one once at the end; between tries `nikaia test --filter NAME`, the whole package once before the commit; `lower-std` and the compiler build once per commit, not per try, since the rod (`scripts/solver.nika run`) measures without them. A probe or `Found:` names one file as one path, `compare --files target/smtlib/qflra/sal/F.smt2`: `run`'s names start below the directory it was given, and `target/smtlib/ qflra/...` (two arguments) now stops at once instead of answering the whole set (#2006). `--cases N` and `--budget N` raise a budget without a copy of the repository; a `git archive` copy runs with `NIKAIA=/home/user/Nikaia/target/fast/nikaia` (the checkout's build). **A run stopped by `--timeout` prints no counters** (the process is killed): `run --stats --cases N` (or `--budget N`) ends the file at a budget instead and prints them, which replaces a temporary early `return` in the search; a temporary trace in a function without `throws` is `eprintln(f"...") catch {}` (ADR-392 D2: printing can fail, so a bare `eprintln` there is NK2605). (#2049)
* **Probes live in the scratchpad**, outside any project: `nikaia check FILE` inside `tests/compiler/tests/` reads the sibling files. A probe or test whose `main` prints needs `fn main() throws` (ADR-392), or it stops at NK2605.
* **The checkout is shared.** A file another agent has modified: stage only your hunks (`git diff FILE > $S/p.diff`, delete theirs, `git apply --cached`, check `git diff --cached FILE`; `git add -p` is refused) and list it under `Partial:`, not `Files:` (#1704). Never touch others' `UU` files. An edit of yours gone mid-cycle is in the driver's merge (`.git/MERGE_HEAD` exists): never type it again, wait for the merge to end (section *Shared checkout* of the lookup). Never `git reset` (unstage your own file with `git restore --staged FILE`), `git stash`, `git checkout .`.
* **`commit.nika` commits only your `Files:` / `Partial:` through a private index**: another agent's staged file stays staged and is named `left staged, not committed:`; a `git mv` is committed by listing both paths. A defect fix and a step in one file are two commits (the lookup's *Shared checkout*). (#1792)
* **`pre-push.nika` checks only what is staged**: run it with your files staged, then unstage them before `commit.nika`, which refuses files already staged. A stale badge prints `badges stale (the driver's)` and fails nothing, and `version` checks the version lines without the badges (`bump.py check --no-badges`): the badges are the driver's, never regenerate or commit them. (issue-1290, issue-1352, #2414)
* `cargo fmt -- FILE` fails here; run `cargo fmt --all` at the repository root (plain `cargo fmt` skips `crates/unsafe/*`, which pre-push checks) or `rustfmt --edition 2024 FILE`. **Never `nikaia fmt` a `crates/nikaia-std/src/compiler/*.nika`**: it reindents the whole file; format by hand there. (issue-1473, #1847)
* **Another agent's uncommitted `crates/nikaia-std/src/*.rs` that rustc refuses breaks every `nikaia run`** (NK1343 *Rustc output* on a file you did not touch): `git archive HEAD | tar -x -C $S/head` once, then `NIKAIA_SYSROOT=$S/head/crates target/fast/nikaia run scripts/commit.nika -- REPORT` from the checkout; never touch their file. (#1979)
* **Before planning**: `git log -400 --grep='#N'` (unbounded it passes the 120 s tool timeout) shows earlier `Part of #N` commits and the commit messages of earlier runs, which hold causes and paths already dropped; a debrief or report an issue cites may be missing from `docs/briefs/`. Check whether the issue is closed or a duplicate and run its probe on `develop`. (issue-1224, issue-1356, issue-1458)
* **Builders have no GitHub**: the prompt holds the issues the brief names; a missing one goes under `Deviation`. Name a commit by subject (`git log --grep`), never by hash. `docs/briefs/` is untracked: `Fixes #N` goes on the last code commit. (p9, issue-1453)
* A brief that needs a new NK code without naming a reserved one is blocked at once; search `docs/supervisor.md` and the brief for an NK block before planning. (issue-1318)
* **ADRs live in `docs/specification/adr/`** (there is no `docs/adr/`); search them with the Grep tool, since a `grep -r` from the root runs into `target/`. (issue-1397, issue-1280)
* A report or edit script written through a heredoc runs every backticked word as a command; quote the delimiter (`<<'EOF'`) and expand the paths separately. (issue-1458, issue-1485)
* `/usr/bin/time` does not exist here; time a command with the bash `time` keyword or `date +%s.%N` before and after. (issue-1470)
