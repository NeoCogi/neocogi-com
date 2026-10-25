"""Resolve the exact locked SDK graphs used by CI and independent fixtures."""

import json
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest


ROOT = Path(__file__).resolve().parents[1]


class WorkspaceResolutionTests(unittest.TestCase):
    """A version edit must leave every locked local consumer resolvable."""

    def metadata(self, manifest: Path) -> dict:
        """Ask Cargo to resolve real manifests without changing their lockfiles."""
        result = subprocess.run(
            ["cargo", "metadata", "--locked", "--offline", "--format-version", "1",
             "--manifest-path", str(manifest)],
            cwd=ROOT, capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, f"{manifest}:\n{result.stderr}")
        return json.loads(result.stdout)

    def test_workspace_resolves_matching_local_package_versions(self):
        """Published members must resolve to the root's chosen SDK version."""
        manifest = ROOT / "Cargo.toml"
        version = tomllib.loads(manifest.read_text())["workspace"]["package"]["version"]
        metadata = self.metadata(manifest)
        members = set(metadata["workspace_members"])
        published = [package for package in metadata["packages"]
                     if package["id"] in members and package["publish"] != []]
        self.assertTrue(published, "the workspace must contain publishable COM packages")
        for package in published:
            with self.subTest(package=package["name"]):
                self.assertEqual(package["version"], version)

    def test_independent_fixture_lockfiles_resolve(self):
        """Host/plugin and native fixtures must work with their committed graphs."""
        fixtures = ROOT / "tests/com/integration-tests/fixtures"
        lockfiles = sorted(fixtures.glob("*/Cargo.lock"))
        lockfiles.append(ROOT / "tests/abi-spec/Cargo.lock")
        for lockfile in lockfiles:
            with self.subTest(fixture=lockfile.parent.relative_to(ROOT)):
                self.metadata(lockfile.with_name("Cargo.toml"))

    def test_no_default_consumer_has_no_compiler_in_its_build_graph(self):
        """Use downstream selection, avoiding workspace/dev feature unification."""
        with tempfile.TemporaryDirectory(prefix="neocogi-no-default-") as directory:
            project = Path(directory)
            (project / "src").mkdir()
            (project / "Cargo.toml").write_text(
                '[package]\nname="no-default-proof"\nversion="0.0.0"\nedition="2024"\n[workspace]\n'
                f'[dependencies]\nneocogi-com={{path={json.dumps(str(ROOT / "crates/api"))},default-features=false}}\n')
            (project / "src/lib.rs").write_text(
                '#![no_std]\npub fn identity() -> neocogi_com::NcGuid { neocogi_com::IID_IUNKNOWN }\n')
            base = ["cargo", "--offline", "--manifest-path", str(project / "Cargo.toml")]
            tree = subprocess.run([base[0], "tree", *base[1:], "--edges", "normal,build", "--prefix", "none"],
                                  cwd=project, capture_output=True, text=True)
            self.assertEqual(tree.returncode, 0, tree.stderr)
            for compiler in ("neocogi-idl", "syn ", "prettyplease", "serde_json"):
                self.assertNotIn(compiler, tree.stdout)
            checked = subprocess.run([base[0], "check", *base[1:], "--target-dir", str(ROOT / "target/no-default-consumer"), "--jobs", "1"],
                                     cwd=project, capture_output=True, text=True)
            self.assertEqual(checked.returncode, 0, checked.stderr)


if __name__ == "__main__":
    unittest.main()
