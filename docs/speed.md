# Speed from round to round

One row per round, to show how we improve. Collect it with `scripts/speed.sh`
and add the printed line to the table.

## Method

* **CI time**: median wall-clock seconds (`run_started_at` to `updated_at`) of
  the last completed runs on `develop`, from the GitHub API. A process
  measure, not a program benchmark; the runners vary by 1.4-1.9x from day to
  day, so read trends over several rounds, not single rows.
* **Green runs**: completed runs with conclusion `success` / completed runs
  (cancelled ones counted in the note). A run per commit is not guaranteed;
  the share of commits that got a run is not collected yet.
* **Tokens per moved line**: the last row of the *Agents* table in
  `docs/tokenspeed.md` (target below 50 k).
* **Self-hosting share**: `scripts/self_hosting.py`, "Nikaia in all".
* **`lower-std` instructions** (cold / warm): callgrind counts per CLAUDE.md
  (cold about 181 G, warm about 1.3 G as stated there). Not collected by the
  script by default: under valgrind it ran over 10 minutes on a shared
  machine, so the columns stay empty until someone runs it on a quiet one.

## Rounds

| Date | Commit | CI median s | Green / runs | Tokens per moved line | Share % | lower-std cold / warm (instr.) |
| :--- | :--- | ---: | :--- | ---: | ---: | :--- |
| 2026-10-24 | 28e01193 | 510 | 1/5 (1 cancelled) | 163,867 | 71.9 | - / - |

Baseline row. Missing columns: CI time per job, the share of commits that
got a CI run, and the `lower-std` counts.
