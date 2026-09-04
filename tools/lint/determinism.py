#!/usr/bin/env python3
"""Determinism lint for the CRPG simulation crates.

Scans every .rs file under crates/crpg-core/, crates/crpg-rules/ and
crates/crpg-sim/ and fails on patterns that would make the deterministic
simulation non-deterministic:

All three crates:
  - HashMap / HashSet iteration (use IndexMap or BTreeMap)
  - SystemTime / Instant / std::time:: (wall clock is not sim time)
  - std::thread
  - rand:: / thread_rng (use crpg_core's seeded RNG)

crpg-core and crpg-rules only:
  - f32 / f64 (rules maths uses integers or fixed-point)

crpg-sim is deliberately exempt from the float ban: spec 2.4 puts spatial
positions in f32, outside the rules path. Everything a rule reads is Fx16_16
or an integer, and that is enforced one layer down in crpg-rules.

Lines that are comments are skipped. Any line ending in
  // determinism-ok: <reason>
is skipped, provided <reason> is non-empty.

Usage: python tools/lint/determinism.py [ROOT]
ROOT defaults to the repo root; an explicit ROOT is used by the self-tests.
Exit 0 if clean, 1 otherwise.
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
CRATES = ["crpg-core", "crpg-rules", "crpg-sim"]

# (crate name, rule name, regex)
RULES = [
    ("both", "no-hashmap", re.compile(r"\bHashMap\b")),
    ("both", "no-hashset", re.compile(r"\bHashSet\b")),
    ("both", "no-wallclock", re.compile(r"\b(SystemTime|Instant)\b|std::time::")),
    ("both", "no-thread", re.compile(r"std::thread")),
    ("both", "no-external-rng", re.compile(r"\brand::|thread_rng")),
    ("no-float-crates", "no-float", re.compile(r"\bf(?:32|64)\b")),
]

ESCAPE_RE = re.compile(r"// determinism-ok:\s*(\S.*)?\s*$")
COMMENT_RE = re.compile(r"^\s*//")


def check_file(crate_name, path):
    found = []
    for lineno, raw in enumerate(path.read_text(encoding="utf-8-sig").splitlines(), 1):
        line = raw.strip()
        if not line or COMMENT_RE.match(raw):
            continue
        # Skip escapes, requiring a non-empty reason.
        m = ESCAPE_RE.search(raw)
        if m:
            if not (m.group(1) or "").strip():
                found.append((lineno, "escape-no-reason", raw))
            continue
        applies = ("both",)
        if crate_name in ("crpg-core", "crpg-rules"):
            applies = ("both", "no-float-crates")
        for scope, rule_name, pattern in RULES:
            if scope not in applies:
                continue
            if pattern.search(raw):
                found.append((lineno, rule_name, raw))
    return found


def main(args):
    root = pathlib.Path(args[0]) if args else ROOT
    violations = []
    for crate in CRATES:
        base = root / "crates" / crate
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            for lineno, rule_name, line in check_file(crate, path):
                violations.append((path, lineno, rule_name, line))

    for path, lineno, rule_name, line in violations:
        print(f"VIOLATION {path}:{lineno} {rule_name} {line.strip()}")
    return 1 if violations else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
