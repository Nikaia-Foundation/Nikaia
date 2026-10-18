#!/usr/bin/env python3
"""Writes the site's open-issue index and one page per issue.

    python3 scripts/site-issues.py --generate <dir>   # the workflow
    python3 scripts/site-issues.py <site copy>        # scripts/site-build.sh

**The workflow makes a bundle.** `.github/workflows/issues-page.yml` runs with
the job's token (`issues: read`), reads every open issue and its comments, and
keeps `issues.json` as the one file of the branch `site-issues`, replaced on
each run. The bundle contains `issues.md`, the short index grouped by status,
and `issues/<number>.md`, the page with one issue and its comments.

**The site build takes it.** `scripts/site-build.sh` downloads that one bundle
and writes its pages into the throwaway site copy. The raw file counts against
no API limit. Only if it cannot be fetched does the build read the issues
itself, with a token if one is set; and if that fails too, the index points to
GitHub.
"""

import json
import os
import re
import sys
import urllib.request
from datetime import datetime, timezone
from html import escape
from pathlib import Path

REPO = "Nikaia-Language/Nikaia"
API = f"https://api.github.com/repos/{REPO}"
STATUSES = [
    ("decision", "Waiting for a decision"),
    ("ready", "Ready to build"),
    ("blocked", "Blocked"),
    ("note", "Notes"),
]
TOKEN = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
BUNDLE = f"https://raw.githubusercontent.com/{REPO}/site-issues/issues.json"
PAGE_PATH = re.compile(r"(?:issues\.md|issues/[1-9][0-9]*\.md)\Z")


def get(url):
    request = urllib.request.Request(url, headers={"Accept": "application/vnd.github+json"})
    if TOKEN:
        request.add_header("Authorization", f"Bearer {TOKEN}")
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def every(url):
    page, out = 1, []
    while True:
        batch = get(f"{url}{'&' if '?' in url else '?'}per_page=100&page={page}")
        out += batch
        if len(batch) < 100:
            return out
        page += 1


def anchor(issue):
    return f"issue-{issue['number']}"


def demoted(text, below):
    """Move only headings too high to sit beneath `below`."""
    text = (text or "").replace("\r\n", "\n").strip()
    out, fence = [], None
    for line in text.split("\n"):
        marker = re.match(r"\s*(`{3,}|~{3,})", line)
        if marker:
            kind = marker.group(1)[0]
            if fence is None:
                fence = kind
            elif fence == kind:
                fence = None
        if fence is None:
            heading = re.match(r"(#{1,6}) ", line)
            if heading:
                depth = min(6, max(len(heading.group(1)), below + 1))
                line = "#" * depth + line[len(heading.group(1)) :]
        out.append(line)
    return "\n".join(out)


def day(stamp):
    return stamp[:10]


def status_of(issue):
    names = {label["name"] for label in issue["labels"]}
    return next((key for key, _ in STATUSES if key in names), None)


def grouped(issues):
    groups = {key: [] for key, _ in STATUSES}
    groups[None] = []
    for issue in issues:
        groups[status_of(issue)].append(issue)
    return groups


def index_page(issues, groups):
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    sections = STATUSES + [(None, "Without a status label")]
    out = ["# Open Issues", ""]
    out.append(
        f"{len(issues)} open issues of [{REPO}](https://github.com/{REPO}/issues), "
        f"read when this site was built ({now}). Each issue has its own page; "
        "that page links to GitHub, where the issue is answered."
    )
    out.append("")
    for key, title in sections:
        if groups[key]:
            out.append(f"* [{title}](#{key or 'none'}) ({len(groups[key])})")
    out.append("")
    for key, title in sections:
        if not groups[key]:
            continue
        out.append(f'## {title} <a id="{key or "none"}"></a>')
        out.append("")
        for issue in groups[key]:
            # Keep the old fragment so a saved `/issues.html#issue-N` still
            # arrives at the right entry, while the title now opens its page.
            out.append(
                f'* <a id="{anchor(issue)}"></a>'
                f'<a href="issues/{issue["number"]}.html">'
                f"#{issue['number']} {escape(issue['title'])}</a>"
            )
        out.append("")
    return "\n".join(out) + "\n"


def issue_page(issue, issue_comments, status):
    number = issue["number"]
    labels = ", ".join(f"`{label['name']}`" for label in issue["labels"])
    count = issue["comments"]
    word = "comment" if count == 1 else "comments"
    out = [f"# Issue #{number}: {issue['title']}", ""]
    out.append(f"[Open and answer this issue on GitHub]({issue['html_url']}).")
    out.append("")
    out.append(
        f"{labels or 'no labels'} · opened {day(issue['created_at'])} · "
        f"updated {day(issue['updated_at'])} · {count} {word}"
    )
    out.append("")
    body = demoted(issue["body"], 1)
    out.append(body or "_No description._")
    if issue_comments:
        out += ["", "## Comments", ""]
        for comment in issue_comments:
            user = (comment.get("user") or {}).get("login") or "ghost"
            out.append(
                f"### {user}, [{day(comment['created_at'])}]({comment['html_url']})"
            )
            out.append("")
            out.append(demoted(comment["body"], 3) or "_No text._")
            out.append("")
    elif count:
        out += ["", f"[{count} {word} on GitHub]({issue['html_url']}).", ""]
    out += ["", f"[Back to Open Issues](../issues.md#{status or 'none'})", ""]
    return "\n".join(out)


def pages(issues, comments):
    groups = grouped(issues)
    out = {"issues.md": index_page(issues, groups)}
    for issue in issues:
        out[f"issues/{issue['number']}.md"] = issue_page(
            issue, comments.get(issue["number"], []), status_of(issue)
        )
    return out


def from_github():
    issues = [i for i in every(f"{API}/issues?state=open") if "pull_request" not in i]
    issues.sort(key=lambda i: i["number"], reverse=True)
    comments = {}
    if TOKEN:
        for issue in issues:
            if issue["comments"]:
                comments[issue["number"]] = every(issue["comments_url"])
    return pages(issues, comments)


def write_pages(root, site_pages):
    root = Path(root)
    root.mkdir(parents=True, exist_ok=True)
    if not isinstance(site_pages, dict):
        raise ValueError("the issue-page bundle is not an object")
    for relative, text in site_pages.items():
        if not PAGE_PATH.fullmatch(relative) or not isinstance(text, str):
            raise ValueError(f"invalid issue-page bundle entry: {relative!r}")
        target = root / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")


def fallback_page():
    return {
        "issues.md": (
            "# Open Issues\n\n"
            f"The open issues are on [GitHub](https://github.com/{REPO}/issues).\n"
        )
    }


def main():
    if len(sys.argv) not in (2, 3) or (len(sys.argv) == 3 and sys.argv[1] != "--generate"):
        raise SystemExit("usage: site-issues.py [--generate] <directory>")
    generate = len(sys.argv) == 3
    target = Path(sys.argv[-1])
    target.mkdir(parents=True, exist_ok=True)

    if generate:
        text = json.dumps(from_github(), ensure_ascii=False, sort_keys=True) + "\n"
        (target / "issues.json").write_text(text, encoding="utf-8")
        return

    try:
        with urllib.request.urlopen(BUNDLE, timeout=30) as response:
            site_pages = json.load(response)
        write_pages(target, site_pages)
        return
    except Exception as error:
        print(f"site-issues: {BUNDLE} could not be read: {error}", file=sys.stderr)

    try:
        site_pages = from_github()
    except Exception as error:
        print(f"site-issues: GitHub could not be read: {error}", file=sys.stderr)
        site_pages = fallback_page()
    write_pages(target, site_pages)


if __name__ == "__main__":
    main()
