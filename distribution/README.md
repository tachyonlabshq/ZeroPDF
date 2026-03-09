# Distribution Assets

`ZeroPDF` is the canonical source of truth for installable assets.

## Public Package Export

Export an install-ready public package layout into a target directory:

```bash
python3 scripts/export_public_package.py \
  --target-dir distribution/public-package
```

## Public Package Release Archives

Build zip/tar archives directly from the main repo:

```bash
python3 scripts/build_public_package_release.py \
  --output-root distribution/public-package-releases
```

Outputs:

- `*.tar.gz`
- `*.zip`
- `*.SHA256SUMS.txt`
- `*.manifest.json`

## Platform-Specific Skill Zips

Build a single zip that already contains the final `ZeroPDF/` folder for one platform:

```bash
python3 scripts/build_platform_bundle.py \
  --platform macos-arm64 \
  --binary-path target/release/zeropdf \
  --output-root distribution/platform-bundles
```

Each zip contains:

- `ZeroPDF/README.md`
- `ZeroPDF/SKILL.md`
- `ZeroPDF/mcp.json`
- `ZeroPDF/bin/zeropdf` or `ZeroPDF/bin/zeropdf.exe`

The GitHub Actions workflow for this path is:

- [platform-bundles.yml](/Users/michaelwong/Developer/ZeroPDF/.github/workflows/platform-bundles.yml)
