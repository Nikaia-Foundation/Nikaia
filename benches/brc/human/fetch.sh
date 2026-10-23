#!/usr/bin/env bash
# The human-written programs whose code is not in this repository, cloned at
# a fixed commit into `benches/brc/target/human` (ignored by git) and built at
# the bench's settings (`build.sh`). `humans.sh` calls it.
#
# * beowolx/fast_1brc has no licence, so none of its code is here: it is
#   built as it is cloned.
# * sheplu/1brc is MIT, written for aarch64 macOS: its stages from `v19` on
#   are NEON intrinsics, and every stage asks Darwin for a thread QoS class.
#   A copy with `sheplu/x86-64.patch` applied builds on x86-64 Linux: the
#   patch compiles the NEON functions on aarch64 only and makes the QoS call
#   a no-op elsewhere, and changes nothing else. Its fastest stage without
#   NEON, `v12_pairs`, is the one counted.
set -euo pipefail

root=$(cd "$(dirname "$0")/../../.." && pwd)
. "$root/benches/brc/human/build.sh"

# human_fetch URL COMMIT DIR: DIR holds URL's tree at COMMIT, unchanged.
human_fetch() {
    local url=$1 commit=$2 src=$3
    if [ ! -d "$src/.git" ]; then
        mkdir -p "$human_out"
        git clone --quiet "$url" "$src"
    fi
    if [ "$(git -C "$src" rev-parse HEAD)" != "$commit" ]; then
        git -C "$src" fetch --quiet origin
        git -C "$src" checkout --quiet --detach "$commit"
    fi
    [ -z "$(git -C "$src" status --porcelain)" ] \
        || { echo "fetch.sh: $src has local changes; remove it" >&2; exit 1; }
}

human_fetch https://github.com/beowolx/fast_1brc 181fef60ea09286270b178561764839b520e2e9c \
    "$human_out/beowolx"
human_build "$human_out/beowolx" beowolx fast_1brc

human_fetch https://github.com/sheplu/1brc 7ee0fee05cb09ea1f9ae24d0e120aa4629ce1a51 \
    "$human_out/sheplu"
ported="$human_out/sheplu-x86-64"
rm -rf "$ported"
mkdir -p "$ported"
cp -r "$human_out/sheplu/Cargo.toml" "$human_out/sheplu/Cargo.lock" "$human_out/sheplu/LICENSE" \
    "$human_out/sheplu/.cargo" "$human_out/sheplu/src" "$ported/"
patch --quiet -p1 -d "$ported" < "$root/benches/brc/human/sheplu/x86-64.patch"
# A workspace of its own, or Cargo takes it for a stray member of this one.
printf '\n[workspace]\n' >> "$ported/Cargo.toml"
human_build "$ported" sheplu v12_pairs
