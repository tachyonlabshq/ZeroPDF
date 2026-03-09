from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path


class BuildPublicPackageReleaseTests(unittest.TestCase):
    def test_build_public_package_release_creates_archives(self) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as tmp:
            output_root = Path(tmp) / "out"
            result = subprocess.run(
                [
                    "python3",
                    str(repo_root / "scripts/build_public_package_release.py"),
                    "--repo-root",
                    str(repo_root),
                    "--output-root",
                    str(output_root),
                    "--allow-missing-platforms",
                ],
                cwd=repo_root,
                check=True,
                capture_output=True,
                text=True,
            )

            payload = json.loads(result.stdout)
            self.assertEqual(payload["status"], "success")
            for artifact in payload["artifacts"]:
                self.assertTrue(Path(artifact).exists(), artifact)


if __name__ == "__main__":
    unittest.main()
