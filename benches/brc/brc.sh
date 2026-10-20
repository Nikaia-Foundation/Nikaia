#!/usr/bin/env bash
# What `examples/1brc.nika` and `naive.nika` cost against the same aggregation
# written in Rust: instructions and mispredicted branches per row, counted with
# callgrind, and each program's length in tokens. It prints the table
# `README.md` keeps.
#
# Counts, not a clock: on a shared machine the same binaries swap places from
# one round to the next, and a count does not.
#
#   benches/brc/brc.sh            1 000 000 rows
#   benches/brc/brc.sh 200000     quicker, the same table at a smaller size
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
rows=${1:-1000000}
work=${TMPDIR:-/tmp}/brc-bench
input="$work/measurements-$rows.txt"
mkdir -p "$work"

command -v valgrind >/dev/null || { echo "brc.sh needs valgrind (callgrind)"; exit 1; }

# The compiler, a release build of its own in `target/release`.
cargo build --manifest-path "$root/Cargo.toml" --release --quiet -p nikaia
nikaia="$root/target/release/nikaia"

# Which codegen unit rustc puts a function in decides whether LLVM inlines it
# into the hot loop (`hashbrown`'s `entry`; on Nikaia's side also the grammar
# runtime's `advance`), and that moves a row by about 40 instructions either
# way (README, #695). So every program is built and counted at two settings,
# the same on both sides each time: one unit, and release's default of 16.
units="1 16"

# The Rust programs: `cargo build --release`, the unit count set per run, each
# setting in a target directory of its own so the compiler's build in
# `target/release` is left as it is.
rustbin() { echo "$work/target-cu$1/release"; }
for u in $units; do
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=$u CARGO_TARGET_DIR="$work/target-cu$u" \
        cargo build --manifest-path "$root/Cargo.toml" --release --quiet -p brc-bench
done

[ -s "$input" ] || "$(rustbin 16)/gen" "$rows" "$input" 2>/dev/null

# A Nikaia program is built the way a user builds it: a project with a
# `nikaia.toml`. `opt-level = 3` as Cargo's release profile has it, no LTO on
# either side, every optimization the compiler offers on. The unit count is
# the project's `codegen-units` (ADR-002 D6).
proj="$work/project"
mkdir -p "$proj/src"
nikaia_toml() {
cat > "$proj/nikaia.toml" <<TOML
[package]
name = "brc"
version = "0.1.0"

[build]
user-parallelism = "$1"
optimization = "remove-bounds-checks:on,remove-overflow-checks:on,skip-free-at-exit:on"

[build.x86_64-linux]
opt-level = 3
codegen-units = $2
incremental = false
TOML
}

# Builds `$4` (default `examples/1brc.nika`) in the project at `$1` codegen
# units and copies the binary out of it, where the next build would overwrite
# it.
build_nikaia() {
    local u=$1 name=$2 parallel=$3 source=${4:-$root/examples/1brc.nika}
    cp "$source" "$proj/src/main.nika"
    nikaia_toml "$parallel" "$u"
    ( cd "$proj" && "$nikaia" build >/dev/null 2>&1 ) \
        || { echo "nikaia build failed ($name, $u units)"; exit 1; }
    cp "$proj/target/nikaia/bin/brc" "$work/nikaia.$name.cu$u"
}

for u in $units; do
    build_nikaia "$u" parallel yes
    build_nikaia "$u" checked no
    # The same program without the overflow checks the language keeps, to say
    # what they cost. `RUSTFLAGS` comes after the profile's flags, so it wins.
    RUSTFLAGS="-C overflow-checks=off" build_nikaia "$u" unchecked no
    build_nikaia "$u" naive no "$root/benches/brc/naive.nika"
done

# --- the same answer, first, at each setting ------------------------------

"$(rustbin 16)/naive" "$input" > "$work/out.naive"
for u in $units; do
    for p in "$(rustbin "$u")/naive" "$(rustbin "$u")/tuned" "$work/nikaia.checked.cu$u" \
        "$work/nikaia.unchecked.cu$u" "$work/nikaia.parallel.cu$u" "$work/nikaia.naive.cu$u"; do
        "$p" "$input" | cmp -s - "$work/out.naive" \
            || { echo "FAIL: $p disagrees with naive"; exit 1; }
    done
done

# --- then the counts -------------------------------------------------------

# Instructions and mispredictions per row of one program, as two cells.
count() {
    valgrind --tool=callgrind --branch-sim=yes --callgrind-out-file="$work/cg.out" "$@" >/dev/null 2>&1
    awk -v r="$rows" '/^summary:/ { printf "%.1f | %.2f", $2 / r, ($4 + $6) / r }' "$work/cg.out"
}
# Lengths by `scripts/tokens.nika`, one run for all four files.
nikaia_tokens="$work/tokens.txt"
( cd "$root" && "$nikaia" run scripts/tokens.nika -- benches/brc/src/bin/naive.rs \
    benches/brc/src/bin/tuned.rs examples/1brc.nika benches/brc/naive.nika ) > "$nikaia_tokens" 2>/dev/null \
    || { echo "scripts/tokens.nika failed"; exit 1; }
tokens() { awk -v f="$1" '$2 == f {print $1}' "$nikaia_tokens"; }

# One row: the label, the tokens, then the program at each setting (`@` stands
# for the unit count in its path).
row() {
    local label=$1 tokens=$2 line
    shift 2
    line="| $label"
    for u in $units; do
        line="$line | $(count "${1//@/$u}" "$input")"
    done
    echo "$line | $tokens |"
}

version=$(sed -n 's/^\*\*Version:\*\* \([0-9.]*\).*/\1/p' "$root/docs/specification/10-nikaia-light.md")
echo "Nikaia $version, $(rustc --version | cut -d' ' -f1-2), $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //')"
echo "callgrind, $rows rows, per row; every program prints the same, byte for byte, at each setting"
echo
echo "| | instructions, 1 unit | mispredictions, 1 unit | instructions, 16 units | mispredictions, 16 units | tokens |"
echo "|---|---:|---:|---:|---:|---:|"
row 'Rust, `naive`' "$(tokens benches/brc/src/bin/naive.rs)" "$work/target-cu@/release/naive"
row 'Nikaia, `naive.nika`' "$(tokens benches/brc/naive.nika)" "$work/nikaia.naive.cu@"
row 'Rust, `tuned`' "$(tokens benches/brc/src/bin/tuned.rs)" "$work/target-cu@/release/tuned"
row '**Nikaia**, `examples/1brc.nika`' "$(tokens examples/1brc.nika)" "$work/nikaia.checked.cu@"
row 'Nikaia, overflow checks off' '' "$work/nikaia.unchecked.cu@"
