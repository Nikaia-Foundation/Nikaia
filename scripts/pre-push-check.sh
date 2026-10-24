#!/bin/sh
# Fast gates CI runs first; seconds, no build. Run before `git push origin develop`.
set -e
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
python3 scripts/badges.py --check
python3 scripts/bump.py check
