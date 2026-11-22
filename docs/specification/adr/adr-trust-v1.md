# ADR-trust-v1: Trust Is a Property of the Program, Written in Its Manifest's `[trust]`, Not at a Call

**Status:** Accepted
**Date:** November 22, 2026
**Answers:** where a program says that it chose its standard input, its command
line or its files, so a claim about them is checked rather than refused and
their maps take the fast hash (the owner's decision of November 22, 2026)
**Supersedes / Related:**
[ADR-365](adr-365.md) D2: `trusted: true` at the source; replaced by D1-D4 here.
[ADR-399](adr-399.md) D3: `cli::args(trusted: true)`; replaced the same way.
[ADR-365](adr-365.md) D1, [ADR-399](adr-399.md) D1: stdin, files and the command line are untrusted; they stand.
[ADR-397](adr-397.md) D4: `[build] environment = "untrusted"`, the first property of who chose the input written in the manifest.
[ADR-raw-sinks-1](adr-raw-sinks-1.md) D11, D14, D16: a `Dir` root keeps a name inside it, `path::is_below` proves a path, and no source gets a new `trusted:` option.
[ADR-010](adr-010.md) D3, D7: every trust decision is recorded, and `--trust` reports it.
[ADR-108](adr-108.md): every file operation takes a root.

## 1. Question

```nika
let path = cli::args(trusted: true).nth(1) ?? "measurements.txt"
let b = fs::map(ref path, fs::Root::Anywhere; trusted: true)
```

A benchmark reads input that its operator generated, and it shows how fast
the program can run once the operator says so. ADR-365 D2 let the program say
so with a marker at each call. Should trust stay a marker at a call, or become
a property of the whole program, written once in the place where the program
states its other properties?

A marker at a call is written by habit, and wherever it appears it reads as
safe. Copied into a program that reads strangers' bytes, it turns an
`NK1202` refusal into a check that only stops the program, and turns a keyed
hash into one an attacker can collide. Nothing at the call shows whether the
author knew who writes the input. Whether the operator chose the input
depends on how the program is deployed, not on any one line of it.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| Perl taint mode | `-T` taints all outside input for the whole process. A value is untainted by a regex capture at the place it is used. | Untainting by habit (`/(.*)/`) defeats the check, and no report shows where it was done. | perlsec, from memory, not checked |
| Deno | Permissions are granted to the whole program, at start, by flag or in the config: `--allow-read=./data` names directories. Code cannot grant itself more. | The flag lives in the launch command, so each place that starts the program can widen it differently. | Deno manual, "Permissions", from memory, not checked |
| Java SecurityManager | A policy file outside the code grants `FilePermission` per directory to a code base. | The policy is far from the code and was rarely kept up to date. Deprecated by JEP 411. | JEP 411, from memory, not checked |
| Android | The manifest declares what the app may reach. Code checks it but cannot add to it. | Coarse, per permission, but reviewed in one place and visible at install time. | Android developer docs, from memory, not checked |

What transfers: trust is granted to the program from outside its code. It is
listed in one reviewed place, and a directory can be named instead of the
whole disk. What does not transfer: Deno's and Java's permission checks run
when the program runs. Nikaia's provenance is decided while the program is
built, so the manifest is read by the compiler, not by the program.

## 3. Options, weighed

### Option A: keep `trusted: true` at the call (ADR-365 D2)

```nika
let data = io::read_to_string(trusted: true)
```

### Option B: `[trust]` in the program's manifest, all or nothing per source

```toml
[trust]
stdin = true
files = true
```

### Option C: as B, and `files` may list directories

```toml
[trust]
args  = true
files = ["data"]
```

### Option D: a build flag, `nikaia build --trust-input=stdin`

### Weighed

| | D1 the special case pays | D2 the compiler works | D3 no cost for the common case | D4 consistent | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | The common program pays: the marker spreads by habit and reads as safe. | The developer judges each call. | none | A marker at the source, as `@untrusted` is. | A copied marker silently trusts a stranger's bytes, and no one notices. |
| **B** | Only the program that trusts writes a line. | The compiler applies one line everywhere. | none | It matches `environment` (ADR-397 D4) and `[allow]`. | A program that reads its own data and an upload dir must trust both. |
| **C** | As B, and a program trusts only the directory it owns. | The compiler proves the root lies under a listed directory. | none | It matches `untrusted-paths-read = ["args"]` (ADR-raw-sinks-1 D15), a key that lists what it admits. | Undone by deleting a line, and the ledger shows the change. |
| **D** | Each person who starts the build decides. | | none | It matches `--environment` and `--allow`. | Each build command can differ, and the program's source does not say which one holds. |

## 4. Decision

C. The owner chose it: trust is a property of the program, not of a line of
code, and a program that reads its own files next to other files trusts only
its own. A lost because a marker in code is dangerous where it is written by
habit (ADR-raw-sinks-1 D16). B lost to C for the case it cannot express. D lost
because the program's manifest, not the person running the build, decides
what the program is.

### D1: `[trust]` in the program's manifest names the sources the operator chose.

```toml
[trust]
stdin = true          # every read of standard input
args  = true          # cli::args() and main's parameters
files = ["data"]      # or true
```

A source named there is **trusted**. A claim about its bytes is checked as any
claim is (no `NK1202`), and a map keyed by them may take the fast hash. Under
ADR-raw-sinks-1 D2 its bytes are not data from outside. A source not named
stays untrusted (ADR-365 D1, ADR-399 D1). An absent `[trust]` trusts nothing.
Each key takes `true`. Any other key, or any other value (including `false`),
is a manifest error.

### D2: `files` takes `true` or a list of directories.

`files = true` trusts every file read: `fs::read`, `fs::read_to_string`,
`fs::map`, and the names that `fs::walk` and `fs::read_dir` return.

`files = ["data", "/srv/fixtures"]` trusts only a read under a root
`fs::Root::Dir(d)` where the compiler proves that `d` lies at or below a listed
directory, comparing path components (`path::is_below`, ADR-raw-sinks-1 D14).
A name read under that root stays inside it (D11), so the file is under the
listed directory.

* A relative entry is taken relative to the working directory where the
  program runs, as a relative root is.
* An entry with a `..` component is a manifest error.
* A read under `fs::Root::Anywhere`, or under a root that is not proved to lie
  below a listed directory, stays untrusted.

### D3: Only the program's own manifest sets `[trust]`.

`[trust]` is read from the program being built. A dependency's `[trust]` is
ignored, as its `environment` and `raw-sinks` are: a library does not know
who writes its caller's input.

A file run by `nikaia run f.nika` with no `nikaia.toml` above it (ADR-260 D4)
has no `[trust]`, and every source it reads is untrusted. A program that
needs trust gets a manifest beside it.

The table is a cache-key dimension. No build flag raises trust: `--trust`
remains the report.

### D4: No call takes a `trusted:` option.

The `trusted:` option is removed from `io::read`, `io::read_to_string`,
`io::lines`, `cli::args` and every `fs` read. A call that writes it is
refused as any unknown option is (`NK1109`). The help names the `[trust]`
line that says the same for the whole program, for example `stdin = true`.
`trusted: false` goes with it, since every source it could narrow is already
untrusted. `@untrusted grammar` (ADR-010 D4) stays. It narrows trust and
cannot widen it.

### D5: The ledger and `--trust` name the manifest.

The ledger's `trusted` row lists, per function, each source call that
`[trust]` makes trusted, with the key that does it:
`trusted = ["fs::map at main.nika:7 (files: data)", "cli::args at main.nika:5 (args)"]`.
The program's entry records the `[trust]` table itself, so a diff of the
ledger shows a new key as a review item (ADR-010 D3). `--trust` prints the
table with its manifest line, then each call it covers.

## 5. Consequences

* **Part III 17.1 and 13.3.** The trusted sources are now those the
  manifest's `[trust]` names. The `trusted: true` example becomes a `[trust]`
  table. The `nikaia.toml` reference gains `[trust]`. The ledger's `trusted`
  row (30-nikaia-tooling) changes as D5 says.
* **Programs.** A program that reads input its operator chose says so once.
  The benchmarks (`benches/brc`, `benches/game/k-nucleotide`, the examples
  that read their own data) get a manifest with `[trust]` and lose their
  markers. Our own tools prove their reads or list their directories,
  following ADR-raw-sinks-1 D16. The compiler's hint that writes
  `cli::args(trusted: true)` writes the `[trust]` line instead.
* **Cost accepted.** A program that trusts one input it reads through a call
  and not another, read the same way, cannot say so: it trusts the source or
  proves the value with a guard. That is the intent. Trust that differs within
  one program is a guard, not a trust setting.
* The build is carried by the issue that builds this record.

*Decided November 22, 2026.*
