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

What counts as code:

  - **Doctests are code.** A fenced block inside a `///` or `//!` comment is
    compiled and executed by `cargo test`, and `crpg-core/AGENTS.md` says the
    bans hold "anywhere, including tests". Its lines are scanned. Fences tagged
    `text` or `ignore` are not compiled, so they are not scanned.
  - **Prose is not code.** Ordinary `//` lines, doc-comment prose outside a
    fence, and `/* ... */` block comments (nesting included) are skipped, so a
    comment that merely names a banned type is not a violation.

Block comments are tracked by a deliberately simple scanner, biased toward
over-reporting because this lint's dangerous failure mode is silence:

  - It stops looking for `/*` on a line once it meets a string literal, so a
    `/*` inside a string never opens a comment. The cost is that a real block
    comment sharing a line with a string literal is still scanned, which
    over-reports; the escape hatch covers it.
  - An unterminated block comment at end of file is reported, rather than
    silently swallowing the rest of the file.

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
    # `\bf(?:32|64)\b` missed every suffixed float literal: in `1.5f64` the
    # character before `f` is a digit, so there is no word boundary and the
    # type annotation was the only form the ban caught. A lookbehind for a
    # letter instead of a word boundary catches `1.5f64`, `1.0_f64` and
    # `to_f32`, while still leaving an identifier like `buf32` alone.
    ("no-float-crates", "no-float", re.compile(r"(?<![A-Za-z])f(?:32|64)\b")),
]

ESCAPE_RE = re.compile(r"// determinism-ok:\s*(\S.*)?\s*$")
COMMENT_RE = re.compile(r"^\s*//")
# `///` and `//!` are doc comments; `////` is an ordinary comment.
DOC_RE = re.compile(r"^(\s*)(///(?!/)|//!)(.*)$")
# Fence info strings that rustdoc does not compile.
UNCOMPILED_FENCE_TAGS = {"text", "ignore"}


def strip_block_comments(lines):
    """Blank out `/* ... */` spans, returning (scannable_lines, unterminated).

    Comment characters are replaced with spaces rather than removed, so column
    positions and any trailing `// determinism-ok:` survive intact. Nesting is
    honoured, as Rust allows it.
    """
    out = []
    depth = 0
    for raw in lines:
        chars = list(raw)
        i = 0
        while i < len(chars):
            if depth == 0:
                if chars[i] == '"' or raw.startswith("//", i):
                    # A string literal or a line comment: nothing after this on
                    # the line can open a block comment in a way worth chasing.
                    break
                if raw.startswith("/*", i):
                    depth = 1
                    chars[i] = chars[i + 1] = " "
                    i += 2
                    continue
                i += 1
            else:
                if raw.startswith("*/", i):
                    depth -= 1
                    chars[i] = chars[i + 1] = " "
                    i += 2
                    continue
                if raw.startswith("/*", i):
                    depth += 1
                    chars[i] = chars[i + 1] = " "
                    i += 2
                    continue
                chars[i] = " "
                i += 1
        if depth > 0:
            # The remainder of the line is inside a comment.
            chars[i:] = " " * (len(chars) - i)
        out.append("".join(chars))
    return out, depth > 0


def fence_is_compiled(info: str) -> bool:
    """True if rustdoc compiles a fenced doc block with this info string."""
    tags = {t.strip().lower() for t in info.replace("`", "").split(",")}
    return not (tags & UNCOMPILED_FENCE_TAGS)


def scan_targets(text):
    """Yield (lineno, scannable_text) for every line that is really code.

    Doc-comment prose and ordinary comments are dropped; the body of a compiled
    doctest fence is kept, with the `///` marker stripped so the content is
    scanned as the code it becomes.
    """
    lines = text.splitlines()
    scannable, unterminated = strip_block_comments(lines)
    in_fence = False
    fence_compiled = False
    for lineno, line in enumerate(scannable, 1):
        doc = DOC_RE.match(line)
        if doc:
            indent, marker, content = doc.groups()
            stripped = content.strip()
            if stripped.startswith("```"):
                if in_fence:
                    in_fence = False
                else:
                    in_fence = True
                    fence_compiled = fence_is_compiled(stripped.lstrip("`"))
                continue
            if in_fence and fence_compiled:
                # Scan the doctest body as code, keeping the column alignment.
                yield lineno, " " * (len(indent) + len(marker)) + content
            continue
        if in_fence:
            # A fence that a non-doc line interrupts is malformed; stop
            # treating following lines as doctest body.
            in_fence = False
        if not line.strip() or COMMENT_RE.match(line):
            continue
        yield lineno, line
    if unterminated:
        yield len(lines), "\x00unterminated"


def check_file(crate_name, path):
    found = []
    text = path.read_text(encoding="utf-8-sig")
    for lineno, line in scan_targets(text):
        if line == "\x00unterminated":
            found.append((lineno, "unterminated-block-comment", line))
            continue
        # Skip escapes, requiring a non-empty reason.
        m = ESCAPE_RE.search(line)
        if m:
            if not (m.group(1) or "").strip():
                found.append((lineno, "escape-no-reason", line))
            continue
        applies = ("both",)
        if crate_name in ("crpg-core", "crpg-rules"):
            applies = ("both", "no-float-crates")
        for scope, rule_name, pattern in RULES:
            if scope not in applies:
                continue
            if pattern.search(line):
                found.append((lineno, rule_name, line))
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
