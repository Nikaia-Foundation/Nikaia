#!/bin/sh
# Collects one row for docs/speed.md. Usage: scripts/speed.sh [--lower]
# --lower also counts `lower-std` instructions with callgrind (slow: minutes).
# Needs curl and python3 (JSON only). Will move to Nikaia (tools/*.nika) when next reworked.
# CI figures come from the unauthenticated GitHub API (rate limited: 60/h);
# if it fails, the CI columns print "-" (pass them by hand in that case).
set -u
cd "$(dirname "$0")/.."
REPO=Nikaia-Foundation/Nikaia
N=${SPEED_RUNS:-10}
commit=$(git rev-parse --short HEAD)
date=$(date +%F)
share=$(python3 scripts/self_hosting.py 2>/dev/null | sed -n 's/^Nikaia in all .*  *\([0-9.]*\) % of.*/\1/p')
tok=$(sed -n '/^| agent/,/^$/p' docs/tokenspeed.md | python3 -c '
import sys
v=[]
for l in sys.stdin:
    c=[x.strip() for x in l.split("|")]
    if len(c)>8 and c[-2].replace(",","").replace(".","").isdigit(): v.append(c[-2])
print(v[-1] if v else "-")')
ci=$(curl -sS -m 30 "https://api.github.com/repos/$REPO/actions/runs?branch=develop&per_page=$N" 2>/dev/null | python3 -c '
import sys,json,datetime as d
try: runs=json.load(sys.stdin)["workflow_runs"]
except Exception: print("- - -"); sys.exit()
p=lambda s:d.datetime.strptime(s,"%Y-%m-%dT%H:%M:%SZ")
done=[r for r in runs if r["status"]=="completed"]
secs=sorted((p(r["updated_at"])-p(r["run_started_at"])).total_seconds() for r in done)
med=int(secs[len(secs)//2]) if secs else "-"
green=sum(r["conclusion"]=="success" for r in done)
canc=sum(r["conclusion"]=="cancelled" for r in done)
print(med, "%d/%d"%(green,len(done)), "%d cancelled"%canc)')
lower="-"
echo "| $date | $commit | $ci | $tok | ${share:--} | $lower |"
python3 scripts/speed_ci.py 5   # per-job medians and share of commits with a run (note under the table)
if [ "${1:-}" = "--lower" ]; then
  bin=target/fast/nikaia
  valgrind --tool=callgrind --callgrind-out-file=/dev/null $bin lower-std 2>&1 | grep -E 'Collected' 
fi
