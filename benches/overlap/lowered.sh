#!/usr/bin/env bash
# ADR-292 D6's number, end to end: what an `overlap { … }` block costs, not
# what the `std` function costs.
#
# `runtime.sh` measures `nikaia_std` from Rust. This counts a `.nika` program
# through the whole pipeline - the checker, the emitter, `cargo`, the runtime -
# at `user_parallelism = no`: `pair.nika`, two reads in an `overlap` block,
# against `sequential.nika`, the same two reads one after the other. Two
# binaries, and the only difference between them is the block.
#
# Both of ADR-303 D3's mechanisms are counted, because D10's decision is about
# the second one: on the completion path (`io-method = "auto"`) a pair is
# cheap, and on the fallback (`blocking`) the pair is performed in written
# order. The fallback's row is therefore the check that the decision is in the
# code.
#
# Counted with callgrind, never timed (CLAUDE.md, *Measuring performance*):
# instructions and mispredicted branches per pair, the two programs' outputs
# compared byte for byte first. See README.md.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
out=${1:-$(mktemp -d)}
pairs=${2:-20000}
size=${3:-5}

command -v valgrind >/dev/null || { echo "lowered.sh needs valgrind (callgrind)"; exit 1; }
mkdir -p "$out"
out=$(cd "$out" && pwd)
if [ -z "${NIKAIA:-}" ]; then
    cargo build --manifest-path "$root/Cargo.toml" -p nikaia --release --quiet
    NIKAIA="$root/target/release/nikaia"
fi

echo "program: benches/overlap/{pair,sequential}.nika, $pairs pairs, $size bytes/file"
echo

for name in pair sequential; do
    project="$out/$name"
    mkdir -p "$project/src" "$project/run"
    cp "$root/benches/overlap/$name.nika" "$project/src/main.nika"
    cat > "$project/nikaia.toml" <<TOML
[package]
name = "$name"
version = "0.1.0"

[build]
user-parallelism = "no"

[build.x86_64-linux]
opt-level = 3
TOML
    (cd "$project" && "$NIKAIA" build >/dev/null)

    # What was built, asserted rather than assumed: a count of two identical
    # binaries would be a very stable nothing.
    gen="$project/target/nikaia/gen/$name/$name.rs"
    if grep -q 'task::overlap2' "$gen"; then has=yes; else has=no; fi
    case "$name/$has" in
        pair/yes | sequential/no) echo "$name: task::overlap2 in the emitted Rust: $has" ;;
        *) echo "$name: task::overlap2 in the emitted Rust: $has, which is wrong"; exit 1 ;;
    esac

    for file in eins zwei; do
        head -c "$size" /dev/urandom | base64 > "$project/run/$file.txt"
        truncate -s "$size" "$project/run/$file.txt"
    done
done
cp "$out/pair/run/"*.txt "$out/sequential/run/"
echo

# Instructions and mispredictions per pair of one program, as two cells.
count() {
    (cd "$out/$1/run" && valgrind --tool=callgrind --branch-sim=yes \
        --callgrind-out-file="$out/cg.out" "$out/$1/target/nikaia/bin/$1" "$pairs" >/dev/null 2>&1)
    awk -v r="$pairs" '/^summary:/ { printf "%.1f | %.2f", $2 / r, ($4 + $6) / r }' "$out/cg.out"
}

echo "| io-method | sequential: instr/pair | mispred/pair | pair: instr/pair | mispred/pair |"
echo "|---|--:|--:|--:|--:|"
for method in auto blocking; do
    printf 'io-method = "%s"\nio-workers = 2\n' "$method" > "$out/nikaia-runtime.toml"
    export NIKAIA_RUNTIME_CONFIG="$out/nikaia-runtime.toml"

    for name in pair sequential; do
        (cd "$out/$name/run" && "$out/$name/target/nikaia/bin/$name" "$pairs") > "$out/$name.out"
    done
    cmp -s "$out/pair.out" "$out/sequential.out" \
        || { echo "io-method=$method: the two programs print different things"; exit 1; }

    echo "| $method | $(count sequential) | $(count pair) |"
done
