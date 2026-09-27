#!/usr/bin/env python3
"""Local contract tests for packaging/homebrew/render_formula.py."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


HERE = Path(__file__).resolve().parent
RENDERER = HERE / "render_formula.py"
TEMPLATE = HERE / "pdftract.rb.template"
VERSION = "1.2.3"
SHA256 = "0123456789abcdef" * 4


def release_inputs() -> dict[str, str]:
    return {
        "RELEASE_TAG": f"v{VERSION}",
        "VERSION": VERSION,
        "SOURCE_ARCHIVE_URL": (
            f"https://github.com/jedarden/pdftract/archive/refs/tags/v{VERSION}.tar.gz"
        ),
        "SOURCE_ARCHIVE_SHA256": SHA256,
        "SHA256SUMS_URL": (
            f"https://github.com/jedarden/pdftract/releases/download/v{VERSION}/SHA256SUMS"
        ),
        "SHA256SUMS_SIG_URL": (
            f"https://github.com/jedarden/pdftract/releases/download/v{VERSION}/SHA256SUMS.sig"
        ),
        "SHA256SUMS_PEM_URL": (
            f"https://github.com/jedarden/pdftract/releases/download/v{VERSION}/SHA256SUMS.pem"
        ),
    }


class RenderFormulaContractTests(unittest.TestCase):
    def run_renderer(self, *extra: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(RENDERER), "--template", str(TEMPLATE), *extra],
            text=True,
            capture_output=True,
        )

    def test_json_tuple_renders_archive_and_checksum_deterministically(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            inputs_path = Path(temp_dir) / "release.json"
            output_path = Path(temp_dir) / "Formula" / "pdftract.rb"
            second_output_path = Path(temp_dir) / "Formula" / "pdftract-second.rb"
            inputs_path.write_text(json.dumps(release_inputs()), encoding="utf-8")

            result = self.run_renderer("--inputs", str(inputs_path), "--output", str(output_path))
            second_result = self.run_renderer(
                "--inputs", str(inputs_path), "--output", str(second_output_path)
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(second_result.returncode, 0, second_result.stderr)
            formula = output_path.read_text(encoding="utf-8")
            self.assertEqual(formula, second_output_path.read_text(encoding="utf-8"))
            self.assertNotIn("<VERSION>", formula)
            self.assertNotIn("<SHA256>", formula)
            self.assertIn(f'url "{release_inputs()["SOURCE_ARCHIVE_URL"]}"', formula)
            self.assertIn(f'version "{VERSION}"', formula)
            self.assertIn(f'sha256 "{SHA256}"', formula)
            self.assertIn('shell_output("#{bin}/pdftract --version")', formula)

    def test_dry_run_handoff_renders_exact_archive_checksum_before_publish(self) -> None:
        """Exercise the local equivalent of the release-to-render handoff.

        The release leg hashes the canonical archive once and records that
        digest in SHA256SUMS. A dry run must pass that handoff to the
        renderer, which only writes a local Formula/pdftract.rb and performs
        no tap operation.
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            archive = root / "source" / f"pdftract-v{VERSION}.tar.gz"
            archive.parent.mkdir()
            archive.write_bytes(b"deterministic source archive fixture\n")
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            sums = root / "SHA256SUMS"
            sums.write_text(
                f"{digest}  source/pdftract-v{VERSION}.tar.gz\n",
                encoding="utf-8",
            )

            handoff = next(
                line.split()[0]
                for line in sums.read_text(encoding="utf-8").splitlines()
                if line.endswith(f"source/pdftract-v{VERSION}.tar.gz")
            )
            inputs = release_inputs()
            inputs["SOURCE_ARCHIVE_SHA256"] = handoff
            inputs_path = root / "release-inputs.json"
            inputs_path.write_text(json.dumps(inputs), encoding="utf-8")
            output_path = root / "Formula" / "pdftract.rb"

            result = self.run_renderer(
                "--inputs", str(inputs_path), "--output", str(output_path)
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            formula = output_path.read_text(encoding="utf-8")
            self.assertIn(f'url "{inputs["SOURCE_ARCHIVE_URL"]}"', formula)
            self.assertIn(f'sha256 "{digest}"', formula)
            self.assertEqual(handoff, digest)
            self.assertNotIn("push", (result.stdout + result.stderr).lower())

    def test_failed_render_is_nonzero_and_writes_no_formula(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            inputs_path = root / "release.json"
            template_path = root / "invalid.rb.template"
            output_path = root / "Formula" / "pdftract.rb"
            inputs_path.write_text(json.dumps(release_inputs()), encoding="utf-8")
            template_path.write_text('version "<VERSION>"\n', encoding="utf-8")

            result = subprocess.run(
                [
                    sys.executable,
                    str(RENDERER),
                    "--template",
                    str(template_path),
                    "--inputs",
                    str(inputs_path),
                    "--output",
                    str(output_path),
                ],
                text=True,
                capture_output=True,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("<SHA256>", result.stderr)
            self.assertFalse(output_path.exists())

    def test_rejects_mismatched_archive_url(self) -> None:
        inputs = release_inputs()
        inputs["SOURCE_ARCHIVE_URL"] = (
            "https://github.com/jedarden/pdftract/archive/refs/heads/main.tar.gz"
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            inputs_path = Path(temp_dir) / "release.json"
            inputs_path.write_text(json.dumps(inputs), encoding="utf-8")
            result = self.run_renderer("--inputs", str(inputs_path))

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SOURCE_ARCHIVE_URL", result.stderr)

    def test_rejects_prerelease_tag(self) -> None:
        inputs = release_inputs()
        inputs["RELEASE_TAG"] = "v1.2.3-rc.1"
        inputs["VERSION"] = "1.2.3-rc.1"
        with tempfile.TemporaryDirectory() as temp_dir:
            inputs_path = Path(temp_dir) / "release.json"
            inputs_path.write_text(json.dumps(inputs), encoding="utf-8")
            result = self.run_renderer("--inputs", str(inputs_path))

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("RELEASE_TAG", result.stderr)

    def test_rejects_incomplete_template(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            inputs_path = Path(temp_dir) / "release.json"
            template_path = Path(temp_dir) / "incomplete.rb.template"
            inputs_path.write_text(json.dumps(release_inputs()), encoding="utf-8")
            template_path.write_text('version "<VERSION>"\n', encoding="utf-8")
            result = subprocess.run(
                [
                    sys.executable,
                    str(RENDERER),
                    "--template",
                    str(template_path),
                    "--inputs",
                    str(inputs_path),
                ],
                text=True,
                capture_output=True,
            )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("<SHA256>", result.stderr)


if __name__ == "__main__":
    unittest.main()
