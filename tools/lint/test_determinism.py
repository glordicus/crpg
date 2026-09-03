#!/usr/bin/env python3
"""Self-tests for the determinism lint.

Builds temporary fixture crate trees and asserts the lint produces the expected
violations. Run: python tools/lint/test_determinism.py
"""

import pathlib
import subprocess
import sys
import tempfile

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

FLOAT_RULES_SRC = """
pub fn ac() -> f64 { 10.5 }
pub fn hp() -> i32 { 20 }
"""

FLOAT_SIM_SRC = """
pub struct Transform { x: f64, y: f64 }
"""


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
    proc = run_lint(root)
    rules = []
    for line in proc.stdout.splitlines():
        parts = line.split()
        if len(parts) >= 3 and parts[0] == "VIOLATION":
            rules.append(parts[2])
    return proc.returncode, rules


def test_clean():
    with tempfile.TemporaryDirectory() as d:
        root = pathlib.Path(d)
        make_crate(root, "crpg-rules", {"lib.rs": CLEAN_SRC})
        make_crate(root, "crpg-sim", {"lib.rs": CLEAN_SRC})
        code, rules = rules_of(root)
        assert code == 0, f"clean should pass, got {rules}"
        print("ok: clean file passes")


def test_banned_patterns():
    banned = [
        ("no-hashmap", "crpg-rules", HASHMAP_SRC),
        ("no-hashmap", "crpg-sim", HASHMAP_SRC),
        ("no-hashset", "crpg-rules", HASHSET_SRC),
        ("no-wallclock", "crpg-rules", WALLCLOCK_SRC),
        ("no-thread", "crpg-rules", THREAD_SRC),
        ("no-external-rng", "crpg-rules", RNG_SRC),
    ]
    for expected_rule, crate, src in banned:
        with tempfile.TemporaryDirectory() as d:
            root = pathlib.Path(d)
            make_crate(root, crate, {"lib.rs": src})
            code, rules = rules_of(root)
            assert code == 1, f"{expected_rule} should fail"
            assert expected_rule in rules, f"{crate}: expected {expected_rule}, got {rules}"
            print(f"ok: {crate} {expected_rule} detected")


def test_banned_in_comment_passes():
    with tempfile.TemporaryDirectory() as d:
        root = pathlib.Path(d)
        make_crate(root, "crpg-rules", {"lib.rs": COMMENT_SRC})
        code, rules = rules_of(root)
        assert code == 0, f"comment should be skipped, got {rules}"
        print("ok: banned pattern in comment passes")


def test_escape_ok_passes():
    with tempfile.TemporaryDirectory() as d:
        root = pathlib.Path(d)
        make_crate(root, "crpg-rules", {"lib.rs": ESCAPE_OK_SRC})
        code, rules = rules_of(root)
        assert code == 0, f"valid escape should pass, got {rules}"
        print("ok: valid determinism-ok escape passes")


def test_escape_no_reason_fails():
    with tempfile.TemporaryDirectory() as d:
        root = pathlib.Path(d)
        make_crate(root, "crpg-rules", {"lib.rs": ESCAPE_NO_REASON_SRC})
        code, rules = rules_of(root)
        assert code == 1, "escape with no reason should fail"
        assert "escape-no-reason" in rules, f"expected escape-no-reason, got {rules}"
        print("ok: escape with no reason fails")


def test_float_rules_fails_sim_ok():
    with tempfile.TemporaryDirectory() as d:
        root = pathlib.Path(d)
        make_crate(root, "crpg-rules", {"lib.rs": FLOAT_RULES_SRC})
        code, rules = rules_of(root)
        assert code == 1, "f64 in crpg-rules should fail"
        assert "no-float" in rules, f"expected no-float in rules, got {rules}"
        print("ok: f64 in crpg-rules fails")
    with tempfile.TemporaryDirectory() as d:
        root = pathlib.Path(d)
        make_crate(root, "crpg-sim", {"lib.rs": FLOAT_SIM_SRC})
        code, rules = rules_of(root)
        assert code == 0, f"f64 in crpg-sim should pass, got {rules}"
        print("ok: f64 in crpg-sim passes")


def main():
    test_clean()
    test_banned_patterns()
    test_banned_in_comment_passes()
    test_escape_ok_passes()
    test_escape_no_reason_fails()
    test_float_rules_fails_sim_ok()
    print("All determinism lint tests passed.")


if __name__ == "__main__":
    main()
