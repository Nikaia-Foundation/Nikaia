---
agent: builder
model: opus
max_turns: 100
budget_usd: 20
---
# Brief p13: #1053, `std` states how many items a sequence yields (`ensures_count`, ADR-385 D2)

Facts: Where things live, Traps already met, Commands

Score: S=3 (claim (b): constant-position indexes of `tools/*.nika` such as `parts[0]` after a `split` left checked; an outsider's first example of "the prover does not know list lengths"), O=3 (every program that splits text or collects its characters), C=4 M (a ledger key through parse, text and walk, three entries, tests). Area: prover. Expected: of the 708 catch-all sites, about 27 index a list collected from `split`, 26 one from `chars()` (architect's sample, 2026-10-31; how many fall is what step 4 measures).

## Decision (architect, 2026-10-31: ADR-385 D2, obvious; read `docs/specification/adr/adr-385.md`)

A ledger entry whose result is a `Seq` may carry `ensures_count = [...]`, facts over `count`, `self` (`self.len()`) and the entry's parameters. The walk reads them where the count becomes a length: the list a `collect()` written right after the producer binds (`let xs = s.split(".").collect()`: `count` is `xs.len()`), the value of `.count()` right after the producer, and the turns of a `for` over the producer. Only where no write to `self` stands between, as `ensures_each` (ADR-375 D5). Entries: `str::split`, `str::rsplit`: `["1 <= count", "count <= self.len() + 1"]`; `String::chars`: `["count <= self.len()", "self.len() <= 4 * count"]`; `String::bytes`: `["count == self.len()"]`. Nothing for `splitn`.

## Problem (probed 2026-10-31 on the 21:11 `target/fast/nikaia`, `nikaia check --bounds`)

```nika
fn h(path: String) -> String {
    let parts: Vec[String] = path.split(".").collect()
    return parts[0].to_string()
}

fn k(word: String) -> scalar? {
    if word.len() > 0 {
        let letters: Vec[scalar] = word.chars().collect()
        return letters[0]
    }
    return null
}

fn main() {
    println(f"{h("x.y")} {k("z") ?? 'q'}")
}
```

Both `parts[0]` and `letters[0]` print `checked: no fact bounds what it reads`. After: both `proved`, at both settings. (The probe was run with `ref String` / `Vec[ref String]`, which warns NK1261; write the test without `ref`, and check it still reads a view where the tools do: `check_boundary.nika` :296 `let parts: Vec[ref String] = path.split(".").collect()` then `parts[0]`.)

## Locations (`scripts/where.nika`, 2026-10-31)

* Ledger: `crates/nikaia-std/src/tools/ty.nika` :1127 (`ensures_each` field of the entry, :1165 its default: add `ensures_count` beside it), `ledger.nika` :465 (the key's parse), `ledger_text.nika` :262 (`claims_text`, the entry written back), `crates/nikaia-std/std.contracts` :4104 `str::split`, :4112 `str::rsplit`, :2733 `String::bytes`, :2744 `String::chars` (s1082 and c1a of the main lane also edit `std.contracts`, other entries: stage your hunks only).
* Reading: `bounds_matched.nika` :254 `bm_each` (the pattern for a `bm_count`), its caller that fills `context.each`; `check_core_method.nika` :517 (`std_receivers` by call node, `check_core_method_ledger_type`: find whether `split` keys as `str::` or `String::`, read only); `bounds_walk.nika` :1639 `bounding_each` (how a ledger condition is read over the call's arguments), :3140 `bounding_let`, :3589 `bounding_built` (where a `ListLit` gives its length: a `collect()` over a producer with `ensures_count` gives its facts with `count` := `length_of(name)`), and the D13 turn count of a `for` (`bounding_turn_bounds` :3110).
* The prover walk's twin for claims: `prover_state.nika` :968 `length_given` returns one term (an equality) and cannot carry `>=`; leave it, and say so in `Body:`.
* Re-lower each changed `.nika` (`lower-std`), commit its `lowered/*.rs`; never stage `lowered/project_members.rs`. Do not edit: `grammar_run.nika`, `ledger_items.nika`, `text_tiers.nika`, `check_function.nika`, `check_overflow.nika`, `check_texts.nika`, `check_core_leaves.nika`, `check_core_stmt.nika`, `crates/nikaia/src/contracts/mod.rs` (other lanes' current briefs).

## Steps

1. **The key.** `ensures_count` parsed, kept, written back by `ledger_text`; the three entries in `std.contracts`. Test: `nikaia ledger` (or the command that prints `std`'s entries; builder-facts *Commands*) shows `ensures_count` on `str::split`; if no command prints it, the walk tests of step 2 guard it.
2. **The walk reads it** at `collect()` and `.count()` right after the producer, and **3. a `for` over the producer** counts its turns by the same facts. Test `tests/compiler/tests/sequence_count_facts.nika` + `.stdout` + `.bounds` + `.loops.bounds`, this program (probed today: it prints `x z 99 a 2`; all five sites below are `checked` today):

```nika
fn h(path: String) -> String {
    let parts: Vec[String] = path.split(".").collect()
    return parts[0].to_string()        // after: proved (split, 1 <= count)
}

fn k(word: String) -> scalar? {
    if word.len() > 0 {
        let letters: Vec[scalar] = word.chars().collect()
        return letters[0]              // after: proved (len <= 4 * count)
    }
    return null
}

fn b(s: String) -> u8 {
    let bs: Vec[u8] = s.bytes().collect()
    if s.len() > 2 {
        return bs[2]                   // after: proved (count == len)
    }
    return 0
}

fn w(s: String) -> String {
    let ws: Vec[String] = s.split_whitespace().collect()
    return ws[0].to_string()           // stays checked: no entry
}

fn t(s: String) -> i64 {
    let mut k = 0
    for _ in s.chars() {
        k += 1                         // after: proved (turns <= s.len())
    }
    return k
}

fn main() {
    println(f"{h("x.y")} {k("z") ?? 'q'} {b("abc")} {w("a b")} {t("hé")}")
}
```

Add one `.count()` row of your own (`s.bytes().count()` read as `s.len()`). Show under `Tests:` that the `.bounds` differs on the unfixed binary (`/home/user/nikaia-suite/target/fast/nikaia` when no suite runs there).
4. **Measure and record.** `lower-std --bounds --asserts`, both settings, `scripts/proved.nika`: both `proved:` lines before/after into `Body:`, and `docs/formal-verification.md` §2's table replaced with today's (this commit's short sha). Part III: in the `optimization` paragraph of `docs/specification/30-nikaia-tooling.md`, after the `ensures_each` sentence ("... `f <= s < b.len()` among them ([ADR-375](adr/adr-375.md) D5)."), add: "A list `collect()` builds right after a sequence whose ledger entry has `ensures_count`, that sequence's `count()`, and the turns of a `for` over it, have a length of which each holds: `split` yields at least one piece, `chars()` at most the text's length in bytes and at least a quarter of it, `bytes()` exactly it ([ADR-385](adr/adr-385.md) D2)."
5. **The panic count.** `scripts/proved.nika` counts `panic(` inside a string literal as a panic: `modules.nika` :496 writes `panic(f"...")` into generated text, so the figure says 3 where the tools hold 2 (`check_walked.nika` :121, `emit_grammar.nika` :225). Skip a `panic(` that stands inside a `"..."` on its line; the figure then reads `panics: 2 in 2 files`.
6. Keep passing: every `tests/compiler` program with a `.bounds` / `.loops.bounds` file (re-bless one that rightly gains a proof, name it in `Body:`; one that loses a proof: stop, `partial`), `cargo test -p nikaia --lib -- bounds contracts ledger`, `cargo test -p nikaia --test it -- bounds ledger`.

Changelog: one sentence, not `none`.

## Done when

Steps 1-6 built and tested; build, fmt, clippy (1.97 and 1.99.1), `pre-push-check.sh`, `lower-std` fixpoint; committed locally with `Part: #1053`. Stop about 15 turns before `max_turns`, commit what is finished and report `partial`.

## Rules (CLAUDE.md; this block is pasted into every brief, also a fix-forward)
* Read `docs/builder-facts.md` first. `develop` in the main checkout; `git status` at start: other agents' uncommitted files are not yours, never touch or stage them.
* `git add <files>` only (`scripts/commit.nika -- REPORT` does it); commit locally after each step and **never push**: the driver runs the complete suite and pushes (`.claude/agents/builder.md`, *Cheap checks only*; owner, 2026-10-31).
* Before committing, cheap checks only: `cargo build --profile fast -p nikaia`, `lower-std` and its fixpoint when a `tools/*.nika` changed, `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/pre-push-check.sh`, the tests the brief names and the ones your change touches. No `gate.nika`.
* No version lines, `CHANGELOG.md`, badges; no new NK code. New tests in Nikaia only; never the full suite. No foreground `sleep`, no polling; scripts in files.
* A form Nikaia, `std` or the toolchain lacks stops the work: no code around it unless an ADR or Part I-III refuses it (name it). Else `Result: blocked`, `Found:` gap + probe, `Needs: architect decision` (CLAUDE.md, *A missing form stops the work*).
* Disk: `NIKAIA_CACHE_LIMIT=2G`; at most two clones with `target/` in the machine; delete your own clone's `target/` and `target/release/nikaia-scratch` at the end; report GB freed.
* Time box: no cause after about 40 turns, or going in circles: stop, `Result: partial` with what you ruled out, the probe, and the suspected file and function (`.claude/agents/builder.md`).
* Write the report, then commit with `target/fast/nikaia run scripts/commit.nika -- REPORT`, which writes `Pushed: local <sha>` into it. A report without a `Pushed:` line, or with a placeholder left in (`TC`, `DISK`), is not done and cuts the batch (k3a1 2026-10-29: `Result: done`, nothing committed).

## Report (at most 15 lines, to exactly `/home/user/Nikaia/docs/briefs/prover/p13-1053-sequence-counts.md.report.md`)
Fields of `docs/architect.md`, *Briefs and reports*: `Result: done|noop|partial|blocked` (the bare word alone on that line: `Result: done`, never a summary after it, which cuts the batch, b869a 2026-10-27; `Owner:` is a question for the owner or `none`, never a role), Files, Commit, Body (optional, one paragraph for the commit message), Fixes, Pushed, Tests, Gate (`green` or `red <check>`: your cheap checks, never `none`, which cuts the batch, j4 2026-10-31), Deviation, Found, Needs, Changelog, Owner, Disk freed, and for a move `Rs2nika:` (the `kept N of M lines` of `rs2nika --score`, then the three costliest hand edits; `none` for a task that moves no Rust, `none (redesign)` for a brief with `Shape: redesign` and no `Convert:` line). `none` where nothing, and then the bare word: `Deviation: none (a note)` cuts the batch (batch.nika compares exactly); a note goes into the commit message. `Found:` is only for a defect outside your change (it cuts the batch); a process event such as another session resetting or rewriting your files is no `Found:`, it goes under `Needs:` (m3a 2026-10-28); a converter draft that came out wrong is no `Found:`, it is one of the costliest edits under `Rs2nika:`, which the converter lane reads (e12-4b 2026-10-27); how you solved your own step goes into the commit message. `Deviation:` is only a step of the brief built otherwise than written or not built (a choice the brief left to you, such as *keep what X needs and say so*, is no deviation: it goes into the commit message; nor is building two steps in one commit when every step is built, e12-3a 2026-10-27, nor a step another commit on develop already built, shown by your test failing with that commit's change reverted, b889 2026-10-27: name the commit in `Body:`, nor a test's expected `.stdout`/`.stderr` taken from what the program really prints where the brief's sketch of it was short or wrong, b934 2026-10-29: say so in `Body:`, nor a lowering that follows the ADR governing it where the brief's expected lowering contradicts that ADR, b933 2026-10-29: name the ADR in `Body:`, nor a lowering other than the brief's where rustc cannot take the brief's, b1029 2026-10-31: say why in `Body:`, nor re-keying an allow-list entry (`KNOWN`) whose hash changed for the same program and the same reason, b-fixforward-sweep2 2026-10-28: say so in `Body:`, nor an existing test's expected `.stdout`/`.stderr` updated because your change rightly changes the message it prints, b1012b 2026-10-31: name the test in `Body:`); how you verified (e.g. a test not run against the unfixed compiler) goes under `Tests:`. A `noop` says why under `Tests:` (what was probed), not under `Deviation:`. Final answer, one line: `done <sha>, report <path>` (or `BLOCKED: ...` / `PUSH REJECTED ...`).

Attribution:
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01UVxJDtwVhc6jMtEYAyvwLU
