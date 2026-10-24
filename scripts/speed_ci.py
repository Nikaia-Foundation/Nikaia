#!/usr/bin/env python3
"""CI detail for scripts/speed.sh: median seconds per job over the last runs on
develop, and the share of develop's commits of the last 24 h that got a run.
Unauthenticated GitHub API (60 requests/h). Prints two lines."""
import json, subprocess, sys, urllib.request, datetime as d
REPO = "Nikaia-Foundation/Nikaia"
N = int(sys.argv[1]) if len(sys.argv) > 1 else 5
get = lambda u: json.load(urllib.request.urlopen("https://api.github.com/repos/%s/%s" % (REPO, u), timeout=30))
p = lambda s: d.datetime.strptime(s, "%Y-%m-%dT%H:%M:%SZ")
try:
    runs = []
    for page in (1, 2, 3):  # 100 runs a page, enough for 24 h
        runs += get("actions/runs?branch=develop&per_page=100&event=push&page=%d" % page)["workflow_runs"]
    jobs = {}
    done = [r for r in runs if r["status"] == "completed"][:N]
    for r in done:
        for j in get("actions/runs/%d/jobs?per_page=100" % r["id"])["jobs"]:
            if j["conclusion"] in ("success", "failure"):
                jobs.setdefault(j["name"], []).append((p(j["completed_at"]) - p(j["started_at"])).total_seconds())
    print("jobs(median s, n):", " ".join("%s=%d(%d)" % (k, sorted(v)[len(v) // 2], len(v)) for k, v in sorted(jobs.items())))
    shas = {r["head_sha"] for r in runs}
    commits = subprocess.run(["git", "log", "origin/develop", "--since=24 hours ago", "--format=%H"], capture_output=True, text=True).stdout.split()
    got = sum(c in shas for c in commits)
    print("commits with a run (24h): %d/%d" % (got, len(commits)))
except Exception as e:
    print("jobs: -\ncommits with a run: -  (%s)" % e)
