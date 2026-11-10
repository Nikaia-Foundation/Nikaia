# The yaml package against serde_yaml and saphyr

The [yaml package](../../packages/yaml) (ADR-yaml-1) is counted here against
the tuned Rust YAML libraries, at three levels:

| comparison | Nikaia | Rust |
|---|---|---|
| the event stream | [`events.nika`](events.nika): `yaml::documents`, `yaml::events` | [`events_libyaml.rs`](src/bin/events_libyaml.rs): `unsafe-libyaml` 0.2, serde_yaml 0.9's parser (libyaml translated to Rust), through its C API; [`events_saphyr.rs`](src/bin/events_saphyr.rs): `saphyr-parser` 0.0.6, the maintained successor of yaml-rust |
| the tree | [`load.nika`](load.nika): `yaml::load` | [`load_serde_yaml.rs`](src/bin/load_serde_yaml.rs): `serde_yaml::Value`, one per document; [`load_saphyr.rs`](src/bin/load_saphyr.rs): `saphyr::Yaml::load_from_str` |
| typed records | [`facade/src/main.rs`](facade/src/main.rs): `#[derive(Deserialize)]` through `nikaia_yaml::from_str`, the package's serde_yaml facade (#1439) | [`typed_serde_yaml.rs`](src/bin/typed_serde_yaml.rs): the same program over `serde_yaml::from_str` |

## The table

Nikaia 0.0.1371 + #1548 (`ff8827529`), rustc 1.97.0, serde_yaml 0.9.34 over
unsafe-libyaml 0.2.11, saphyr and saphyr-parser 0.0.6; an Intel Xeon at
2.10 GHz; large input 10 MB. Millions of instructions and of mispredicted
branches; the last column is the instructions with checks off over those of
the fastest Rust program of the row's input.

| input | program | instructions, checks on | mispredicted, on | instructions, checks off | mispredicted, off | off / fastest Rust |
|---|---|---:|---:|---:|---:|---:|
| small | yaml::events | 1.0 | 0.0 | 1.0 | 0.0 | 1.64 |
| small | unsafe-libyaml | 0.7 | 0.0 | 0.7 | 0.0 | 1.10 |
| small | saphyr-parser | 0.6 | 0.0 | 0.6 | 0.0 | 1.00 |
| large | yaml::events | 7205.1 | 54.4 | 6703.0 | 39.9 | 2.20 |
| large | unsafe-libyaml | 3537.4 | 23.8 | 3562.5 | 22.5 | 1.17 |
| large | saphyr-parser | 3142.4 | 24.5 | 3040.4 | 21.9 | 1.00 |
| deep | yaml::events | 222.4 | 0.7 | 204.7 | 0.5 | 2.53 |
| deep | unsafe-libyaml | 191.1 | 0.3 | 188.6 | 0.3 | 2.33 |
| deep | saphyr-parser | 88.3 | 0.2 | 80.9 | 0.2 | 1.00 |
| scalars | yaml::events | 437.5 | 1.9 | 424.7 | 1.9 | 2.73 |
| scalars | unsafe-libyaml | 363.6 | 0.3 | 378.4 | 0.3 | 2.43 |
| scalars | saphyr-parser | 162.8 | 0.3 | 155.6 | 0.3 | 1.00 |
| suite | yaml::events | 16.8 | 0.2 | 16.0 | 0.2 | 3.04 |
| suite | unsafe-libyaml | 6.6 | 0.1 | 6.5 | 0.1 | 1.25 |
| suite | saphyr-parser | 5.4 | 0.1 | 5.3 | 0.1 | 1.00 |
| small | yaml::load | 1.1 | 0.0 | 1.1 | 0.0 | 1.52 |
| small | serde_yaml::Value | 0.8 | 0.0 | 0.8 | 0.0 | 1.20 |
| small | saphyr::Yaml | 0.7 | 0.0 | 0.7 | 0.0 | 1.00 |
| large | yaml::load | 7668.1 | 57.6 | 7121.7 | 46.7 | 1.76 |
| large | serde_yaml::Value | 5756.9 | 39.3 | 5769.3 | 36.3 | 1.42 |
| large | saphyr::Yaml | 4410.3 | 34.1 | 4054.4 | 30.9 | 1.00 |
| deep | yaml::load | 230.9 | 0.7 | 211.0 | 0.5 | 1.77 |
| deep | serde_yaml::Value | 240.0 | 0.4 | 237.3 | 0.3 | 1.99 |
| deep | saphyr::Yaml | 130.9 | 0.3 | 119.5 | 0.3 | 1.00 |
| scalars | yaml::load | 438.3 | 2.1 | 421.2 | 2.0 | 2.86 |
| scalars | serde_yaml::Value | 356.2 | 0.4 | 370.9 | 0.4 | 2.52 |
| scalars | saphyr::Yaml | 165.5 | 0.6 | 147.1 | 0.4 | 1.00 |
| suite | yaml::load | 16.2 | 0.2 | 15.4 | 0.2 | 2.61 |
| suite | serde_yaml::Value | 8.7 | 0.1 | 8.7 | 0.1 | 1.47 |
| suite | saphyr::Yaml | 6.4 | 0.1 | 5.9 | 0.1 | 1.00 |
| large | nikaia_yaml::from_str | 6769.2 | 49.7 | 6176.8 | 37.4 | 1.50 |
| large | serde_yaml::from_str | 4085.5 | 26.4 | 4107.7 | 24.9 | 1.00 |

| program | tokens |
|---|---:|
| yaml::events | 418 |
| unsafe-libyaml | 1012 |
| saphyr-parser | 616 |
| yaml::load | 388 |
| serde_yaml::Value | 544 |
| saphyr::Yaml | 554 |
| nikaia_yaml::from_str | 265 |
| serde_yaml::from_str | 265 |

The inputs: small 798 bytes, large 10,000,207, deep 1,008,472, scalars
2,002,237; of the suite's 402 cases, 229 for the events and 241 for the trees
(the others one program refuses, or the programs read differently: the
package's conformance is `packages/yaml/tests`, not this table).

## Where yaml is behind

yaml is behind on every row: 2.2 to 2.7 times saphyr-parser's instructions for
the events on large, deep and scalars, 1.8 to 2.9 times saphyr's for a tree,
1.5 times serde_yaml for typed records. #1548 asks for at most 1.5 times
saphyr-parser on the events; its causes, from `callgrind_annotate` of the
inputs at a tenth, checks off:

* **Events copied once per level**: fixed (#1548). `walk` appends to one
  `Vec[Event]` instead of returning one per node; deep went from 5.4 to 2.5
  times.
* **Properties tried twice**: fixed (#1548). `Props` is read once and the
  alternatives after it share the read; large went from 2.5 to 2.2 times.
* **The directives cloned at each rule**: open. The grammar threads
  `ds: Vec[Dir]` through every node rule and a rule value is passed by value,
  so it is cloned per call (4 % of large, though it is empty there); lending
  it waits for a decision on rule values.
* **Allocation and copying**: open (with #1518). `malloc`, `free` and
  `memcpy` are about 15 % of large: the parser builds a `Node` tree with a
  `String` per scalar and the events are made from that tree, and `text()`
  captures over the bytes grammar cost `from_utf8` (3 %); saphyr-parser scans
  straight into events with spans.
* **Per character**: open. Plain scalars go through `parse_PlainChar` and
  `list::chars` a character at a time (7 % of large), where saphyr scans a run
  of bytes; a guarded repetition over a run waits for a fix in the grammar
  macro (a label in a repeated group is not in scope of its guard).

The typed row is the tree row: `nikaia_yaml::from_str` counts 6,177 million
against `yaml::load`'s 7,122 million on the same input, so the gap is the
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
