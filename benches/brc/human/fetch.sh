#!/usr/bin/env bash
# beowolx/fast_1brc has no licence, so none of its code is in this
# repository. This script clones it at a fixed commit into
# `benches/brc/target/human/beowolx` (ignored by git) and builds it at the
# bench's settings (`build.sh`). `humans.sh` calls it.
set -euo pipefail

root=$(cd "$(dirname "$0")/../../.." && pwd)
url=https://github.com/beowolx/fast_1brc
commit=181fef60ea09286270b178561764839b520e2e9c
. "$root/benches/brc/human/build.sh"
src="$human_out/beowolx"

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

human_build "$src" beowolx fast_1brc
