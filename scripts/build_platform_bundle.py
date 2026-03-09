#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import tempfile
import zipfile
from pathlib import Path

PLATFORMS = {
    "macos-arm64": {
        "label": "macOS Apple Silicon",
        "binary_name": "zeropdf",
        "default_binary": "target/release/zeropdf",
    },
    "macos-x64": {
        "label": "macOS Intel",
        "binary_name": "zeropdf",
        "default_binary": "target/x86_64-apple-darwin/release/zeropdf",
    },
    "windows-x64": {
        "label": "Windows x64",
        "binary_name": "zeropdf.exe",
        "default_binary": "target/x86_64-pc-windows-gnu/release/zeropdf.exe",
    },
    "windows-arm64": {
        "label": "Windows ARM64",
        "binary_name": "zeropdf.exe",
        "default_binary": "target/aarch64-pc-windows-gnullvm/release/zeropdf.exe",
    },
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build a platform-specific ZeroPDF zip bundle for extract-and-drop installation"
    )
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--platform", choices=sorted(PLATFORMS), required=True)
    parser.add_argument("--binary-path", default=None)
    parser.add_argument("--output-root", default="distribution/platform-bundles")
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


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    args = parse_args()
    repo_root = Path(args.repo_root).resolve()
    platform = PLATFORMS[args.platform]
    binary_path = (repo_root / (args.binary_path or platform["default_binary"])).resolve()
    if not binary_path.exists():
        raise FileNotFoundError(f"missing binary for {args.platform}: {binary_path}")

    output_root = (repo_root / args.output_root).resolve()
    output_root.mkdir(parents=True, exist_ok=True)

    schema_values = load_schema_values(repo_root / "src" / "schema.rs")
    substitutions = {
        **schema_values,
        "PLATFORM_LABEL": platform["label"],
        "BINARY_NAME": platform["binary_name"],
    }

    bundle_root_name = "ZeroPDF"
    zip_name = f"ZeroPDF-{args.platform}-{schema_values['TOOL_SCHEMA_VERSION']}.zip"
    zip_path = output_root / zip_name
    manifest_path = output_root / f"ZeroPDF-{args.platform}-{schema_values['TOOL_SCHEMA_VERSION']}.manifest.json"
    checksum_path = output_root / f"ZeroPDF-{args.platform}-{schema_values['TOOL_SCHEMA_VERSION']}.SHA256.txt"

    with tempfile.TemporaryDirectory(prefix=f"zeropdf_{args.platform}_") as tmp:
        stage_root = Path(tmp) / bundle_root_name
        (stage_root / "bin").mkdir(parents=True, exist_ok=True)
        shutil.copy2(binary_path, stage_root / "bin" / platform["binary_name"])
        (stage_root / "README.md").write_text(
            render_template(repo_root / "distribution/templates/platform-package-README.template.md", substitutions),
            encoding="utf-8",
        )
        (stage_root / "SKILL.md").write_text(
            render_template(repo_root / "distribution/public-package/SKILL.template.md", substitutions),
            encoding="utf-8",
        )
        (stage_root / "mcp.json").write_text(
            render_template(repo_root / "distribution/templates/platform-package-mcp.template.json", substitutions),
            encoding="utf-8",
        )
        (stage_root / ".gitignore").write_text("__pycache__/\n*.pyc\n.DS_Store\n.zeropdf/\n", encoding="utf-8")

        with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for path in stage_root.rglob("*"):
                if path.is_file():
                    archive.write(path, arcname=f"{bundle_root_name}/{path.relative_to(stage_root)}")

    manifest = {
        "status": "success",
        "platform": args.platform,
        "platform_label": platform["label"],
        "tool_schema_version": schema_values["TOOL_SCHEMA_VERSION"],
        "contract_version": schema_values["CONTRACT_VERSION"],
        "binary_name": platform["binary_name"],
        "zip_path": str(zip_path),
    }
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    checksum_path.write_text(f"{sha256_file(zip_path)}  {zip_path.name}\n", encoding="utf-8")
    print(json.dumps({**manifest, "checksum_path": str(checksum_path)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
