#!/usr/bin/env bash
# The human-written 1BRC programs in Rust and C++, counted as `brc.sh` counts its own:
# instructions and mispredicted branches per row by callgrind, at codegen units
# 1 and 16, after each one's output has been compared with `naive`'s byte for
# byte. It prints the rows in the format of `README.md`'s table; `brc.sh`
# calls it with its row count.
#
#   benches/brc/human/humans.sh            1 000 000 rows
#   benches/brc/human/humans.sh 200000
#
# * beowolx/fast_1brc has no licence: `fetch.sh` clones it at a fixed commit
#   into `benches/brc/target/human`, nothing of it is in the repository.
# * sheplu/1brc is MIT, written for aarch64 macOS: `fetch.sh` clones it too
#   and builds `v12_pairs`, its fastest stage without NEON, from a copy with
#   `sheplu/x86-64.patch` applied, which only lets it build on x86-64 Linux.
# * lehuyduc/1brc-simd is C++ with no licence, fetched as beowolx is. It
#   writes `result.txt` in the directory it runs in rather than printing, and
#   forks once, so its runs are compared there and counted with every
#   process callgrind sees.
# * automataIA/1brc-rs is MIT: `automataia/` holds what its two binaries need.
#   `v20_safe` runs as it is; `v16_flat_table` only derived and corrected,
#   with `automataia/v16-derived.patch` applied to a copy.
#
# These programs run on every core. Callgrind runs their threads one at a
# time and counts every thread's instructions, so a count is the total work,
# not the time on the clock. beowolx takes its thread count from
# `FAST1BRC_THREADS` and sheplu from `OBRC_THREADS` (8 unless set); both are
# set to the core count and also counted at 1. automataIA's take
# `available_parallelism` and have no setting.
set -euo pipefail

root=$(cd "$(dirname "$0")/../../.." && pwd)
rows=${1:-1000000}
work=${TMPDIR:-/tmp}/brc-bench
input="$work/measurements-$rows.txt"
here="$root/benches/brc/human"
mkdir -p "$work"

command -v valgrind >/dev/null || { echo "humans.sh needs valgrind (callgrind)"; exit 1; }

# The input and `naive`'s answer, as `brc.sh` makes them (it has, when it
# calls this script).
if [ ! -s "$input" ] || [ ! -s "$work/out.naive.$rows" ]; then
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 CARGO_TARGET_DIR="$work/target-cu16" \
        cargo build --manifest-path "$root/Cargo.toml" --release --quiet -p brc-bench
    [ -s "$input" ] || "$work/target-cu16/release/gen" "$rows" "$input" 2>/dev/null
    "$work/target-cu16/release/naive" "$input" > "$work/out.naive.$rows"
fi
expected="$work/out.naive.$rows"

# --- the builds ------------------------------------------------------------

"$here/fetch.sh"
. "$here/build.sh"
human_build "$here/automataia" v20 v20_safe
derived="$human_out/automataia-v16-derived"
rm -rf "$derived"
mkdir -p "$derived"
cp -r "$here/automataia/Cargo.toml" "$here/automataia/Cargo.lock" "$here/automataia/LICENSE" \
    "$here/automataia/.cargo" "$here/automataia/src" "$derived/"
patch --quiet -p1 -d "$derived" < "$here/automataia/v16-derived.patch"
human_build "$derived" v16d v16_flat_table
bin="$human_out/bin"

# The CPU level `native` stands for. Valgrind cannot run AVX-512, so where
# `native` has it the authors' own build is checked for its output and not
# counted; `authors-v3` is their build with the CPU capped at `x86-64-v3`.
native_countable=yes
grep -q -w avx512f /proc/cpuinfo && native_countable=no

# --- the same answer, first ------------------------------------------------

cores=$(nproc)
export OBRC_THREADS=$cores
for p in beowolx sheplu v20 v16d; do
    for s in $human_settings; do
        "$bin/$p.$s" "$input" | cmp -s - "$expected" \
            || { echo "FAIL: $p.$s disagrees with naive"; exit 1; }
    done
done
for s in cu1 cu16; do
    FAST1BRC_THREADS=1 "$bin/beowolx.$s" "$input" | cmp -s - "$expected" \
        || { echo "FAIL: beowolx.$s at 1 thread disagrees with naive"; exit 1; }
    OBRC_THREADS=1 "$bin/sheplu.$s" "$input" | cmp -s - "$expected" \
        || { echo "FAIL: sheplu.$s at 1 thread disagrees with naive"; exit 1; }
done
# lehuyduc's runs, each in a directory of its own for its `result.txt`.
lehuyduc_dir="$work/lehuyduc"
for s in v3 v3-1 authors; do
    rm -rf "$lehuyduc_dir" && mkdir -p "$lehuyduc_dir"
    ( cd "$lehuyduc_dir" && "$bin/lehuyduc.$s" "$input" )
    cmp -s "$lehuyduc_dir/result.txt" "$expected" \
        || { echo "FAIL: lehuyduc.$s disagrees with naive"; exit 1; }
done

# --- the counts ------------------------------------------------------------

# Instructions and mispredictions per row of one run, as two cells; the run
# under callgrind is compared with `naive` too.
count() {
    valgrind --tool=callgrind --branch-sim=yes --callgrind-out-file="$work/cg-human.out" \
        "$@" "$input" > "$work/out.human" 2>/dev/null
    cmp -s "$work/out.human" "$expected" \
        || { echo "FAIL: $* under callgrind disagrees with naive" >&2; kill $$; }
    awk -v r="$rows" '/^summary:/ { printf "%.1f | %.2f", $2 / r, ($4 + $6) / r }' "$work/cg-human.out"
}

# The same for lehuyduc: its `result.txt` compared, and the counts of the
# process it forks summed with its own.
count_lehuyduc() {
    rm -rf "$lehuyduc_dir" && mkdir -p "$lehuyduc_dir"
    ( cd "$lehuyduc_dir" && valgrind --tool=callgrind --branch-sim=yes --trace-children=yes \
        --callgrind-out-file="$lehuyduc_dir/cg.%p" "$1" "$input" >/dev/null 2>&1 )
    cmp -s "$lehuyduc_dir/result.txt" "$expected" \
        || { echo "FAIL: $1 under callgrind disagrees with naive" >&2; kill $$; }
    cat "$lehuyduc_dir"/cg.* | awk -v r="$rows" \
        '/^summary:/ { i += $2; m += $4 + $6 } END { printf "%.1f | %.2f", i / r, m / r }'
}

# Tokens by `scripts/tokens.nika`, summed over the files a binary is built
# from, each counted whole.
nikaia=${NIKAIA:-$root/target/release/nikaia}
[ -x "$nikaia" ] || cargo build --manifest-path "$root/Cargo.toml" --release --quiet -p nikaia
tokens() {
    ( cd "$root" && "$nikaia" run scripts/tokens.nika -- "$@" ) 2>/dev/null \
        | awk '{ n += $1 } END { print n }'
}
a="$here/automataia/src"
t_beowolx=$(tokens "$human_out/beowolx/src/main.rs")
t_v20=$(tokens "$a/bin/v20_safe.rs" "$a/lib.rs")
t_v16d=$(tokens "$derived/src/bin/v16_flat_table.rs" "$derived/src/lib.rs" "$derived/src/hash.rs")
# The binary and the library files it uses, whole: their tests and the other
# stages' variants included, so at most what `v12_pairs` needs.
s="$human_out/sheplu/src"
# The same rule applied to C++: its comments are Rust's.
t_lehuyduc=$(tokens "$human_out/lehuyduc/main.cpp" "$human_out/lehuyduc/my_timer.h")
t_sheplu=$(tokens "$s/bin/v12_pairs.rs" "$s/lib.rs" "$s/chunk.rs" "$s/inline_table.rs" \
    "$s/output.rs" "$s/parse.rs" "$s/swar.rs" "$s/sys.rs")

# The rows of one program: the bench's settings, then its authors' build.
rows_of() {
    local p=$1 label=$2 tokens=$3
    echo "| $label, $cores threads | $(count "$bin/$p.cu1") | $(count "$bin/$p.cu16") | $tokens |"
    if [ "$p" = beowolx ] || [ "$p" = sheplu ]; then
        echo "| $label, 1 thread | $(FAST1BRC_THREADS=1 OBRC_THREADS=1 count "$bin/$p.cu1") | $(FAST1BRC_THREADS=1 OBRC_THREADS=1 count "$bin/$p.cu16") | |"
    fi
    echo "| $label, \`x86-64-v3\`, $cores threads | $(count "$bin/$p.v3-cu1") | $(count "$bin/$p.v3-cu16") | |"
    if [ "$native_countable" = yes ]; then
        echo "| $label, as its authors build it (\`native\`, fat LTO, 1 unit, \`panic = \"abort\"\`) | $(count "$bin/$p.authors") | | | |"
    else
        echo "| $label, as its authors build it (\`native\`, fat LTO, 1 unit, \`panic = \"abort\"\`) | not countable with callgrind (AVX-512) | | | | |"
    fi
    echo "| $label, its authors' profile at \`x86-64-v3\` | $(count "$bin/$p.authors-v3") | | | |"
}
rows_of beowolx 'Rust, human: [beowolx `fast_1brc`](https://github.com/beowolx/fast_1brc)' "$t_beowolx"
rows_of sheplu 'Rust, human: [sheplu `v12_pairs`](https://github.com/sheplu/1brc), on x86-64' "at most $t_sheplu"
rows_of v20 'Rust, human: [automataIA `v20_safe`](https://github.com/automataIA/1brc-rs)' "$t_v20"
rows_of v16d 'Rust, human: [automataIA `v16_flat_table`](https://github.com/automataIA/1brc-rs), derived and corrected' "$t_v16d"

# lehuyduc: one translation unit, no codegen units, so the one count stands in
# both columns; no build at `x86-64`, which it does not compile for.
label='C++, human: [lehuyduc `1brc-simd`](https://github.com/lehuyduc/1brc-simd), `g++ -O3`'
c=$(count_lehuyduc "$bin/lehuyduc.v3")
echo "| $label, \`x86-64-v3\`, $cores threads | $c | $c | $t_lehuyduc |"
c=$(count_lehuyduc "$bin/lehuyduc.v3-1")
echo "| $label, \`x86-64-v3\`, 1 thread | $c | $c | |"
if [ "$native_countable" = yes ]; then
    echo "| $label, as its authors build it (\`native\`) | $(count_lehuyduc "$bin/lehuyduc.authors") | | | |"
else
    echo "| $label, as its authors build it (\`native\`) | not countable with callgrind (AVX-512) | | | | |"
fi
