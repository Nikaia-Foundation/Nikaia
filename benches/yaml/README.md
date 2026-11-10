# The yaml package against serde_yaml and saphyr

The [yaml package](../../packages/yaml) (ADR-yaml-1) is counted here against
the tuned Rust YAML libraries, at three levels:

| comparison | Nikaia | Rust |
|---|---|---|
| the event stream | [`events.nika`](events.nika): `yaml::documents`, `yaml::events` | [`events_libyaml.rs`](src/bin/events_libyaml.rs): `unsafe-libyaml` 0.2, serde_yaml 0.9's parser (libyaml translated to Rust), through its C API; [`events_saphyr.rs`](src/bin/events_saphyr.rs): `saphyr-parser` 0.0.6, the maintained successor of yaml-rust |
| the tree | [`load.nika`](load.nika): `yaml::load` | [`load_serde_yaml.rs`](src/bin/load_serde_yaml.rs): `serde_yaml::Value`, one per document; [`load_saphyr.rs`](src/bin/load_saphyr.rs): `saphyr::Yaml::load_from_str` |
| typed records | [`facade/src/main.rs`](facade/src/main.rs): `#[derive(Deserialize)]` through `nikaia_yaml::from_str`, the package's serde_yaml facade (#1439) | [`typed_serde_yaml.rs`](src/bin/typed_serde_yaml.rs): the same program over `serde_yaml::from_str` |

## The table

Nikaia 0.0.1371 (`703d8afea`), rustc 1.97.0, serde_yaml 0.9.34 over
unsafe-libyaml 0.2.11, saphyr and saphyr-parser 0.0.6; an Intel Xeon at
2.10 GHz; large input 10 MB. Millions of instructions and of mispredicted
branches; the last column is the instructions with checks off over those of
the fastest Rust program of the row's input.

| input | program | instructions, checks on | mispredicted, on | instructions, checks off | mispredicted, off | off / fastest Rust |
|---|---|---:|---:|---:|---:|---:|
| small | yaml::events | 1.2 | 0.0 | 1.1 | 0.0 | 1.82 |
| small | unsafe-libyaml | 0.7 | 0.0 | 0.7 | 0.0 | 1.10 |
| small | saphyr-parser | 0.6 | 0.0 | 0.6 | 0.0 | 1.00 |
| large | yaml::events | 9382.1 | 60.2 | 8564.0 | 53.2 | 2.82 |
| large | unsafe-libyaml | 3537.4 | 23.8 | 3562.5 | 22.5 | 1.17 |
| large | saphyr-parser | 3142.4 | 24.5 | 3040.4 | 21.9 | 1.00 |
| deep | yaml::events | 525.4 | 0.9 | 437.6 | 0.8 | 5.41 |
| deep | unsafe-libyaml | 191.1 | 0.3 | 188.6 | 0.3 | 2.33 |
| deep | saphyr-parser | 88.3 | 0.2 | 80.9 | 0.2 | 1.00 |
| scalars | yaml::events | 446.5 | 1.9 | 432.4 | 2.0 | 2.78 |
| scalars | unsafe-libyaml | 363.6 | 0.3 | 378.4 | 0.3 | 2.43 |
| scalars | saphyr-parser | 162.8 | 0.3 | 155.6 | 0.3 | 1.00 |
| suite | yaml::events | 19.0 | 0.2 | 17.8 | 0.2 | 3.37 |
| suite | unsafe-libyaml | 6.6 | 0.1 | 6.6 | 0.1 | 1.24 |
| suite | saphyr-parser | 5.4 | 0.1 | 5.3 | 0.1 | 1.00 |
| small | yaml::load | 1.2 | 0.0 | 1.1 | 0.0 | 1.60 |
| small | serde_yaml::Value | 0.8 | 0.0 | 0.8 | 0.0 | 1.20 |
| small | saphyr::Yaml | 0.7 | 0.0 | 0.7 | 0.0 | 1.00 |
| large | yaml::load | 8723.8 | 55.3 | 7994.5 | 48.3 | 1.97 |
| large | serde_yaml::Value | 5756.9 | 39.3 | 5769.3 | 36.3 | 1.42 |
| large | saphyr::Yaml | 4410.3 | 34.1 | 4054.4 | 30.9 | 1.00 |
| deep | yaml::load | 252.3 | 0.6 | 228.8 | 0.6 | 1.92 |
| deep | serde_yaml::Value | 240.0 | 0.4 | 237.3 | 0.3 | 1.99 |
| deep | saphyr::Yaml | 130.9 | 0.3 | 119.5 | 0.3 | 1.00 |
| scalars | yaml::load | 446.0 | 2.1 | 427.9 | 2.0 | 2.91 |
| scalars | serde_yaml::Value | 356.2 | 0.4 | 370.9 | 0.4 | 2.52 |
| scalars | saphyr::Yaml | 165.5 | 0.6 | 147.1 | 0.4 | 1.00 |
| suite | yaml::load | 17.6 | 0.2 | 16.5 | 0.2 | 2.79 |
| suite | serde_yaml::Value | 8.7 | 0.1 | 8.7 | 0.1 | 1.47 |
| suite | saphyr::Yaml | 6.4 | 0.1 | 5.9 | 0.1 | 1.00 |
| large | nikaia_yaml::from_str | 8909.8 | 48.2 | 8035.5 | 50.3 | 1.96 |
| large | serde_yaml::from_str | 4085.5 | 26.4 | 4107.7 | 24.9 | 1.00 |

| program | tokens |
|---|---:|
| yaml::events | 418 |
| unsafe-libyaml | 1011 |
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

yaml is behind on every row: about 2.8 times saphyr-parser's instructions for
the events, 2 to 2.9 times saphyr's for a tree, 1.96 times serde_yaml for typed
records, and up to twice the mispredicted branches. The overflow checks cost it
3 to 20 % (saphyr 3 to 11 %; libyaml, C translated with wrapping arithmetic,
nothing). From `callgrind_annotate` of
the 1 MB inputs, checks off:

* **Allocation and copying**: `malloc`, `free` and `memcpy` are 23 % of
  `yaml::events` on large and 19 % of `yaml::load`. The parser builds a `Node`
  tree with a `String` per scalar, its raw text and its properties, and the
  events are made from that tree; saphyr-parser and libyaml scan straight into
  events.
* **Events copied once per level** (the deep row, 5.4 times): `walk` returns
  a `Vec[Event]` per node and the parent `extend`s it, so an event is copied
  once for every collection around it. `Vec[Event]` growing and
  `List[Event]::extend` are 42 % of `yaml::events` on deep.
* **The directives cloned at each rule**: the grammar threads
  `ds: Vec[Dir]` through every node rule; cloning it is 5 % of large, 3 % of
  deep, though it is empty there.
* **Properties tried twice**: `Props` tries `Tag(ds)` in two alternatives
  before `Anchor`, on every node; `parse_Tag` and `parse_Props` are 10 % of
  large, which has a property on one record in fifty.
* **Per character**: plain scalars go through `parse_PlainChar` and
  `list::chars` a character at a time (9 % of large), where saphyr scans a run
  of bytes.

The typed row is the tree row: `nikaia_yaml::from_str` counts 8,036 million
against `yaml::load`'s 7,995 million on the same input, so deserialising from
the tree adds half a percent and the gap is the parser's. Each item above is
filed against the package.

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
