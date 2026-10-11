#!/bin/sh
# Two sessions raise the version at once; the second rebases and repairs it with bump.py.
set -e
E=$(cd "$(dirname "$0")" && pwd)
ROOT=$(git -C "$E" rev-parse --show-toplevel)
T=${TMPDIR:-/tmp}/bump-test; rm -rf $T; mkdir -p $T; cd $T
git init -q --bare bare.git
mkdir seed; git -C "$ROOT" archive HEAD docs/specification CHANGELOG.md scripts assets/badges crates/nikaia/src crates/nikaia-std/src/tools | tar -x -C seed
git -C seed init -q -b develop; git -C seed config user.email t@t; git -C seed config user.name t
git -C seed add -A; git -C seed commit -qm base; git -C seed push -q $T/bare.git develop
git clone -q -b develop bare.git a; git clone -q -b develop bare.git b
for d in a b; do git -C $d config user.email t@t; git -C $d config user.name t; done
B="python3 $E/bump.py"
# session a
cd $T/a
printf 'fn a_x() -> i64 sync {\n    return 1\n}\n' > crates/nikaia-std/src/tools/a_x.nika
VA=$($B new --title "a: one thing (#1)" --entry "moved a_x" --nika a_x.nika="check: a thing")
git add -A; git commit -qm "a: one thing ($VA)"; git push -q origin HEAD:develop
# session b: same base, same number, plus a spec text change elsewhere
cd $T/b
printf 'fn b_y() -> i64 sync {\n    return 2\n}\n' > crates/nikaia-std/src/tools/b_y.nika
sed -i 's/^### 1.1. What Nikaia is$/### 1.1. What Nikaia is (b)/' docs/specification/10-nikaia-light.md
VB=$($B new --version $VA --title "b: other thing (#2)" --entry "moved b_y" --nika b_y.nika="emit: b thing")
git add -A; git commit -qm "b: other thing ($VB)"
echo "a took $VA, b took $VB"
git pull -q --rebase origin develop 2>/dev/null || true
echo "conflicted: $(git diff --name-only --diff-filter=U | tr '\n' ' ')"
$B rebase-fix
GIT_EDITOR=true git rebase --continue >/dev/null
$B check && echo "check: ok"
echo "--- top of CHANGELOG"; grep -m3 '^## \[' CHANGELOG.md
grep -h '^\*\*Version' docs/specification/*.md | sort | uniq -c
grep -n 'a_x.nika\|b_y.nika' scripts/self_hosting.py
grep -c '(b)$' docs/specification/10-nikaia-light.md
git log --oneline -3
# second round: b pushes, a has two commits queued with the same numbers
git push -q origin HEAD:develop
cd $T/a
VA2=$($B new --version $VB --title "a: second (#3)" --entry "x" --nika a_x.nika="check: a thing, again"); git commit -qam "a: second ($VA2)"
VA3=$($B new --title "a: third (#4)" --entry "y"); git commit -qam "a: third ($VA3)"
echo "a queued $VA2 $VA3"; grep a_x.nika scripts/self_hosting.py
git pull -q --rebase origin develop 2>/dev/null || true
n=0; while git rev-parse -q --verify REBASE_HEAD >/dev/null && [ $n -lt 4 ]; do n=$((n+1)); $B rebase-fix | tail -3; GIT_EDITOR=true git rebase --continue >/dev/null 2>&1 || true; done
$B check && echo "check: ok"
grep -m5 '^## \[' CHANGELOG.md
git log --format=%s -4
