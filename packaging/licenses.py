"""Collect redistributed dependency notices from Cargo.lock's fetched crates."""
import json
from pathlib import Path
import subprocess

metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1"]))
sections = ["Third-party dependency notices for doc-search-lite.\nIncludes build and platform-specific dependencies.\n"]
missing = []
for package in sorted(metadata["packages"], key=lambda p: p["name"]):
    if package["name"] == "doc-search-lite":
        continue
    root = Path(package["manifest_path"]).parent
    files = sorted(p for p in root.iterdir() if p.is_file() and p.name.lower().startswith(("license", "licence", "copying", "notice")))
    if package.get("license_file"):
        files = sorted(set(files + [root / package["license_file"]]))
    sections.append(f"\n{'='*72}\n{package['name']} {package['version']} — {package.get('license')}\n{package.get('repository') or ''}\n")
    if not files:
        # These upstream crates declare MIT in Cargo.toml but ship no license file.
        if package["name"] in {"adobe-cmap-parser", "pdf-extract", "type1-encoding-parser", "r-efi"}:
            sections.append("License declaration: " + package["license"] + "\n")
            sections.append("Package authors: " + ", ".join(package["authors"]) + "\n")
            sections.append(Path("LICENSE").read_text().split("Permission is hereby", 1)[0].split("Copyright", 1)[0])
            sections.append("Permission is hereby" + Path("LICENSE").read_text().split("Permission is hereby", 1)[1])
        else:
            missing.append(package["name"])
    for path in files:
        sections.append(f"\n--- {path.name} ---\n{path.read_text(encoding='utf-8', errors='replace')}\n")
Path("THIRD_PARTY_LICENSES.txt").write_text("".join(sections), encoding="utf-8")
if missing:
    raise SystemExit("Missing bundled license text; inspect upstream: " + ", ".join(missing))
