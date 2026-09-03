#!/usr/bin/env python3
"""Self-tests for tools/lint/deps.py using temporary fixture crate trees."""

import os
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


def _make_tree(tmp: Path, crates: dict[str, str]) -> None:
    """Create a temporary crate tree.  crates = {name: [dep, ...]}."""
    for name, dep_list in crates.items():
        crate_dir = tmp / name
        crate_dir.mkdir(parents=True, exist_ok=True)
        deps_entries = ""
        for d in dep_list:
            deps_entries += f'{d} = "0.1"\n'
        cargo = textwrap.dedent(f"""\
            [package]
            name = "{name}"
            version = "0.1.0"
            edition = "2021"

            [dependencies]
            {deps_entries}
        """)
        (crate_dir / "Cargo.toml").write_text(cargo)


class TestCleanGraph(unittest.TestCase):
    def test_clean(self):
        tmp = Path(tempfile.mkdtemp())
        try:
            _make_tree(tmp, {
                "crpg-core": [],
                "crpg-data": ["crpg-core"],
                "crpg-rules": ["crpg-core", "crpg-data"],
            })
            internal, external = deps.build_graph(tmp)
            violations = deps.check_godot(external) + deps.check_cycles(internal) + deps.check_allowed(internal)
            self.assertEqual(violations, [])
        finally:
            shutil.rmtree(tmp)


class TestCycle(unittest.TestCase):
    def test_cycle(self):
        tmp = Path(tempfile.mkdtemp())
        try:
            _make_tree(tmp, {
                "crpg-core": ["crpg-data"],
                "crpg-data": ["crpg-core"],
            })
            internal, external = deps.build_graph(tmp)
            violations = deps.check_cycles(internal)
            self.assertTrue(any("cycle" in v for v in violations))
        finally:
            shutil.rmtree(tmp)


class TestUpwardEdge(unittest.TestCase):
    def test_upward(self):
        tmp = Path(tempfile.mkdtemp())
        try:
            _make_tree(tmp, {
                "crpg-core": ["crpg-sim"],
                "crpg-data": [],
                "crpg-sim": [],
            })
            internal, external = deps.build_graph(tmp)
            violations = deps.check_allowed(internal)
            self.assertTrue(any("allowed-edges" in v for v in violations))
        finally:
            shutil.rmtree(tmp)


class TestGodotRule(unittest.TestCase):
    def test_non_godot_depends_on_godot(self):
        tmp = Path(tempfile.mkdtemp())
        try:
            _make_tree(tmp, {
                "crpg-core": ["godot"],
            })
            internal, external = deps.build_graph(tmp)
            violations = deps.check_godot(external)
            self.assertTrue(any("godot-only" in v for v in violations))
        finally:
            shutil.rmtree(tmp)

    def test_godot_depends_on_godot_ok(self):
        tmp = Path(tempfile.mkdtemp())
        try:
            _make_tree(tmp, {
                "crpg-godot": ["godot"],
            })
            internal, external = deps.build_graph(tmp)
            violations = deps.check_godot(external)
            self.assertEqual(violations, [])
        finally:
            shutil.rmtree(tmp)


if __name__ == "__main__":
    unittest.main()
