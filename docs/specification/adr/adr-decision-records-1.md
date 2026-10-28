# ADR-decision-records-1: A Record Is Named by Its Topic and a Number, `ADR-<topic>-<n>`; the Numbered Records Keep Their Numbers Until a Consolidation Replaces Them

**Status:** Accepted
**Date:** October 28, 2026
**Answers:** how a decision record is named, so that a reader finds it by what it decides and two sessions writing at once do not take the same name
**Supersedes / Related:** [`README.md`](README.md) (*Writing a new one*, *Reserved numbers*: the next free number was the name), [`TEMPLATE.md`](TEMPLATE.md), `scripts/check-adr-refs.py` (one record per name, every citation resolves), [ADR-258](adr-258.md) (the weights a record is decided by)

## 1. Question

A record has been named by the next free number: `ADR-296`, cited as
`ADR-296 D7` in about 17 000 places, more than 7 000 of them comments in
`crates/`. A number says nothing about what it names. A comment that cites
`ADR-296 D7` is followed only with the index open; a reader who looks for the
record on regular expressions has to search the titles. And the next free
number is one counter for every session: two sessions writing at once took the
same number more than once (`check-adr-refs.py` checks for it because it
happened), and a record written on a branch took a number `develop` gave to
another record meanwhile.

Is a record named by its topic, and if so, how?

## 2. How others do it

| | a record's name | found by |
|---|---|---|
| Nygard's ADRs, `adr-tools`, MADR | a sequence number and a slug, `0042-use-postgres.md` | the slug in the file name; cited by number |
| Rust RFCs, Python PEPs | a number given when merged or accepted | an index |
| Kubernetes KEPs | a number and a directory per area, `sig-network/1234-…` | the area |
| IETF RFCs | a number | an index |

A number with a slug finds the file, but the citation is the number, and the
counter stays one for everyone.

## 3. Decisions

### D1 — A record is named `ADR-<topic>-<n>`.

`<topic>` is one to three words, each lowercase letters and digits starting
with a letter, joined by `-`: `regex`, `decision-records`, `text-positions`.
`<n>` counts the records of that topic from 1. The record is the file
`adr-<topic>-<n>.md`, its first line is `# ADR-<topic>-<n>: <the decision>`,
and a decision in it is cited as `ADR-regex-1 D4`.

**Why.** A name that says what it decides is found without the index, in a
comment, in a file list, in a search. A counter per topic is shared only by
sessions working on the same topic, who have to know of each other anyway.
**Rejected:** numbers alone (found only through the index; one counter for
everyone); words in CamelCase (`ADR-RegEx-1`) - the default filesystems of
macOS and Windows do not tell `RegEx` from `Regex`, so two names that look
different are one file there; the number of the issue that asked (not every
record has one issue, and the issue numbers overlap the record numbers already
given); more than three words (a name, not a title).

### D2 — The topic is what the record decides.

A record that touches several topics takes the one it decides about and names
the others under **Supersedes / Related**. The next record on a topic takes the
next number; a change to a decision is still made in the record that holds it
(README, *updated in place*). A new topic is taken when none of the existing
ones fits; the index lists the topics in use.

### D3 — The numbered records keep their numbers until a consolidation replaces them.

No record is renamed. A numbered record leaves the live set the way records
always have: a consolidating record of the new scheme states its decisions in
their current form, the numbered record moves to
[`docs/history/adr/`](../../history/adr/), and every citation of it moves to the
new record in the same change. Nothing live cites an archived record, as
before. The archive's README maps each archived decision to the live one.

### D4 — A name is claimed before it is written, as a number was.

The index's **Reserved numbers** table holds names of both schemes: a row
`regex-2` claims that name as a row `385` claims that number. A row is deleted
in the commit that adds the record.

### D5 — `check-adr-refs.py` checks both schemes, and stays Python until `std::regex` exists.

It checks that every record has a valid name, one index row and a title that
agrees, that every `Dn` and `§` cited resolves, that no live file cites an
archived record, and that every topic name cited names a record.

The rule that a Python tool moves to Nikaia when it is reworked
(`CLAUDE.md`) is broken here on purpose: the tool is made of regular
expressions, and `std::regex` ([#673](https://github.com/Nikaia-Foundation/Nikaia/issues/673))
is decided but not built. Written in Nikaia today it would be the hand-written
scanning #673 exists to replace. It moves when #673's run-time patterns are
built, and is their first program.

## 4. Consequences

* A new record is named by its topic from now on; the next free number is not
  taken any more.
* Two schemes live side by side until the numbered records are consolidated.
  The consolidation is a list of its own, kept in an issue: the old numbers,
  the topic each joins, the citations to move.
* A citation in a comment says what it points at.
