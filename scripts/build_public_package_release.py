#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import tarfile
import tempfile
import zipfile
from datetime import datetime, timezone
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build install-ready public package archives from the main ZeroPDF repo")
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--output-root", default="distribution/public-package-releases")
    parser.add_argument("--allow-missing-platforms", action="store_true")
    return parser.parse_args()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_schema_values(schema_rs: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in schema_rs.read_text(encoding="utf-8").splitlines():
        text = line.strip()
        for key in [
            "TOOL_SCHEMA_VERSION",
            "MIN_COMPATIBLE_TOOL_SCHEMA_VERSION",
            "AGENT_TASK_STATE_VERSION",
            "SKILL_API_CONTRACT_VERSION",
        ]:
            marker = f"pub const {key}:"
            if text.startswith(marker):
                values[key] = text.split('"')[1]
    return values


def main() -> int:
    args = parse_args()
    repo_root = Path(args.repo_root).resolve()
    output_root = (repo_root / args.output_root).resolve()
    output_root.mkdir(parents=True, exist_ok=True)
    schema_values = load_schema_values(repo_root / "src" / "schema.rs")

    stamp = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
    bundle_name = f"zeropdf-public-package-{schema_values['TOOL_SCHEMA_VERSION']}-{stamp}"

    with tempfile.TemporaryDirectory(prefix="zeropdf_public_package_") as tmp:
        stage_root = Path(tmp) / bundle_name
        subprocess.run(
            [
                "python3",
                str(repo_root / "scripts/export_public_package.py"),
                "--repo-root",
                str(repo_root),
                "--target-dir",
                str(stage_root),
            ] + (["--allow-missing-platforms"] if args.allow_missing_platforms else []),
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )

        tar_path = output_root / f"{bundle_name}.tar.gz"
        zip_path = output_root / f"{bundle_name}.zip"
        with tarfile.open(tar_path, "w:gz") as tar:
            tar.add(stage_root, arcname=bundle_name)
        with zipfile.ZipFile(zip_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for path in stage_root.rglob("*"):
                if path.is_file():
                    archive.write(path, arcname=f"{bundle_name}/{path.relative_to(stage_root)}")

    checksums_path = output_root / f"{bundle_name}.SHA256SUMS.txt"
    checksums_path.write_text(
        f"{sha256_file(tar_path)}  {tar_path.name}\n{sha256_file(zip_path)}  {zip_path.name}\n",
        encoding="utf-8",
    )

    manifest = {
        "status": "success",
        "bundle_name": bundle_name,
        "tool_schema_version": schema_values["TOOL_SCHEMA_VERSION"],
        "contract_version": schema_values["SKILL_API_CONTRACT_VERSION"],
        "artifacts": [str(tar_path), str(zip_path), str(checksums_path)],
    }
    manifest_path = output_root / f"{bundle_name}.manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(manifest, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
