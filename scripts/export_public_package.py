#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Export the public ZeroPDF package from the main repo")
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--target-dir", required=True)
    parser.add_argument("--macos-arm64-bin", default="target/release/zeropdf")
    parser.add_argument("--macos-x64-bin", default="target/x86_64-apple-darwin/release/zeropdf")
    parser.add_argument("--windows-x64-bin", default="target/x86_64-pc-windows-gnu/release/zeropdf.exe")
    parser.add_argument("--windows-arm64-bin", default="target/aarch64-pc-windows-gnullvm/release/zeropdf.exe")
    parser.add_argument("--allow-missing-platforms", action="store_true")
    return parser.parse_args()


def load_schema_values(schema_rs: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    markers = {
        "TOOL_SCHEMA_VERSION": "TOOL_SCHEMA_VERSION",
        "MIN_COMPATIBLE_VERSION": "MIN_COMPATIBLE_TOOL_SCHEMA_VERSION",
        "TASK_STATE_VERSION": "AGENT_TASK_STATE_VERSION",
        "CONTRACT_VERSION": "SKILL_API_CONTRACT_VERSION",
    }
    for line in schema_rs.read_text(encoding="utf-8").splitlines():
        text = line.strip()
        for out_key, marker in markers.items():
            if text.startswith(f"pub const {marker}:"):
                values[out_key] = text.split('"')[1]
    missing = [key for key in markers if key not in values]
    if missing:
        raise ValueError(f"missing schema constants: {', '.join(missing)}")
    return values


def render_template(template_path: Path, substitutions: dict[str, str]) -> str:
    content = template_path.read_text(encoding="utf-8")
    for key, value in substitutions.items():
        content = content.replace(f"{{{{{key}}}}}", value)
    return content


def copy_file(src: Path, dest: Path) -> None:
    if src.resolve() == dest.resolve():
        return
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest)


def maybe_copy_binary(src: Path, dest: Path) -> bool:
    if not src.exists():
        return False
    copy_file(src, dest)
    return True


def main() -> int:
    args = parse_args()
    repo_root = Path(args.repo_root).resolve()
    target_dir = Path(args.target_dir).resolve()
    target_dir.mkdir(parents=True, exist_ok=True)

    schema_values = load_schema_values(repo_root / "src" / "schema.rs")
    substitutions = dict(schema_values)

    (target_dir / "README.md").write_text(
        render_template(repo_root / "distribution/public-package/README.template.md", substitutions),
        encoding="utf-8",
    )
    (target_dir / "SKILL.md").write_text(
        render_template(repo_root / "distribution/public-package/SKILL.template.md", substitutions),
        encoding="utf-8",
    )
    copy_file(repo_root / "distribution/public-package/mcp.json", target_dir / "mcp.json")

    binary_map = {
        "zeropdf-macos-arm64": (repo_root / args.macos_arm64_bin).resolve(),
        "zeropdf-macos-x64": (repo_root / args.macos_x64_bin).resolve(),
        "zeropdf-windows-x64.exe": (repo_root / args.windows_x64_bin).resolve(),
        "zeropdf-windows-arm64.exe": (repo_root / args.windows_arm64_bin).resolve(),
    }
    copied = {}
    missing = []
    for name, src in binary_map.items():
        updated = maybe_copy_binary(src, target_dir / "bin" / name)
        copied[name] = updated
        if not updated:
            missing.append(name)

    if missing and not args.allow_missing_platforms:
        raise FileNotFoundError(
            "missing platform binaries: " + ", ".join(missing)
        )

    gitignore = target_dir / ".gitignore"
    gitignore.write_text("__pycache__/\n*.pyc\n.DS_Store\n.zeropdf/\n", encoding="utf-8")

    report = {
        "status": "success",
        "tool_schema_version": schema_values["TOOL_SCHEMA_VERSION"],
        "contract_version": schema_values["CONTRACT_VERSION"],
        "target_dir": str(target_dir),
        "copied_binaries": copied,
        "missing_binaries": missing,
    }
    (target_dir / "export-report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
