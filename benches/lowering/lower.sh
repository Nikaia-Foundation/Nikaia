#!/usr/bin/env bash
# What lowering costs (#727): (a) a cold `nikaia lower-std` of the tools
# package, its phases (NIKAIA_LOWER_TIMES) and a check that `lowered.rs` and `lowered/` come
# out byte for byte as committed; (b) callgrind's instruction count (Ir) of a
# cold `nikaia lower` of one mid-size real source.
#
#   benches/lowering/lower.sh          both
#   benches/lowering/lower.sh std      only (a)
#   benches/lowering/lower.sh count    only (b)
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
what=${1:-all}
work=${TMPDIR:-/tmp}/lowering-bench
nikaia="$root/target/release/nikaia"
source_file="$root/crates/rs2nika/convert/src/main.nika"
mkdir -p "$work"

cargo build --manifest-path "$root/Cargo.toml" --release --quiet -p nikaia --bin nikaia

if [ "$what" = all ] || [ "$what" = std ]; then
    rm -rf "$work/cache" && mkdir -p "$work/cache"
    echo "== (a) cold lower-std, phases"
    NIKAIA_CACHE_DIR="$work/cache" NIKAIA_LOWER_TIMES=1 "$nikaia" lower-std
    git -C "$root" diff --exit-code --stat crates/nikaia-std/src/tools/lowered.rs crates/nikaia-std/src/tools/lowered \
        && echo "lowered: identical"
fi

if [ "$what" = all ] || [ "$what" = count ]; then
    command -v valgrind >/dev/null || { echo "needs valgrind (callgrind)"; exit 1; }
    echo "== (b) callgrind, cold lower of ${source_file#$root/}"
    out="$work/callgrind.out"
    valgrind --tool=callgrind --callgrind-out-file="$out" \
        "$nikaia" lower --no-cache -o "$work/main.rs" "$source_file" 2>"$work/valgrind.log" >/dev/null
    grep -E '^(summary|totals):' "$out" | head -1 | awk '{printf "Ir: %'"'"'d\n", $2}'
fi
