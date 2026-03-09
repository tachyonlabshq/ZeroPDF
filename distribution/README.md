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
