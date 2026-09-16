"""Build and smoke-test the exact portable ZIP published in Releases."""
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile
import tomllib
import zipfile

root = Path(__file__).resolve().parents[1]
os.chdir(root)
version = tomllib.loads(Path("Cargo.toml").read_text())["package"]["version"]
system = "windows" if sys.platform == "win32" else "macos"
arch = "arm64" if platform.machine().lower() in ("aarch64", "arm64") else "x64"
executable = "doc-search-lite.exe" if system == "windows" else "doc-search-lite"
binary = root / "target/release" / executable
destination = root / "dist" / f"doc-search-lite-{version}-{system}-{arch}.zip"
destination.parent.mkdir(exist_ok=True)
files = {executable: binary, "SKILL.md": root / "skills/doc-search-lite/SKILL.md",
         "LICENSE": root / "LICENSE", "THIRD_PARTY_LICENSES.txt": root / "THIRD_PARTY_LICENSES.txt"}
with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
    for name, source in files.items():
        entry = zipfile.ZipInfo(f"doc-search-lite/{name}")
        entry.create_system = 3
        entry.external_attr = (0o100755 if name == executable else 0o100644) << 16
        entry.compress_type = zipfile.ZIP_DEFLATED
        archive.writestr(entry, source.read_bytes())
assert destination.stat().st_size < 5 * 1024 * 1024, "portable ZIP exceeded 5 MiB budget"
with tempfile.TemporaryDirectory(prefix="doc-search-lite-release-") as temp:
    with zipfile.ZipFile(destination) as archive:
        archive.extractall(temp)
        for entry in archive.infolist():
            (Path(temp) / entry.filename).chmod(entry.external_attr >> 16)
    unpacked = Path(temp) / "doc-search-lite" / executable
    # Test the portable default path independently of an explicitly selected database.
    subprocess.run([str(unpacked), "status"], check=True, env=dict(os.environ, PATH=""))
    assert (unpacked.parent / "data/index.db").is_file()
    subprocess.run([sys.executable, "tests/smoke.py", str(unpacked)], check=True)
print(f"ZIP: {destination.name}: {destination.stat().st_size:,} bytes; executable: {binary.stat().st_size:,} bytes")
