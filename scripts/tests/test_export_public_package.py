from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path


class ExportPublicPackageTests(unittest.TestCase):
    def test_export_public_package_renders_templates_and_copies_existing_artifacts(self) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as tmp:
            output_dir = Path(tmp) / "public"
            result = subprocess.run(
                [
                    "python3",
                    str(repo_root / "scripts/export_public_package.py"),
                    "--repo-root",
                    str(repo_root),
                    "--target-dir",
                    str(output_dir),
                    "--allow-missing-platforms",
                ],
                cwd=repo_root,
                check=True,
                capture_output=True,
                text=True,
            )

            payload = json.loads(result.stdout)
            self.assertEqual(payload["status"], "success")
            self.assertTrue((output_dir / "README.md").exists())
            self.assertTrue((output_dir / "SKILL.md").exists())
            self.assertTrue((output_dir / "mcp.json").exists())
            self.assertIn("2026.03", (output_dir / "SKILL.md").read_text(encoding="utf-8"))
            self.assertIn("1.0.0", (output_dir / "README.md").read_text(encoding="utf-8"))
            self.assertTrue((output_dir / "export-report.json").exists())


if __name__ == "__main__":
    unittest.main()
