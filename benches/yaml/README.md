# The yaml package against serde_yaml and saphyr

The [yaml package](../../packages/yaml) (ADR-yaml-1) is counted here against
the tuned Rust YAML libraries, at three levels:

| comparison | Nikaia | Rust |
|---|---|---|
| the event stream | [`events.nika`](events/events.nika): `yaml::documents`, `yaml::events` | [`events_libyaml.rs`](src/bin/events_libyaml.rs): `unsafe-libyaml` 0.2, serde_yaml 0.9's parser (libyaml translated to Rust), through its C API; [`events_saphyr.rs`](src/bin/events_saphyr.rs): `saphyr-parser` 0.0.6, the maintained successor of yaml-rust |
| the tree | [`load.nika`](load/load.nika): `yaml::load` | [`load_serde_yaml.rs`](src/bin/load_serde_yaml.rs): `serde_yaml::Value`, one per document; [`load_saphyr.rs`](src/bin/load_saphyr.rs): `saphyr::Yaml::load_from_str` |
| typed records | [`facade/src/main.rs`](facade/src/main.rs): `#[derive(Deserialize)]` through `nikaia_yaml::from_str`, the package's serde_yaml facade (#1439) | [`typed_serde_yaml.rs`](src/bin/typed_serde_yaml.rs): the same program over `serde_yaml::from_str` |

## The table

Nikaia at `6a21c8f5`, rustc 1.97.0, serde_yaml 0.9.34 over
unsafe-libyaml 0.2.11, saphyr and saphyr-parser 0.0.6; an Intel Xeon at
2.10 GHz; large input 10 MB. Millions of instructions and of mispredicted
branches; the last column is the instructions with checks off over those of
the fastest Rust program of the row's input.

| input | program | instructions, checks on | mispredicted, on | instructions, checks off | mispredicted, off | off / fastest Rust |
|---|---|---:|---:|---:|---:|---:|
| small | yaml::events | 0.9 | 0.0 | 0.9 | 0.0 | 1.39 |
| small | unsafe-libyaml | 0.7 | 0.0 | 0.7 | 0.0 | 1.09 |
| small | saphyr-parser | 0.6 | 0.0 | 0.6 | 0.0 | 1.00 |
| large | yaml::events | 5042.2 | 31.5 | 4495.0 | 28.9 | 1.48 |
| large | unsafe-libyaml | 3537.4 | 23.2 | 3562.5 | 22.5 | 1.17 |
| large | saphyr-parser | 3142.4 | 23.8 | 3040.4 | 20.0 | 1.00 |
| deep | yaml::events | 158.7 | 0.5 | 140.6 | 0.3 | 1.74 |
| deep | unsafe-libyaml | 191.2 | 0.3 | 188.6 | 0.2 | 2.33 |
| deep | saphyr-parser | 88.3 | 0.2 | 81.0 | 0.2 | 1.00 |
| scalars | yaml::events | 221.9 | 1.2 | 210.8 | 1.1 | 1.35 |
| scalars | unsafe-libyaml | 363.6 | 0.3 | 378.4 | 0.3 | 2.43 |
| scalars | saphyr-parser | 162.8 | 0.2 | 155.6 | 0.2 | 1.00 |
| suite | yaml::events | 14.4 | 0.2 | 13.4 | 0.2 | 2.53 |
| suite | unsafe-libyaml | 6.6 | 0.1 | 6.6 | 0.1 | 1.25 |
| suite | saphyr-parser | 5.4 | 0.1 | 5.3 | 0.1 | 1.00 |
| small | yaml::load | 1.0 | 0.0 | 0.9 | 0.0 | 1.30 |
| small | serde_yaml::Value | 0.8 | 0.0 | 0.8 | 0.0 | 1.20 |
| small | saphyr::Yaml | 0.7 | 0.0 | 0.7 | 0.0 | 1.00 |
| large | yaml::load | 5744.1 | 45.5 | 5196.3 | 39.2 | 1.28 |
| large | serde_yaml::Value | 5756.9 | 36.7 | 5769.3 | 36.3 | 1.42 |
| large | saphyr::Yaml | 4410.3 | 35.8 | 4054.4 | 30.1 | 1.00 |
| deep | yaml::load | 178.8 | 0.6 | 159.7 | 0.5 | 1.34 |
| deep | serde_yaml::Value | 240.0 | 0.4 | 237.3 | 0.3 | 1.99 |
| deep | saphyr::Yaml | 130.9 | 0.3 | 119.5 | 0.3 | 1.00 |
| scalars | yaml::load | 228.8 | 1.3 | 215.4 | 1.2 | 1.46 |
| scalars | serde_yaml::Value | 356.2 | 0.4 | 371.0 | 0.4 | 2.52 |
| scalars | saphyr::Yaml | 165.5 | 0.6 | 147.1 | 0.4 | 1.00 |
| suite | yaml::load | 14.4 | 0.2 | 13.4 | 0.1 | 2.27 |
| suite | serde_yaml::Value | 8.7 | 0.1 | 8.7 | 0.1 | 1.47 |
| suite | saphyr::Yaml | 6.4 | 0.1 | 5.9 | 0.1 | 1.00 |
| large | nikaia_yaml::from_str | 5712.9 | 38.8 | 5142.4 | 36.9 | 1.25 |
| large | serde_yaml::from_str | 4085.5 | 26.7 | 4107.7 | 27.7 | 1.00 |

| program | tokens |
|---|---:|
| yaml::events | 550 |
| unsafe-libyaml | 1012 |
| saphyr-parser | 616 |
| yaml::load | 386 |
| serde_yaml::Value | 544 |
| saphyr::Yaml | 554 |
| nikaia_yaml::from_str | 265 |
| serde_yaml::from_str | 265 |

The inputs: small 798 bytes, large 10,000,207, deep 1,008,472, scalars
2,002,237; of the suite's 402 cases, 229 for the events and 241 for the trees
(the others one program refuses, or the programs read differently: the
package's conformance is `packages/yaml/tests`, not this table).

## Where yaml is behind

The events are within 1.5 times saphyr-parser on large (1.48) and scalars
(1.35), not on deep (1.74) and the suite (2.53); the trees are within 1.5
times saphyr's on every input but the suite (1.28 to 1.46, 2.27 on the
suite) and ahead of serde_yaml on large, deep and scalars; typed records
1.25 times serde_yaml. The events program appends each event to its output,
as the Rust programs do; until `6a21c8f5` it built a `String` per event, a
fifth of large. #1548 asks for at most 1.5 times saphyr-parser on the
events; its causes, from `callgrind_annotate` of `events.off`, checks off:

* **Events copied once per level**: fixed (#1548). `walk` appends to one
  `Vec[Event]` instead of returning one per node; deep went from 5.4 to 2.5
  times.
* **Properties tried twice**: fixed (#1548). `Props` is read once and the
  alternatives after it share the read; large went from 2.5 to 2.2 times.
* **The directives cloned at each rule**: fixed (#1560). A rule value is
  lent (ADR-grammar-actions-v1 D7) and lowered as `&Vec<Dir>`, so no rule
  value is cloned per call (the clone was 4 % of large).
* **Input copied per character**: fixed (#1981). The scalar readers read
  `s.bytes` and `.scalars` in place and copy a run once, as a slice.
* **Per character in the grammar**: fixed (#1548). `PlainMore` takes a run
  of plain characters as one guarded repetition (#1561); scalars went from
  282.8 to 278.3 million.
* **Edge tracking of the `@stream` entry**: fixed (#2281, winnow-grammar
  `0612989`). Large went from 7,874 to 5,887 million, deep from 340 to 186.
* **A line's spaces scanned three times between nodes**: fixed (#1548).
  `Blanks` scans them once; deep went from 185.7 to 164.1 million.
* **The text of every flow entry checked as UTF-8**: open, 8 % of deep.
  `FlowItem` takes `text(..)` of each entry only to check that a key stays
  on one line, and over a `Bytes` input the text is checked (ADR-grammar-input-v1
  D2), so a sequence 120 deep checks its inner levels 120 times. `bytes(..)`,
  unchecked, would do, but binds no name inside it (Part II 10, the grammar
  table), and the entry's node is needed; with the text not taken, deep
  counts 129.0 million (1.59 times).
* **The indentation counted twice**: fixed (#2411). Below a line's end,
  `Indentation` reads the blank lines and the next line's spaces in one
  fold and hands the count to `BlockAt`; deep went from 130.0 to 121.2
  million.
* **The engine's alternatives**: the choices of `FlowItem` and `FlowAny` and
  `winnow_grammar::rt::expected` are 15 % of deep.
* **Allocation and the UTF-8 check of slices**: `malloc` and `free` are
  about 11 % of deep (a `String` per scalar in the tree and again in its
  event); the `text()` of a slice is checked as UTF-8 (`from_utf8`, 5 % of
  large with the captures of the rules), which a cut by text positions (ADR-374 D3, D8)
  would not need, and those cuts are not built yet.
* **The streaming entry itself**: `for e in yaml::Yaml::events(input)`
  prints the same events at 40,279 million on deep, against 140.6 through
  `documents` and `events`: the stream's driver parses the rule's prefix
  again in its diagnosing pass (`Stream::attempt`, 99 %). The bench keeps
  `documents` and `events` until that is fixed.

The typed row is the tree row: `nikaia_yaml::from_str` counts 5,142 million
against `yaml::load`'s 5,196 million on the same input, so the gap is the
parser's.

## How it is measured

```sh
nikaia run benches/yaml/bench.nika          # the table above, large input 10 MB
nikaia run benches/yaml/bench.nika -- 1     # quicker: every input a tenth
```

[`bench.nika`](bench.nika) generates the inputs into `target/yaml-bench`,
builds every program, checks the outputs and counts them, and prints the
table. It needs `valgrind`. The compiler is `$NIKAIA`, else the `nikaia` on
the path, else a release build of the repository's own.

* **Counts, not a clock** (`docs/measuring.md`): instructions and mispredicted
  branches (conditional and indirect), counted by callgrind with
  `--branch-sim=yes` over the whole process, reading the file included.
* **The same output first.** Every program of a comparison, at both settings,
  prints byte for byte what the first one prints before anything is counted.
  The event programs print the yaml-test-suite's event lines with what
  `saphyr-parser` cannot report left out: no `{}` / `[]` after a flow
  collection's start and no `...` after a document's end; an anchor is
  numbered in the order it is defined (`&1`, `=ALI *1`), since `saphyr-parser`
  reports numbers, not names. The tree programs print a canonical dump, one
  line per node: `{` `}` around a mapping (keys and values in turn), `[` `]`
  around a sequence, `~`, `true` / `false`, `#` and an integer, `%` and a
  float, `"` and a string; aliases resolved. In both, `\`, a line break, a tab
  and a carriage return are escaped.
* **Release builds on both sides, the same settings**: `opt-level = 3`,
  **link-time optimisation on** (`lto = true`), one codegen unit, no
  incremental build ([`Cargo.toml`](Cargo.toml), [`facade/Cargo.toml`](facade/Cargo.toml),
  and the `[build.x86_64-linux]` table `bench.nika` writes for the Nikaia
  projects, which build the package with the program). The Nikaia programs
  are built with the default `optimization` keys.
* **Overflow checks on and off, on both sides**: each program is built twice,
  with `RUSTFLAGS="-C overflow-checks=on"` and `=off`. Nikaia keeps them by
  default; Cargo's release profile has them off.
* **Each program a binary of its own**; the Rust programs are one crate with a
  workspace of its own, so its dependencies stay out of the repository's
  `Cargo.lock`.
* **Length in tokens** by [`scripts/tokens.nika`](../../scripts/tokens.nika),
  each file whole. The typed programs are the same Rust file over two crates.

## The inputs

Generated by `bench.nika`, none checked in:

| input | what it is |
|---|---|
| small | a service configuration of about 1 KB: nested block mappings, a flow sequence, an anchor and its alias, quoted strings, a literal and a folded block |
| large | records (`- id: …`) of block and flow mappings and sequences, a folded note each, every 50th an anchor the next aliases; 10 MB. The typed comparison reads it into `Vec<Record>` |
| deep | items of block mappings 100 deep and flow sequences 120 deep, under the parsers' depth limit of 128; a tenth of `large` |
| scalars | long scalars of every style: literal and folded blocks, double-quoted with escapes, single-quoted, plain over several lines; a fifth of `large` |
| suite | the vendored yaml-test-suite (`packages/yaml/tests/suite`), every case all programs of a comparison read the same way and none refuses, read by one process |
