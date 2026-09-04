#!/usr/bin/env python3
"""Self-tests for the determinism lint.

Builds temporary fixture crate trees and asserts the lint produces the expected
violations. Run: python -m unittest discover -s tools/lint -p "test_*.py"
"""

import pathlib
import subprocess
import sys
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
SCRIPT = HERE / "determinism.py"

CLEAN_SRC = """
pub fn apply(world: &mut World) {
    let v: Vec<i32> = vec![1, 2, 3];
    for x in &v {
        let _ = x;
    }
}
"""

HASHMAP_SRC = """
use std::collections::HashMap;
pub fn f() {
    let mut h = HashMap::new();
    h.insert(1, 2);
    for (k, _) in &h {
        let _ = k;
    }
}
"""

HASHSET_SRC = """
use std::collections::HashSet;
pub fn f() {
    let mut s = HashSet::new();
    s.insert(1);
}
"""

WALLCLOCK_SRC = """
use std::time::Instant;
pub fn f() {
    let start = Instant::now();
    let _ = start;
}
"""

THREAD_SRC = """
pub fn f() {
    std::thread::spawn(|| {});
}
"""

RNG_SRC = """
pub fn f() {
    let mut rng = rand::thread_rng();
    let _ = rng.gen::<u32>();
}
"""

COMMENT_SRC = """
// HashMap iteration is banned, but this is just a comment.
// rand::thread_rng is banned too.
pub fn f() { let x = 1; let _ = x; }
"""

ESCAPE_OK_SRC = """
pub fn f() {
    let mut h = std::collections::HashMap::new(); // determinism-ok: only used as a lookup, never iterated
    let _ = h;
}
"""

ESCAPE_NO_REASON_SRC = """
pub fn f() {
    let h = std::collections::HashMap::new(); // determinism-ok:
    let _ = h;
}
"""

FLOAT_SRC = """
pub fn ac() -> f64 { 10.5 }
pub fn hp() -> i32 { 20 }
"""

FLOAT_SIM_SRC = """
pub struct Transform { x: f64, y: f64 }
"""

# A crate root carrying a UTF-8 BOM, as several stubs in this repo do. The BOM
# must not hide a violation on line 1.
BOM_SRC = "﻿use std::collections::HashMap;\n"


def make_crate(root, name, sources):
    crate_dir = root / "crates" / name
    crate_dir.mkdir(parents=True)
    for fname, content in sources.items():
        (crate_dir / fname).write_text(content, encoding="utf-8")


def run_lint(root):
    return subprocess.run(
        [sys.executable, str(SCRIPT), str(root)],
        capture_output=True,
        text=True,
    )


def rules_of(root):
    """Return (exit_code, [rule_name, ...]) for a lint run over `root`."""
    proc = run_lint(root)
    rules = []
    for line in proc.stdout.splitlines():
        parts = line.split()
        if len(parts) >= 3 and parts[0] == "VIOLATION":
            rules.append(parts[2])
    return proc.returncode, rules


class LintCase(unittest.TestCase):
    """Base class providing a one-crate fixture run."""

    def lint_one(self, crate, src, fname="lib.rs"):
        with tempfile.TemporaryDirectory() as d:
            root = pathlib.Path(d)
            make_crate(root, crate, {fname: src})
            return rules_of(root)


class TestClean(LintCase):
    def test_clean_tree_passes(self):
        with tempfile.TemporaryDirectory() as d:
            root = pathlib.Path(d)
            for crate in ("crpg-core", "crpg-rules", "crpg-sim"):
                make_crate(root, crate, {"lib.rs": CLEAN_SRC})
            code, rules = rules_of(root)
            self.assertEqual(code, 0, f"clean tree should pass, got {rules}")


class TestBannedPatterns(LintCase):
    """The `both`-scope rules apply to every linted crate."""

    CASES = [
        ("no-hashmap", "crpg-core", HASHMAP_SRC),
        ("no-hashmap", "crpg-rules", HASHMAP_SRC),
        ("no-hashmap", "crpg-sim", HASHMAP_SRC),
        ("no-hashset", "crpg-core", HASHSET_SRC),
        ("no-hashset", "crpg-rules", HASHSET_SRC),
        ("no-wallclock", "crpg-core", WALLCLOCK_SRC),
        ("no-wallclock", "crpg-rules", WALLCLOCK_SRC),
        ("no-thread", "crpg-rules", THREAD_SRC),
        ("no-external-rng", "crpg-core", RNG_SRC),
        ("no-external-rng", "crpg-rules", RNG_SRC),
    ]

    def test_banned_patterns_are_detected(self):
        for expected_rule, crate, src in self.CASES:
            with self.subTest(crate=crate, rule=expected_rule):
                code, rules = self.lint_one(crate, src)
                self.assertEqual(code, 1, f"{crate}/{expected_rule} should fail")
                self.assertIn(expected_rule, rules)


class TestSkips(LintCase):
    def test_banned_pattern_in_comment_passes(self):
        code, rules = self.lint_one("crpg-rules", COMMENT_SRC)
        self.assertEqual(code, 0, f"comment should be skipped, got {rules}")

    def test_escape_with_reason_passes(self):
        code, rules = self.lint_one("crpg-rules", ESCAPE_OK_SRC)
        self.assertEqual(code, 0, f"valid escape should pass, got {rules}")

    def test_escape_without_reason_fails(self):
        code, rules = self.lint_one("crpg-rules", ESCAPE_NO_REASON_SRC)
        self.assertEqual(code, 1, "escape with no reason should fail")
        self.assertIn("escape-no-reason", rules)


class TestFloatScope(LintCase):
    """Floats are banned in crpg-core and crpg-rules, allowed in crpg-sim.

    crpg-sim holds spatial positions, which spec 2.4 puts in f32 outside the
    rules path. If that decision is ever revisited, this test is the place it
    shows up.
    """

    def test_float_in_core_fails(self):
        code, rules = self.lint_one("crpg-core", FLOAT_SRC)
        self.assertEqual(code, 1, "f64 in crpg-core should fail")
        self.assertIn("no-float", rules)

    def test_float_in_rules_fails(self):
        code, rules = self.lint_one("crpg-rules", FLOAT_SRC)
        self.assertEqual(code, 1, "f64 in crpg-rules should fail")
        self.assertIn("no-float", rules)

    def test_float_in_sim_passes(self):
        code, rules = self.lint_one("crpg-sim", FLOAT_SIM_SRC)
        self.assertEqual(code, 0, f"f64 in crpg-sim should pass, got {rules}")


class TestEncoding(LintCase):
    def test_bom_does_not_hide_a_violation_on_line_one(self):
        code, rules = self.lint_one("crpg-rules", BOM_SRC)
        self.assertEqual(code, 1, "BOM should not mask the first line")
        self.assertIn("no-hashmap", rules)


if __name__ == "__main__":
    unittest.main()
