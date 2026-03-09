from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
import zipfile
from pathlib import Path


class BuildPlatformBundleTests(unittest.TestCase):
    def test_build_platform_bundle_creates_extractable_zip(self) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            fake_binary = tmp_path / "zeropdf"
            fake_binary.write_bytes(b"binary")
            output_root = tmp_path / "out"
            result = subprocess.run(
                [
                    "python3",
                    str(repo_root / "scripts/build_platform_bundle.py"),
                    "--repo-root",
                    str(repo_root),
                    "--platform",
                    "macos-arm64",
                    "--binary-path",
                    str(fake_binary),
                    "--output-root",
                    str(output_root),
                ],
                cwd=repo_root,
                check=True,
                capture_output=True,
                text=True,
            )

            payload = json.loads(result.stdout)
            zip_path = Path(payload["zip_path"])
            self.assertTrue(zip_path.exists())
            self.assertTrue(Path(payload["checksum_path"]).exists())
            self.assertTrue(Path(str(zip_path).replace(".zip", ".manifest.json")).exists())

            with zipfile.ZipFile(zip_path) as archive:
                names = set(archive.namelist())
            self.assertIn("ZeroPDF/README.md", names)
            self.assertIn("ZeroPDF/SKILL.md", names)
            self.assertIn("ZeroPDF/mcp.json", names)
            self.assertIn("ZeroPDF/bin/zeropdf", names)


if __name__ == "__main__":
    unittest.main()
