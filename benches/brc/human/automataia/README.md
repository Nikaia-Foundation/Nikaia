# automataIA/1brc-rs, two binaries

Taken from <https://github.com/automataIA/1brc-rs> at commit
`ab88b1bc2d14110a20191bc19803c5d5916382b8`, under its MIT licence
([`LICENSE`](LICENSE), unchanged).

What was taken, unchanged: `src/lib.rs`, `src/hash.rs`,
`src/bin/v20_safe.rs`, `src/bin/v16_flat_table.rs`, `.cargo/config.toml` and
`LICENSE`. `Cargo.toml` is the authors' with the dependencies, binaries,
benches and dev-dependencies these two do not use left out and `[workspace]`
added; `Cargo.lock` is the authors', cut down to what remains by Cargo. The
authors' 18 other binaries, their generator, benches and scripts were not
taken.

* `v20_safe` runs as it is.
* `v16_flat_table` breaks the 1BRC rules as it is: it reads 8 bytes past the
  end of an unpadded file, panics on a name longer than 32 bytes and loops
  forever on more than 4 096 stations. It runs only **derived and corrected**,
  with [`v16-derived.patch`](v16-derived.patch) applied to a copy
  (`../humans.sh` does that). The patch's header gives the reason for every
  hunk; it adds no speed. The patch is under the same MIT licence, with the
  original notice kept.

[`../humans.sh`](../humans.sh) builds both at the bench's settings and at the
authors' and counts them; [`../build.sh`](../build.sh) says how the authors'
settings are overridden.
