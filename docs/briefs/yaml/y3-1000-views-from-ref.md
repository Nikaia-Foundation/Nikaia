---
agent: builder
model: opus
max_turns: 100
budget_usd: 25
---
# Brief: #1000, a struct or enum built in a function from views of a `ref` parameter keeps the views (blocks `yaml` events and `load`, #928)

This is a compiler fix inside the `yaml` lane: the next `yaml` steps (events and `load` over the parsed tree, ADR-yaml-1 D6, D7) build structures from views of a `ref` tree. It changes `crates/`, so the gate of `.claude/agents/builder.md` applies in full (a change to `tools/check_*.nika` or `tools/emit_*.nika` also runs `target/fast/nikaia test --both-settings --project tests/compiler`), the change raises the toolchain version (the supervisor's: write the `Changelog:` line, touch no version), and `target/fast/nikaia test --project packages/yaml` must stay green.

## Issue #1000 (copied)

## Found while

Setting up the `yaml` package (#928): a pass that walks a parsed tree and builds a second structure (events, a resolved tree) from views of the first. ADR-282 D12/D13: a struct's field and an enum's payload are positions; where only views flow in, the position is a view (`ref String` below). With `target/fast/nikaia` on `develop` (2026-10-30, 833321f), two symptoms of one gap:

### Probe 1: a struct field filled from a `ref` parameter's element is refused (NK1106)

```nika
pub struct Node {
    pub raw: String,
    pub start: u32,
}

pub struct Ev {
    pub raw: String,
}

grammar G {
    entry rule doc -> Vec[Node] = "[" first:item rest:more* "]" eof -> {
        let mut all = [first]
        all.extend(rest)
        all
    }
    rule more -> Node = "," n:item -> { n }
    rule item -> Node = a:here s:Plain -> { Node { raw: s, start: a } }
    rule Plain -> String = s:until("," | "]") -> { s }
}

fn walk(ns: ref Vec[Node]) -> Vec[Ev] {
    let mut out: Vec[Ev] = []
    for n in ns {
        out.push(Ev { raw: n.raw })
    }
    return out
}

pub fn events(input: ref Bytes) -> Vec[Ev] throws {
    let ns = G::doc(input)
    return walk(ns)
}

fn bytes_of(text: String) -> Bytes {
    let mut all: Vec[u8] = []
    for b in text.bytes() {
        all.push(b)
    }
    return Bytes(all)
}

fn main() throws {
    let input = bytes_of("[ab,cd,ü]")
    for e in events(input) {
        println(e.raw)
    }
}
```

```
error[NK1106]: `Ev.raw` holds `String`, but you're giving it `ref String`.
  24 |         out.push(Ev { raw: n.raw })
   = note: `Ev` keeps its `raw` after this line, so it needs text of its own.
```

Expected (D13, views only flow into `Ev.raw`): `Ev` lowers with a lifetime, like `Node` does, and the program prints `ab`, `cd`, `ü`.

### Probe 2: the same with an enum payload is accepted, then rustc refuses it

```nika
pub enum Kind {
    SeqStart,
    SeqEnd,
    Scalar,
}

pub struct Event {
    pub kind: Kind,
    pub raw: String,
    pub start: u32,
    pub end: u32,
}

pub enum Value {
    Str(String),
    Seq(Vec[Value]),
}

grammar G {
    entry rule doc -> Vec[Event] = a:here "[" first:item rest:more* b:here "]" eof -> {
        let mut all = [Event { kind: Kind::SeqStart, raw: "", start: a, end: a + 1 }, first]
        all.extend(rest)
        all.push(Event { kind: Kind::SeqEnd, raw: "", start: b, end: b + 1 })
        all
    }
    rule more -> Event = "," n:item -> { n }
    rule item -> Event = a:here s:Plain b:here -> { Event { kind: Kind::Scalar, raw: s, start: a, end: b } }
    rule Plain -> String = s:until("," | "]") -> { s }
}

pub fn events(input: ref Bytes) -> Vec[Event] throws {
    return G::doc(input)
}

fn build(evs: ref Vec[Event], mut at: i64) -> Value {
    let here = at
    at += 1
    return match evs[here].kind {
        Kind::Scalar => Value::Str(evs[here].raw),
        else => {
            let mut items: Vec[Value] = []
            while true {
                match evs[at].kind {
                    Kind::SeqEnd => break,
                    else => items.push(build(evs, at)),
                }
            }
            Value::Seq(items)
        },
    }
}

pub fn load(input: ref Bytes) -> Value throws {
    let evs = events(input)
    let mut at = 0
    return build(evs, at)
}

fn bytes_of(text: String) -> Bytes {
    let mut all: Vec[u8] = []
    for b in text.bytes() {
        all.push(b)
    }
    return Bytes(all)
}

fn main() throws {
    let input = bytes_of("[ab,cd,ü]")
    match load(input) {
        Value::Seq(xs) => {
            for x in xs {
                match x {
                    Value::Str(s) => println(s),
                    else => println("?"),
                }
            }
        },
        else => println("?"),
    }
}
```

`nikaia check` accepts; `nikaia run` stops in rustc with `Mismatched types` at `return match evs[here].kind`. `nikaia lower` shows `pub enum Value { Str(String), .. }` (owned) while `build` writes `Value::Str(...get(&evs, ..).raw)`, a `&str`: neither the view tier nor a refusal. Expected: `Value<'a>` with `Str(&'a str)`, output `ab`, `cd`, `ü`.

## Locations

* `crates/nikaia-std/src/tools/check_core_places.nika:577`, `:582` (NK1106 for a field, `expect_kept_asked`);
* `crates/nikaia-std/src/tools/text_tiers.nika` (the tiers of D13: which positions take views; 2f73c24 extended them to tuples, #929);
* the emit of an enum's payload tier (`emit_*.nika`; probe 2 shows the tier and the emitted value disagree);
* tests beside `tests/compiler/tests/grammar_a_struct_and_an_enum_result_take_views.nika` (the grammar-result case, which works).

## To build

1. A struct field or an enum payload filled, in a plain function, only from views of a `ref` parameter (an element of a `ref Vec[T]` whose `T` holds views, a field of a `ref` struct) is a view position (ADR-282 D13): the type lowers with a lifetime and the result points into the parameter's buffer (ADR-353 D5, D7). Tests `tests/compiler/tests/a_struct_built_from_a_ref_parameters_views_keeps_them.nika` (probe 1) and `an_enum_built_from_a_ref_parameters_views_keeps_them.nika` (probe 2), each with its `.stdout`.
2. Where the tier and the emitted value disagree (probe 2), the checker refuses with a Nikaia diagnostic instead of handing rustc a mismatch (ADR-056): covered by step 1's tests once they pass; if a case remains that is refused on purpose, a `.stderr` test for it.



**Why it is a defect, not a gap to decide:** ADR-282 D12 makes every declared `String` field and payload a position, D13 gives a position that only views flow into the view tier; ADR-353 D5, D7 record which parameter a result points into. A grammar result already takes views this way (`grammar_a_struct_and_an_enum_result_take_views.nika`); a plain function does not. If the fix turns out to need a rule Part I-III does not state (for instance a struct whose fields come from two `ref` parameters), stop with `Result: blocked`, `Found:` the case with a probe, `Needs: owner decision`.

## Done when

Both new tests pass at both settings; the gate is green; `Fixes #1000` in the commit message; `Changelog:` one sentence for the version heading.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only; commit after each step; `git push origin develop` fast-forward, no rebase (rejected: report `Pushed: rejected`, go on committing).
* Before pushing: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, and the gate of `.claude/agents/builder.md` (#852).
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: owner decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Commit and push before you write the report; the report's `Pushed:` line names the pushed sha. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).
* **Package gate** (this lane): `NIKAIA_CACHE_LIMIT=2G target/fast/nikaia test --project packages/yaml` green, and `cargo test --profile fast -p nikaia --test it -- fmt::every_nika_file` (it formats every `.nika` under `packages/`). For a change only under `packages/yaml/` this is the gate the line above asks for; a change under `crates/` runs the gate of `.claude/agents/builder.md` (#852) as well. `target/fast/nikaia` is built by the session hook; if it is missing, `cargo build --profile fast -p nikaia` once.
* A package change raises no toolchain version (CLAUDE.md table; ADR-yaml-1 D1): `Changelog: none`, unless the brief changes `crates/`.
* Files not to touch: anything outside `packages/yaml/` and the brief's own report, unless the brief names it; never `docs/briefs/batch.txt` or a brief of another lane.

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/yaml/y3-1000-views-from-ref.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (green|red), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDveRM5qKZKyCqq6Y24
