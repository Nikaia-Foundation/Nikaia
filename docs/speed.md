# Speed from round to round

One row per round, to show how we improve. Collect it with `scripts/speed.sh`
and add the printed line to the table.

## Method

* **CI time**: median wall-clock seconds (`run_started_at` to `updated_at`) of
  the last completed runs on `develop`, from the GitHub API. A process
  measure, not a program benchmark; the runners vary by 1.4-1.9x from day to
  day, so read trends over several rounds, not single rows.
* **Green runs**: completed runs with conclusion `success` / completed runs
  (cancelled ones counted in the note). 
* **CI time per job**: median seconds (`started_at` to `completed_at`) of each
  job over the last 5 push runs, skipped jobs left out (`scripts/speed_ci.py`,
  jobs API, no token). Jobs that stop early (a red `bootstrap_compiler`) count
  with their short time, so read it with the green column.
* **Commits with a run**: commits on `origin/develop` of the last 24 h whose
  sha is the head of a push run / all such commits.
* **Tokens per moved line**: the last row of the *Agents* table in
  `docs/tokenspeed.md` (target below 50 k).
* **Self-hosting share**: `scripts/self_hosting.py`, "Nikaia in all".
* **`lower-std` instructions** (cold / warm): callgrind counts per CLAUDE.md
  (cold about 181 G, warm about 1.3 G as stated there). Not collected by the
  script by default: under valgrind it ran over 10 minutes on a shared
  machine, so the columns stay empty until someone runs it on a quiet one. Not
  collected in the 2026-10-24 rows for that reason.

## Rounds

| Date | Commit | CI median s | Green / runs | Tokens per moved line | Share % | lower-std cold / warm (instr.) |
| :--- | :--- | ---: | :--- | ---: | ---: | :--- |
| 2026-10-24 | 28e01193 | 510 | 1/5 (1 cancelled) | 163,867 | 71.9 | - / - |

Baseline row. Missing: the `lower-std` counts (see Method).

### Per job and coverage

| Date | Commit | Job medians s (n runs) | Commits with a run, 24 h |
| :--- | :--- | :--- | :--- |
| 2026-10-24 | 28e01193 | arm64 528, bootstrap_compiler 60, changes 18 (5) | 141/142 |

The last 5 runs were red at `bootstrap_compiler`/`arm64`, so only three jobs
ran to a result; the other jobs were skipped and have no time yet.
