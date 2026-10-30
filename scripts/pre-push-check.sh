#!/bin/sh
# Fast gates CI runs first; seconds, no build. Run before `git push origin develop`.
set -e
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
# CI's next two gates; ca6ae08 cited an ADR decision not yet written and kept develop red an hour.
python3 scripts/check-adr-refs.py >/dev/null
python3 scripts/badges.py --check
python3 scripts/bump.py check
