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

# Release builds on both sides: `cargo build --release` for the Rust programs,
# and the compiler itself.
cargo build --manifest-path "$root/Cargo.toml" --release --quiet -p brc-bench -p nikaia
bin="$root/target/release"
nikaia="$bin/nikaia"

[ -s "$input" ] || "$bin/gen" "$rows" "$input" 2>/dev/null

# A Nikaia program is built the way a user builds it: a project with a
# `nikaia.toml`. `opt-level = 3` as Cargo's release profile has it, no LTO on
# either side, every optimization the compiler offers on. One codegen unit, as
# the Rust programs have (README); a project cannot say it yet (#692).
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
incremental = false
TOML
}

# Builds `$3` (default `examples/1brc.nika`) in the project and copies the
# binary out of it, where the next build would overwrite it.
build_nikaia() {
    local name=$1 parallel=$2 source=${3:-$root/examples/1brc.nika}
    cp "$source" "$proj/src/main.nika"
    nikaia_toml "$parallel"
    ( cd "$proj" && CARGO_PROFILE_DEV_CODEGEN_UNITS=1 "$nikaia" build >/dev/null 2>&1 ) || { echo "nikaia build failed ($name)"; exit 1; }
    local built
    cp "$proj/target/nikaia/bin/brc" "$work/nikaia.$name"
}

build_nikaia parallel yes
build_nikaia checked no
# The same program without the overflow checks the language keeps, to say what
# they cost. `RUSTFLAGS` comes after the profile's flags, so it wins.
RUSTFLAGS="-C overflow-checks=off" build_nikaia unchecked no
build_nikaia checked no >/dev/null
build_nikaia naive no "$root/benches/brc/naive.nika"

# --- the same answer, first ------------------------------------------------

"$bin/naive" "$input" > "$work/out.naive"
for p in "$bin/tuned" "$work/nikaia.checked" "$work/nikaia.unchecked" "$work/nikaia.parallel" "$work/nikaia.naive"; do
    "$p" "$input" | cmp -s - "$work/out.naive" || { echo "FAIL: $(basename "$p") disagrees with naive"; exit 1; }
done

# --- then the counts -------------------------------------------------------

count() {
    local label=$1 tokens=$2
    shift 2
    valgrind --tool=callgrind --branch-sim=yes --callgrind-out-file="$work/cg.out" "$@" >/dev/null 2>&1
    awk -v r="$rows" -v l="$label" -v t="$tokens" '/^summary:/ {
        printf "| %s | %.1f | %.2f | %s |\n", l, $2 / r, ($4 + $6) / r, t }' "$work/cg.out"
}
# Lengths by `scripts/tokens.nika`, one run for all four files.
nikaia_tokens="$work/tokens.txt"
( cd "$root" && "$nikaia" run scripts/tokens.nika -- benches/brc/src/bin/naive.rs \
    benches/brc/src/bin/tuned.rs examples/1brc.nika benches/brc/naive.nika ) > "$nikaia_tokens" 2>/dev/null \
    || { echo "scripts/tokens.nika failed"; exit 1; }
tokens() { awk -v f="$1" '$2 == f {print $1}' "$nikaia_tokens"; }

version=$(sed -n 's/^\*\*Version:\*\* \([0-9.]*\).*/\1/p' "$root/docs/specification/10-nikaia-light.md")
echo "Nikaia $version, $(rustc --version | cut -d' ' -f1-2), $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //')"
echo "callgrind, $rows rows, per row; every program prints the same, byte for byte"
echo
echo "| | instructions | mispredictions | tokens |"
echo "|---|---:|---:|---:|"
count 'Rust, `naive`' "$(tokens benches/brc/src/bin/naive.rs)" "$bin/naive" "$input"
count 'Nikaia, `naive.nika`' "$(tokens benches/brc/naive.nika)" "$work/nikaia.naive" "$input"
count 'Rust, `tuned`' "$(tokens benches/brc/src/bin/tuned.rs)" "$bin/tuned" "$input"
count '**Nikaia**, `examples/1brc.nika`' "$(tokens examples/1brc.nika)" "$work/nikaia.checked" "$input"
count 'Nikaia, overflow checks off' '' "$work/nikaia.unchecked" "$input"
