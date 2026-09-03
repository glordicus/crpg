#!/usr/bin/env python3
"""Dependency-direction lint: enforces the layering rule in AGENTS.md."""

import sys
import tomllib
from pathlib import Path

# Allowed edges: from_crate -> set of allowed to-crates, or None/Every.
# "Every" means any workspace crate is allowed.
# Excluded crates are stored separately.
ALLOWED: dict[str, tuple[set[str] | None, set[str]]] = {
    "crpg-core":      (set(),   set()),
    "crpg-data":      ({"crpg-core"}, set()),
    "crpg-rules":     ({"crpg-core", "crpg-data"}, set()),
    "crpg-sim":       ({"crpg-core", "crpg-data", "crpg-rules"}, set()),
    "crpg-nav":       ({"crpg-core"}, set()),
    "crpg-script":    ({"crpg-core", "crpg-data", "crpg-rules", "crpg-sim"}, set()),
    "crpg-ai":        ({"crpg-core", "crpg-rules", "crpg-sim", "crpg-nav"}, set()),
    "crpg-net":       ({"crpg-core", "crpg-data", "crpg-sim"}, set()),
    "crpg-persist":   ({"crpg-core", "crpg-data", "crpg-sim"}, set()),
    "crpg-edit":      ({"crpg-core", "crpg-data", "crpg-rules"}, set()),
    "crpg-contracts": ({"crpg-core"}, set()),
    "crpg-testkit":   (None, set()),
    "crpg-server":    (None, {"crpg-godot", "crpg-edit"}),
    "crpg-cli":       (None, {"crpg-godot"}),
    "crpg-godot":     (None, set()),
}


def discover_crates(crates_dir: Path) -> dict[str, Path]:
    """Return {crate_name: Cargo.toml_path} for every crate in the workspace."""
    crates: dict[str, Path] = {}
    for cargo in sorted(crates_dir.glob("*/Cargo.toml")):
        with open(cargo, "rb") as f:
            data = tomllib.load(f)
        crates[data["package"]["name"]] = cargo
    return crates


def parse_deps(cargo_path: Path) -> tuple[list[str], list[str]]:
    """Return (workspace_deps, external_deps) from a Cargo.toml."""
    with open(cargo_path, "rb") as f:
        data = tomllib.load(f)
    ws, ext = [], []
    for dep in data.get("dependencies", {}):
        ext.append(dep)
    return ws, ext


def build_graph(crates_dir: Path) -> tuple[dict[str, set[str]], dict[str, set[str]]]:
    """Build internal and external dependency graphs."""
    known = discover_crates(crates_dir)
    internal: dict[str, set[str]] = {name: set() for name in known}
    external: dict[str, set[str]] = {name: set() for name in known}
    for name, path in known.items():
        with open(path, "rb") as f:
            data = tomllib.load(f)
        for dep, spec in data.get("dependencies", {}).items():
            if dep in known:
                internal[name].add(dep)
            else:
                external[name].add(dep)
    return internal, external


def check_allowed(internal: dict[str, set[str]]) -> list[str]:
    """Return violation lines for edges not in the allowed-edges table."""
    violations = []
    for src, targets in internal.items():
        allowed, excluded = ALLOWED.get(src, (None, set()))
        for tgt in sorted(targets):
            if allowed is not None:
                if tgt not in allowed:
                    violations.append(f"VIOLATION {src} -> {tgt} (allowed-edges)")
            elif tgt in excluded:
                violations.append(f"VIOLATION {src} -> {tgt} (allowed-edges)")
    return violations


def check_cycles(internal: dict[str, set[str]]) -> list[str]:
    """Return violation lines for dependency cycles."""
    WHITE, GRAY, BLACK = 0, 1, 2
    color = {n: WHITE for n in internal}
    violations: list[str] = []
    path: list[str] = []

    def dfs(node: str) -> None:
        color[node] = GRAY
        path.append(node)
        for nb in internal.get(node, []):
            if color[nb] == GRAY:
                cycle = path[path.index(nb):] + [nb]
                violations.append(
                    f"VIOLATION {' -> '.join(cycle)} (cycle)"
                )
            elif color[nb] == WHITE:
                dfs(nb)
        path.pop()
        color[node] = BLACK

    for node in sorted(internal):
        if color[node] == WHITE:
            dfs(node)
    return violations


def check_godot(external: dict[str, set[str]]) -> list[str]:
    """Return violation lines if a non-crpg-godot crate depends on godot."""
    violations = []
    for src, deps in external.items():
        if "godot" in deps and src != "crpg-godot":
            violations.append(f"VIOLATION {src} -> godot (godot-only)")
    return violations


def main() -> int:
    crates_dir = Path(__file__).resolve().parents[2] / "crates"
    if not crates_dir.is_dir():
        print(f"ERROR: {crates_dir} not found", file=sys.stderr)
        return 2
    internal, external = build_graph(crates_dir)
    violations = (
        check_godot(external)
        + check_cycles(internal)
        + check_allowed(internal)
    )
    for v in violations:
        print(v)
    return 1 if violations else 0


if __name__ == "__main__":
    raise SystemExit(main())
