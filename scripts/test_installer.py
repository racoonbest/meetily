"""Exercise installer failures and updates using real signed macOS fixture apps.

Network responses and app launching are isolated; user applications are never used.
Run on macOS: python3 -m unittest discover -s scripts -p 'test_installer.py' -v
"""

import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
import unittest


INSTALLER = Path(__file__).resolve().parents[1] / "install.sh"
ASSET = "Memo-macos-arm64.zip"


@unittest.skipUnless(sys.platform == "darwin", "requires macOS ditto/codesign")
class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="memo-installer-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.downloads = self.root / "downloads"
        self.downloads.mkdir()
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.target = self.root / "Alternate Applications"
        self.env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}",
                        FIXTURES=str(self.downloads), TEST_ROOT=str(self.root))
        # No network is used. Reject unpinned or unexpected asset URLs.
        self.tool("curl", f"""#!{sys.executable}
import os, pathlib, shutil, sys
args = sys.argv[1:]
url = next(arg for arg in args if arg.startswith('https://'))
if url == 'https://api.github.com/repos/racoonbest/memo/releases/latest':
    name = 'release.json'
elif url.startswith('https://github.com/racoonbest/memo/releases/download/memo-v0.4.1/'):
    name = url.rsplit('/', 1)[1]
else:
    sys.exit(22)
source = pathlib.Path(os.environ['FIXTURES']) / name
if not source.exists(): sys.exit(22)
shutil.copyfile(source, args[args.index('-o') + 1])
""")
        self.tool("open", '#!/bin/bash\ntouch "$TEST_ROOT/launched"\n')
        self.tool("pgrep", "#!/bin/bash\nexit 1\n")
        self.tool("uname", '#!/bin/bash\nif [[ "$1" == -s ]]; then echo Darwin; else echo arm64; fi\n')
        self.tool("sw_vers", "#!/bin/bash\necho 14.4\n")
        (self.downloads / "release.json").write_text(json.dumps({"tag_name": "memo-v0.4.1"}))
        self.app = self.root / "source" / "Memo.app"
        self.make_app(self.app)
        self.archive()

    def tool(self, name, text):
        path = self.bin / name
        path.write_text(text)
        path.chmod(0o755)

    def make_app(self, path, version="0.4.1", identity="com.racoonbest.memo"):
        contents = path / "Contents"
        (contents / "MacOS").mkdir(parents=True)
        shutil.copyfile("/usr/bin/true", contents / "MacOS" / "memo")
        (contents / "MacOS" / "memo").chmod(0o755)
        (contents / "Info.plist").write_bytes(plistlib.dumps({
            "CFBundleIdentifier": identity, "CFBundleExecutable": "memo",
            "CFBundleName": "Memo", "CFBundlePackageType": "APPL",
            "CFBundleVersion": version, "CFBundleShortVersionString": version,
        }))
        subprocess.run(["codesign", "--force", "--sign", "-", str(path)],
                       check=True, capture_output=True)

    def archive(self):
        archive = self.downloads / ASSET
        archive.unlink(missing_ok=True)
        subprocess.run(["ditto", "-c", "-k", "--keepParent", str(self.app), str(archive)], check=True)
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        (self.downloads / f"{ASSET}.sha256").write_text(f"{digest}  {ASSET}\n")

    def run_installer(self, *args, success=True):
        result = subprocess.run(["/bin/bash", str(INSTALLER), "--dir", str(self.target), *args],
                                env=self.env, capture_output=True, text=True)
        self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
        self.assertFalse((self.target / ".memo-install.lock").exists())
        self.assertEqual(list(self.target.glob(".memo-stage.*")), [])
        return result

    def test_install_and_launch_with_spaces_in_destination(self):
        self.run_installer()
        self.assertTrue((self.target / "Memo.app/Contents/MacOS/memo").exists())
        self.assertTrue((self.root / "launched").exists())

    def test_check_does_not_install_or_launch(self):
        self.run_installer("--check")
        self.assertFalse(self.target.exists())
        self.assertFalse((self.root / "launched").exists())

    def test_update_preserves_previous_app(self):
        self.make_app(self.target / "Memo.app", version="0.4.0")
        self.run_installer("--no-open")
        backups = list(self.target.glob(".memo-backups/*/Memo.app/Contents/Info.plist"))
        self.assertEqual(len(backups), 1)
        self.assertEqual(plistlib.loads(backups[0].read_bytes())["CFBundleShortVersionString"], "0.4.0")
        self.assertFalse((self.root / "launched").exists())

    def test_failed_move_restores_previous_app(self):
        self.make_app(self.target / "Memo.app", version="0.4.0")
        self.tool("mv", '#!/bin/bash\n[[ "$1" == *".memo-stage."* ]] && exit 1\nexec /bin/mv "$@"\n')
        self.run_installer("--no-open", success=False)
        info = plistlib.loads((self.target / "Memo.app/Contents/Info.plist").read_bytes())
        self.assertEqual(info["CFBundleShortVersionString"], "0.4.0")

    def test_bad_checksum_leaves_existing_app_untouched(self):
        self.make_app(self.target / "Memo.app", version="0.4.0")
        (self.downloads / f"{ASSET}.sha256").write_text(f"{'0' * 64}  {ASSET}\n")
        self.run_installer(success=False)
        self.assertFalse((self.target / ".memo-backups").exists())
        info = plistlib.loads((self.target / "Memo.app/Contents/Info.plist").read_bytes())
        self.assertEqual(info["CFBundleShortVersionString"], "0.4.0")

    def test_missing_download_fails_before_install(self):
        (self.downloads / ASSET).unlink()
        self.run_installer(success=False)
        self.assertFalse(self.target.exists())

    def test_invalid_signature_is_rejected(self):
        plist = self.app / "Contents/Info.plist"
        info = plistlib.loads(plist.read_bytes())
        info["CFBundleShortVersionString"] = "0.4.2"
        plist.write_bytes(plistlib.dumps(info))
        self.archive()
        result = self.run_installer(success=False)
        self.assertIn("signature is invalid", result.stderr)
        self.assertFalse(self.target.exists())

    def test_unrelated_destination_is_preserved(self):
        self.make_app(self.target / "Memo.app", identity="org.example.unrelated")
        self.run_installer(success=False)
        info = plistlib.loads((self.target / "Memo.app/Contents/Info.plist").read_bytes())
        self.assertEqual(info["CFBundleIdentifier"], "org.example.unrelated")

    def test_symlink_destination_is_preserved(self):
        self.target.mkdir()
        (self.target / "Memo.app").symlink_to(self.app)
        self.run_installer(success=False)
        self.assertTrue((self.target / "Memo.app").is_symlink())

    def test_running_app_is_not_replaced(self):
        self.tool("pgrep", "#!/bin/bash\nexit 0\n")
        self.run_installer(success=False)
        self.assertFalse(self.target.exists())

    def test_old_macos_is_rejected(self):
        self.tool("sw_vers", "#!/bin/bash\necho 14.3\n")
        self.run_installer(success=False)
        self.assertFalse(self.target.exists())

    def test_intel_is_rejected(self):
        self.tool("uname", '#!/bin/bash\nif [[ "$1" == -s ]]; then echo Darwin; else echo x86_64; fi\n')
        self.tool("sysctl", "#!/bin/bash\necho 0\n")
        self.run_installer(success=False)
        self.assertFalse(self.target.exists())

    def test_rosetta_on_apple_silicon_is_supported(self):
        self.tool("uname", '#!/bin/bash\nif [[ "$1" == -s ]]; then echo Darwin; else echo x86_64; fi\n')
        self.tool("sysctl", "#!/bin/bash\necho 1\n")
        self.run_installer("--check")

    def test_linux_is_rejected(self):
        self.tool("uname", "#!/bin/bash\necho Linux\n")
        self.run_installer(success=False)
        self.assertFalse(self.target.exists())


if __name__ == "__main__":
    unittest.main()
