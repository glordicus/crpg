#!/usr/bin/env python3
"""Self-tests for tools/lint/deps.py using temporary fixture crate trees.

Run: python -m unittest discover -s tools/lint -p "test_*.py"
"""

import shutil
import tempfile
import textwrap
import unittest
from pathlib import Path

# Ensure the lint module is importable.
import importlib.util
SPEC = importlib.util.spec_from_file_location("deps", Path(__file__).with_name("deps.py"))
deps = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(deps)

FORBID = "#![forbid(unsafe_code)]\n//! doc\n"


def _make_tree(tmp: Path, crates: dict) -> None:
    """Create a temporary crate tree.

    `crates` maps a crate name to either a list of runtime deps, or a dict with
    any of the keys `dependencies`, `dev-dependencies`, `build-dependencies`
    and `root` (the text of src/lib.rs, defaulting to a compliant stub).
    """
    for name, spec in crates.items():
        if isinstance(spec, list):
            spec = {"dependencies": spec}
        crate_dir = tmp / name
        (crate_dir / "src").mkdir(parents=True, exist_ok=True)

        sections = ""
        for section in deps.DEP_SECTIONS:
            entries = spec.get(section, [])
            if not entries:
                continue
            sections += f"\n[{section}]\n"
            for d in entries:
                sections += f'{d} = "0.1"\n'

        cargo = textwrap.dedent(f"""\
            [package]
            name = "{name}"
            version = "0.1.0"
            edition = "2021"
        """) + sections
        (crate_dir / "Cargo.toml").write_text(cargo, encoding="utf-8")
        (crate_dir / "src" / "lib.rs").write_text(
            spec.get("root", FORBID), encoding="utf-8"
        )


class TreeCase(unittest.TestCase):
    """Base class that builds a fixture tree and tears it down."""

    def tree(self, crates: dict) -> Path:
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp, ignore_errors=True)
        _make_tree(tmp, crates)
        return tmp

    def all_violations(self, crates: dict) -> list:
        tmp = self.tree(crates)
        internal, external = deps.build_graph(tmp)
        return (
            deps.check_godot(external)
            + deps.check_cycles(deps.runtime_edges(internal))
            + deps.check_allowed(internal)
            + deps.check_unsafe(tmp)
        )


class TestCleanGraph(TreeCase):
    def test_clean(self):
        self.assertEqual(self.all_violations({
            "crpg-core": [],
            "crpg-data": ["crpg-core"],
            "crpg-rules": ["crpg-core", "crpg-data"],
        }), [])

    def test_external_dev_dependency_is_fine(self):
        # proptest/serde_json in crpg-core, as ADR-0006 authorises.
        self.assertEqual(self.all_violations({
            "crpg-core": {"dev-dependencies": ["proptest", "serde_json"]},
        }), [])


class TestCycle(TreeCase):
    def test_cycle(self):
        violations = self.all_violations({
            "crpg-core": ["crpg-data"],
            "crpg-data": ["crpg-core"],
        })
        self.assertTrue(any("cycle" in v for v in violations))


class TestUpwardEdge(TreeCase):
    def test_upward(self):
        violations = self.all_violations({
            "crpg-core": ["crpg-sim"],
            "crpg-data": [],
            "crpg-sim": [],
        })
        self.assertTrue(any("allowed-edges" in v for v in violations))

    def test_upward_dev_dependency(self):
        """A dev-dependency is still an import: crpg-core takes no workspace crate."""
        violations = self.all_violations({
            "crpg-core": {"dev-dependencies": ["crpg-testkit"]},
            "crpg-testkit": [],
        })
        self.assertTrue(
            any("allowed-edges, dev-dependencies" in v for v in violations),
            violations,
        )

    def test_upward_build_dependency(self):
        violations = self.all_violations({
            "crpg-core": {"build-dependencies": ["crpg-data"]},
            "crpg-data": [],
        })
        self.assertTrue(
            any("allowed-edges, build-dependencies" in v for v in violations),
            violations,
        )


class TestUnknownCrate(TreeCase):
    def test_crate_missing_from_the_table_fails_closed(self):
        """A new crate must be added to ALLOWED, not inherit blanket permission."""
        violations = self.all_violations({
            "crpg-core": [],
            "crpg-newthing": ["crpg-core"],
        })
        self.assertTrue(
            any("not in the allowed-edges table" in v for v in violations),
            violations,
        )


class TestGodotRule(TreeCase):
    def test_non_godot_depends_on_godot(self):
        violations = self.all_violations({"crpg-core": ["godot"]})
        self.assertTrue(any("godot-only" in v for v in violations))

    def test_non_godot_dev_depends_on_godot(self):
        violations = self.all_violations({
            "crpg-core": {"dev-dependencies": ["godot"]},
        })
        self.assertTrue(
            any("godot-only, dev-dependencies" in v for v in violations), violations
        )

    def test_godot_depends_on_godot_ok(self):
        # crpg-godot is also the one crate exempt from the forbid-unsafe rule.
        self.assertEqual(self.all_violations({
            "crpg-godot": {"dependencies": ["godot"], "root": "//! doc\n"},
        }), [])


class TestUnsafeRule(TreeCase):
    def test_missing_forbid_attribute_fails(self):
        violations = self.all_violations({
            "crpg-core": {"root": "//! no forbid attribute here\n"},
        })
        self.assertTrue(any("forbid(unsafe_code)" in v for v in violations), violations)

    def test_bom_prefixed_root_still_counts(self):
        """Several stub roots carry a UTF-8 BOM; it must not read as missing."""
        self.assertEqual(self.all_violations({
            "crpg-core": {"root": "﻿" + FORBID},
        }), [])

    def test_crate_with_no_root_fails(self):
        tmp = self.tree({"crpg-core": []})
        (tmp / "crpg-core" / "src" / "lib.rs").unlink()
        violations = deps.check_unsafe(tmp)
        self.assertTrue(any("no crate root" in v for v in violations), violations)


if __name__ == "__main__":
    unittest.main()
