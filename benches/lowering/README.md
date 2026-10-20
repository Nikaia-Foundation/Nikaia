# Lowering

What it costs the compiler to lower a package (#727).

`lower.sh` builds `target/release/nikaia`, then

* **(a)** lowers the tools package cold (`nikaia lower-std` with an empty
  `NIKAIA_CACHE_DIR`), prints the phases `NIKAIA_LOWER_TIMES` reports and
  checks that `crates/nikaia-std/src/tools/package.rs` comes out byte for
  byte as committed (`git diff --exit-code`): a change that moves the output
  is not a speed-up. The phases are a clock and only say *where* the time
  goes; they are not quoted as a result.
* **(b)** counts the instructions (callgrind's `Ir`) of a cold
  `nikaia lower --no-cache` of `crates/rs2nika/convert/src/main.nika`
  (about 5 000 lines of real Nikaia). This count is the result.

```
benches/lowering/lower.sh          both
benches/lowering/lower.sh std      only (a)
benches/lowering/lower.sh count    only (b)
```

| Lowered | Instructions (Ir) |
| :--- | ---: |
| `rs2nika/convert/src/main.nika`, cold | 3 812 955 235 |
