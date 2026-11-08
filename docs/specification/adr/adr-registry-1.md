# ADR-registry-1: Nikaia Packages Have a Registry of Their Own — a Git Index Whose Versions Arrive by Pull Request, a Canonical Archive Hashed With SHA-256, and Mirrors Anyone Can Run; crates.io Carries No Nikaia Package

**Status:** Open
**Date:** November 8, 2026
**Answers:** whether a Nikaia package keeps being published to crates.io under
the `nikaia_` prefix with its `.nika` sources ([ADR-286](adr-286.md) D23-D27),
after part of the Rust community asked that crates.io carry Rust only; and, if
not, how a package is found, fetched, checked and mirrored without crates.io,
without paid infrastructure the ecosystem depends on, and so that one hijacked
account cannot poison the ecosystem
**Supersedes / Related:**
[ADR-286](adr-286.md) D23-D27 — a published package is the crate
`nikaia_<name>` on crates.io, resolved and fetched by Cargo, with a stub
manifest; this record replaces them if it is accepted, and §6 of ADR-286 left
the question open ("Reversible: the compiler reads a directory");
[ADR-286](adr-286.md) D15, D19-D21 — a package is depended on by path, is its
own crate, a type's identity is the package's directory, a transitive package
cannot be named; this record keeps them;
[ADR-336](adr-336.md) D2 — the compiler is published on crates.io from 0.1;
this record keeps that for the compiler and takes packages out of it;
[ADR-002](adr-002.md) — Cargo stays the build backend and the source of Rust
crates (`type = "rust"`);
[ADR-100](adr-100.md) D3-D4 — a consumer believes a dependency's ledger while
its source hashes match, and `--locked` regenerates and compares it; this needs
the sources at the consumer;
[ADR-270](adr-270.md) D4, D5, D24 — the solver's search need not be
deterministic, an answer carries a certificate, the logic layer is a package
any package may depend on;
[ADR-324](adr-324.md) — a package has no build script;
[ADR-338](adr-338.md) — the official packages and their names;
[ADR-393](adr-393.md) — a package published as a Rust library

## 1. Question

Does a Nikaia package keep living on crates.io, or does Nikaia get a registry
of its own — and if so, which one?

[ADR-286](adr-286.md) put a published package on crates.io as the crate
`nikaia_<name>`, holding `nikaia.toml`, `src/*.nika`, `nikaia.contracts` and a
stub `Cargo.toml` that exists for Cargo's resolver only. There were no
libraries then, and a registry of Nikaia's own would have been decided when
there were three packages. Three costs of that answer are now visible:

* **The Rust community asks for it to change.** A crate that is not Rust,
  published so that another tool can fetch it, is taken by many as a misuse of
  crates.io. Generated Rust on crates.io is accepted; Nikaia sources are not.
* **Nikaia has no name space.** A squatter can take `nikaia_foo`; D26 makes it
  useless as a package, not free.
* **Resolution belongs to a tool that cannot read the ledger.** Cargo picks
  versions by semver alone; whether a minor release kept its `sync`, its error
  set or its locks is invisible to it.

Without an answer, `nikaia publish`, a dependency by version (#551) and every
library written outside this repository wait, and every library published in
the meantime is published under a scheme that may change.

## 2. How others do it

| Language / tool | Its answer | What it costs there | Source |
| :--- | :--- | :--- | :--- |
| Julia (General registry) | A Git repository; each new version is a pull request carrying metadata only (repository URL, Git tree hash); a bot opens it, CI checks rules and merges automatically; package servers mirror the content | A bot to run; the tree hash is Git's SHA-1 | from memory, not checked |
| winget (Microsoft), Homebrew | Manifests accepted by pull request into one repository, checked by CI | Review load on the maintainers; the repository grows with every version | from memory, not checked |
| Go | The fetch location is the identity; a proxy mirrors, a checksum database is an append-only log; Minimal Version Selection | Years without a registry; MVS does not bring fixes on its own | from memory, not checked |
| Cargo, npm, pip | A registry server with upload; the newest compatible version at resolution, frozen in a lockfile; Cargo's index is one file per package, read file by file over HTTP | A server and an account system to run; account takeovers poisoned npm and PyPI packages repeatedly | from memory, not checked |
| Dart, uv (PubGrub) | A conflict-driven solver built to explain an unsolvable set in sentences | — | from memory, not checked |
| npm | Several versions of one package side by side, nested per dependent | Duplicated code and state; "two copies of React" | from memory, not checked |
| Rust (RFC 1977) | Public and private dependencies, so a private one may be duplicated | Declared by hand; not stable | from memory, not checked |
| Debian, CPAN, CTAN | Volunteer mirrors, trusted through checksums or signatures from another source | A mirror list to keep healthy | from memory, not checked |
| Nix | A file tree hashed over a canonical archive format (NAR), not over a tarball | — | from memory, not checked |

What does not transfer: a registry server with upload and accounts (crates.io,
npm) — it is paid infrastructure the whole ecosystem would depend on, and the
account system is the part that was attacked. A bot that must run somewhere
(Julia's Registrator) — the same objection on a smaller scale.

## 3. Options, weighed

### Option A — crates.io as decided ([ADR-286](adr-286.md) D23-D27)

```toml
http = "1.2"   # the crate nikaia_http on crates.io, resolved by Cargo
```

### Option B — crates.io for the generated Rust, Git for the sources

crates.io carries the lowered Rust with the ledger and the proofs, no `.nika`;
the registry points at the author's Git for the sources, and crates.io serves
as the mirror.

### Option C — a registry server of Nikaia's own, with upload

### Option D — a Git index fed by pull requests, sources from Git, a canonical archive, mirrors anyone runs

```toml
http = "1.2"   # resolved against the Nikaia index, fetched from a mirror or from Git
```

For resolution under D, two sub-options: **D-min**, Minimal Version Selection;
**D-new**, the newest compatible version.

### Weighed

| | D1 the special case pays, not the common one | D2 the compiler works, not the developer | D3 no cost for the common case | D4 consistent with itself | If it is wrong |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **A** | Every package pays the prefix and the stub | Cargo resolves without the ledger | Nothing to run | A Nikaia package is a crate that is not Rust | Packages published until then move; the community's objection stands meanwhile |
| **B** | A consumer without the sources cannot check the ledger (ADR-100 D3-D4); build options are frozen at publication; generated Rust ties a package to one `std` version; the cross-package analysis stops | The author builds for every consumer | A second artifact per version | Two ways to consume one package | A forged ledger is a data race at the consumer with no message |
| **C** | — | — | A server and an account system to pay for and run | — | The ecosystem stops when the server stops |
| **D** | A package's first registration pays a review; a mirror pays a sync | Ownership, semver and duplication are derived, not declared | No server; GitHub and Codeberg free tiers | One way: the sources, always | Index and mirrors are files and move with a `mirror sync`; the checks are a tool any caller runs |

Between **D-min** and **D-new**: MVS is simple and tests the lower bounds, but
does not bring fixes on its own and is not what a developer expects. Its
argument "no SAT solver needed" does not hold here: Nikaia has one
([ADR-270](adr-270.md) D24). The risks of D-new — a break that calls itself
compatible, a poisoned release flowing in at once, untested lower bounds — are
each answered below (D7-D9). D2 decides for D-new.

## 4. Decision

Option D with D-new.

A lost because it keeps Nikaia sources on crates.io, gives no name space and
leaves resolution to a tool that cannot read the ledger. B lost because a
consumer without the sources cannot check the ledger, which turns a lying
ledger into an unreported data race (ADR-010 D1's polarity); it also freezes
the consumer's build options and couples a package to one `std`. C lost
because it is paid infrastructure and a single point the ecosystem would stop
at.

### D1 — The registry is a Git repository with one file per package.

The index repository holds one file per package, under directories named by
the first letters of its name. For each version the file records: the
version; the Git URL of the source repository; the commit; the content hash
(D4); the direct dependencies with their ranges; the lowest Nikaia version the
package builds with; the licence and a one-line description; whether the
version is yanked or removed (D15); and, per dependency, whether it is public
(D17). The head of the file names the owners and their public signing keys
(D5). A client reads only the files it needs, one by one over HTTP; it never
clones the whole index.

### D2 — A new version arrives by pull request, and the pull request carries metadata only.

Sources never enter the index. `nikaia publish` checks locally that the tree
is clean, the commit pushed, the tag signed, the build and the tests green and
the ledger unchanged under `--locked`, and opens the pull request through a
fork. An entry may be written by hand. No bot or service opens pull requests.

### D3 — The registry's CI builds from the pinned commit, checks, and keeps nothing but the metadata.

The CI checks out the named commit, builds it, runs its tests, computes the
content hash and compares it with the entry; when all holds it merges. A run in
the author's repository is not trusted: neither the runner nor the workflow
that ran can be known from outside.

Every check is a Nikaia tool, `nikaia registry check`, a `tools/*.nika`. The CI
is one caller of it; when GitHub Actions goes away, another CI or a maintainer
by hand runs the same tool.

Foreign code runs only in a workflow without secrets and without write
permission, triggered by `pull_request`, never `pull_request_target`; a
separate workflow merges and reads only the results. A package has no build
script ([ADR-324](adr-324.md)), which narrows what building it can do; its
tests still run foreign code.

### D4 — A package is hashed as a canonical archive, and the archive is the byte stream the hash is taken over.

The format is `nkpkg`, version 1. Paths are relative, sorted bytewise, UTF-8 in
NFC. Every field — path, mode, content — is preceded by its length. The mode
says only whether a file is executable. Symlinks, absolute paths, `..`, empty
directories and anything outside the package are refused. The package is
`nikaia.toml`, `src/`, `nikaia.contracts`, `nikaia.proofs`, the tests and the
licence.

The hash is SHA-256 over the uncompressed archive, written with its algorithm
and format: `sha256:nkpkg1:<hex>`. The length prefixes make it impossible for
two different trees to give one byte stream; length extension does not apply,
since no secret is hashed; the prefix lets the algorithm change later.

The Git commit is recorded as where to fetch from, not as what is checked:
what is checked is the content hash, not Git's SHA-1.

### D5 — Ownership is proved by the commit and kept by a signature with a key that is not the GitHub login.

At a package's **first registration**, `nikaia.toml` in the registered commit
names the owners and their public keys; only someone with write access to the
repository can put them there. The CI checks that the pull request's author is
in that list and that the release tag is signed by one of those keys (an SSH
signature or Sigstore).

From the **second version** on, the key that counts is the one in the index,
not the one in the repository: whoever takes over the GitHub account and edits
the list in the repository does not get through.

**Owners and keys change** by a pull request on the index, signed with a
current key, after a waiting period during which the current owners are told.

A **new package and a change of owner** need a human review. A new version of a
known package is merged automatically when every check is green.

### D6 — Resolution takes the newest compatible version, and the result is defined, not the search.

Resolution is a SAT problem: a package at a version is a variable, a dependency
and an exclusion are clauses. It is solved with the logic package of
[ADR-270](adr-270.md) D24.

Since the solver's search need not be deterministic (ADR-270 D4), the
**result** is fixed: the lexicographic optimum, which for the packages in a
fixed order — direct dependencies first, then by name — takes for each the
highest version that still admits a solution. Two machines write the same
lockfile.

When the set is unsolvable, an explanation layer turns the solver's
certificate (ADR-270 D5) into sentences in the manner of PubGrub: "`a 2.x`
needs `http >= 2`; `b 1.x` needs `http < 2`."

### D7 — The lockfile pins, and a new version must first have waited.

`nikaia.lock` records for every package its version, commit and content hash.
Only `nikaia add` and `nikaia update` resolve; a build never does.

At resolution a version counts only once it has stood in the index for a
cooling period; a switch skips the period for a security fix. `nikaia update`
shows, for every dependency it changes, the difference in its ledger: what
newly throws, what newly may pause, which locks it newly takes.

### D8 — The registry's CI checks semver from the public API and the ledger.

A patch or minor version is refused when it breaks the one before: a public
item removed, a signature changed incompatibly, a `sync` lost, a member added
to `throws`, a `touches` or `locks` entry changed so that a caller that
believed the old one is wrong. A major version may do anything.

The largest risk of taking the newest version — a break that calls itself
compatible — is thereby checked by a machine, not promised.

### D9 — The lower bounds are tested.

`nikaia test --minimal-versions` resolves to the lowest allowed versions and
tests with them. The registry's CI runs it once for every new version.

### D10 — The client fetches from mirrors and else from Git, always checks the hash, and Cargo resolves no Nikaia package.

The client tries the configured mirrors in order (D11), then the author's
repository, from which it fetches only the pinned commit by partial clone and
shallow fetch. What it fetched counts only when the content hash matches; it
then sits in the local cache.

The generated `Cargo.toml` names every Nikaia package by `path` only. Cargo
builds; it never sees where a Nikaia package came from.

The `path` and `git` arms of `nikaia.toml` stay; `git` is fetched by Nikaia,
not by Cargo. A Rust crate (`type = "rust"`) still comes through Cargo from
crates.io: that is Rust's ecosystem, not Nikaia's.

### D11 — A mirror is a format, not a server.

A mirror is a directory of fixed layout, `v1/sha256/<hash>.nkpkg`, optionally
compressed as `.nkpkg.zst`. A file's name is its hash: whoever downloads it
hashes it and compares. A mirror is therefore not trusted — it can serve or
fail to serve, never substitute. Any static file host will do: release assets
on GitHub or Codeberg, a bucket, a university's or a company's web server, a
local directory.

The tools, each a `tools/*.nika`:

* `nikaia mirror sync <target>` builds or updates a mirror, for the whole index
  or, with `--project`, for one project's dependencies. A file never changes,
  so a sync only adds and may be repeated at will.
* `nikaia mirror serve` is a small HTTP server for a local network or a
  company.
* `nikaia vendor` puts the archives of every dependency into the project.

A client reads its mirrors from the user's configuration or from
`NIKAIA_MIRRORS`. A private registry is a second index repository with its own
mirror, served by the same tools.

### D12 — The foundation runs one mirror, the community others, and none is the only one.

After every merge the registry's CI feeds the foundation's mirror with
`nikaia mirror sync`, in two places: the release assets of a mirror repository
on GitHub and on Codeberg. A service with a free tier may be a further mirror,
never the only one.

The index keeps a list of public mirrors; a mirror is added by pull request,
and the registry's CI checks regularly that each is complete and reachable.

For the long term, the registry's CI asks Software Heritage to archive the
source repository at every registration.

### D13 — Search is on the website, static, with Pagefind.

When the website is built, a `tools/*.nika` writes one page per package under
`/packages/<name>/` from the index: description, versions, licence, repository,
the public surface from the ledger, the mirrors. Pagefind indexes the pages,
and search runs in the browser as WebAssembly, with filters such as licence.
There is no search server, and the pages run on any static host. `nikaia
search` on the command line reads the index directly.

### D14 — Names.

A package's name is a lowercase identifier, because it is the name a `use`
writes. `std` and the names of the official packages
([ADR-338](adr-338.md)) are reserved. A new name very close to an existing
one — one typo away, or with confusable characters — is not refused but put
to a human review.

### D15 — Yanking and removing.

A **yanked** version is marked in the index: existing lockfiles keep building,
a new resolution skips it. A version is **removed** only for malicious code or
a legal reason: it is deleted from the foundation's mirrors and listed as
removed, and a lockfile that pins it fails loudly with the reason, not quietly.

### D16 — Who wants to reach Rust users publishes the generated Rust to crates.io themselves.

An author who offers a library to Rust users publishes the lowered code as a
Rust library ([ADR-393](adr-393.md)). That is the author's choice for a Rust
audience; the registry neither requires nor reads it.

### D17 — Two versions of one package may stand side by side where they cannot meet.

1. **One version is preferred.** The resolver first looks for a solution with
   exactly one version of each package; a duplicate is the way out of a
   conflict, not the normal case.
2. **A dependency is public** where its types or traits appear in the
   package's public surface: a `pub` signature, a `pub` field, an `impl` of a
   foreign trait. The compiler derives it and records it in the ledger; no
   author declares it.
3. **The condition:** two versions of one package may be in one graph when no
   point of the graph reaches both through public edges. In the SAT problem a
   public edge is a clause "the same version"; a private one is not. Two
   versions are two directories, hence two crates with distinct types
   ([ADR-286](adr-286.md) D19), and a transitive package cannot be named
   (ADR-286 D21), so where the condition holds nothing can confuse them.
4. **A package with global state** declares `single = true` in `nikaia.toml`
   and is never duplicated.
5. **A duplicate is visible:** `nikaia build` notes it with both paths through
   the graph, and the lockfile lists both versions. Two copies cost binary
   size and build time; nobody is to miss them.
6. **A conflict that stays unsolvable**, because two versions would meet
   through public edges, is reported at the point where they would meet: "`a`
   hands out `http::Request` of 1.2; `b` takes `http::Request` of 1.5."

Two major versions side by side are a case of this rule, not a separate one.

## 5. Consequences

* Nikaia has a name space of its own, and crates.io carries no Nikaia sources;
  the squatting of `nikaia_<name>` stops mattering.
* A package is always built from its sources, with the consumer's options, the
  full analysis and `--locked`: [ADR-100](adr-100.md) holds without
  exception.
* Resolution through Cargo ([ADR-286](adr-286.md) D24) is gone: the generated
  `Cargo.toml` names Nikaia packages by path only. Part III 13.3 is rewritten,
  and `nikaia.lock` gains the commit and the content hash per package.
* To be built, all in Nikaia: the archive format and its hash; the resolver on
  the logic package with its explanation layer and the public-edge rule; the
  fetch from mirrors and Git; the semver check over API and ledger; the
  derivation of public dependencies; `nikaia publish`, `registry check`,
  `mirror sync`, `mirror serve`, `vendor`, `search`; the generator of the
  package pages.
* The running cost is nothing. What it rests on is GitHub for the index and the
  CI and GitHub or Codeberg for the mirrors, each replaceable without changing
  the logic, because every check is a tool and every mirror is files.
* A hijacked GitHub account cannot register a version without the signing key,
  and cannot change one already registered; if it gets through anyway, the
  cooling period and the ledger difference slow it down.
* The compiler itself is still published to crates.io ([ADR-336](adr-336.md)
  D2); its packages are not. #551 changes accordingly.

## 6. What this does not decide

* The length of the cooling period, and who may use the switch that skips it.
* A transparency log or TUF metadata for the index itself; until then the
  client accepts only fast-forwards of the index.
* How a private registry authenticates access.
* Which compression a mirror may offer besides zstd.
* Who keeps the registry: its maintainers, its reviewers, who decides a
  removal.
* Whether the index later moves off GitHub, and where.
* Whether the compiler can recognise global state itself, so that D17's
  `single = true` is derived instead of declared.
* How versions enter the names of the generated crates (`nikaia_http_1_2`):
  the emitter's business, never a name an author writes.
