## Task
Clarify the ADR immutability rule after T002b edited ADR-0004. Human-decision task.

## Why this is deferred

`docs/architecture/README.md` says ADRs are "created for decisions; records are
immutable — never edited". But the review found T002b appended/amended ADR-0004
during its work. The project also relies on dated, dated appendices (ADR-0007
sets a new policy rather than editing the old decision).

## Decision to make
- Adopt an explicit rule, e.g.: *the "Decision" section is immutable; evidence,
  corrections, and links may be appended dated*. Record it in
  `docs/architecture/README.md`.
- Optionally provide a short "ADR lifecycle" note: new decision supersedes old
  (new ADR) vs edit-in-place (correction), with guidance.

## Deliverable
- One paragraph or bullet list in `docs/architecture/README.md` defining the
  immutable-vs-append/fix boundary.

## Constraints
- Doc-only; no source or lint changes. Human sign-off so the docs rule is
  stable for future tasks.