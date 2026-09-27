#!/usr/bin/env python3
"""Static contract checks for the release-cascade Homebrew handoff."""

from __future__ import annotations

import os
import re
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[2]
HOMEBREW_WORKFLOW = ROOT / ".ci/argo-workflows/pdftract-homebrew-publish.yaml"
RELEASE_WORKFLOW = ROOT / ".ci/argo-workflows/pdftract-github-release.yaml"


class HomebrewWorkflowContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.homebrew = HOMEBREW_WORKFLOW.read_text(encoding="utf-8")
        cls.release = RELEASE_WORKFLOW.read_text(encoding="utf-8")
        cls.gate = cls.homebrew.split("    # === Versioned-Tag Gate ===", 1)[1].split(
            "    # === Verify Release Metadata ===", 1
        )[0]
        cls.render = cls.homebrew.split("    # === Render Formula ===", 1)[1].split(
            "    # === Push to Tap ===", 1
        )[0]
        cls.verify = cls.homebrew.split("    # === Verify Release Metadata ===", 1)[1].split(
            "    # === Render Formula ===", 1
        )[0]
        cls.push = cls.homebrew.split("    # === Push to Tap ===", 1)[1].split(
            "    # === Wait for Mirror ===", 1
        )[0]
        cls.wait = cls.homebrew.split("    # === Wait for Mirror ===", 1)[1].split(
            "    # === Brew Install Verification ===", 1
        )[0]

    def _run_gate(self, tag: str) -> tuple[subprocess.CompletedProcess[str], Path]:
        """Run the checked-in gate shell with an isolated proceed output."""
        with tempfile.TemporaryDirectory() as temp_dir:
            proceed = Path(temp_dir) / "proceed"
            gate_task = self.gate.split("    - name: versioned-tag-gate", 1)[1].split(
                "        resources:", 1
            )[0]
            script = gate_task.split("          - |", 1)[1]
            script = textwrap.dedent(script).replace(
                'TAG="{{workflow.parameters.tag}}"', 'TAG="${GATE_TAG}"'
            )
            script = script.replace("/tmp/proceed", str(proceed))
            result = subprocess.run(
                ["sh", "-c", script],
                env={**os.environ, "GATE_TAG": tag},
                text=True,
                capture_output=True,
            )
            saved_handle = tempfile.NamedTemporaryFile(
                prefix="pdftract-gate-result-", delete=False
            )
            saved = Path(saved_handle.name)
            saved_handle.close()
            if proceed.exists():
                saved.write_text(proceed.read_text(encoding="utf-8"), encoding="utf-8")
            return result, saved

    def test_release_producer_hands_off_signed_source_archive_digest(self) -> None:
        self.assertIn(
            'SOURCE_ARCHIVE="source/pdftract-v${VERSION}.tar.gz"', self.release
        )
        self.assertIn(
            'SOURCE_ARCHIVE_URL="https://github.com/${REPO}/archive/refs/tags/${TAG}.tar.gz"',
            self.release,
        )
        self.assertIn('sha256sum "${SOURCE_ARCHIVE}" >> SHA256SUMS', self.release)

    def test_versioned_cascade_orders_release_handoff_before_tap_publication(self) -> None:
        """The release handoff must complete before Homebrew can publish."""
        release_dag = self.release.split(
            "    - name: release-pipeline\n", 1
        )[1].split("    # === Setup Step ===", 1)[0]
        release_task_positions = {
            name: release_dag.index(f"          - name: {name}\n")
            for name in ("compute-sha256sums", "sign-sums", "gh-release-create")
        }
        self.assertLess(
            release_task_positions["compute-sha256sums"],
            release_task_positions["sign-sums"],
        )
        self.assertLess(
            release_task_positions["sign-sums"],
            release_task_positions["gh-release-create"],
        )
        self.assertIn(
            'from: "{{tasks.compute-sha256sums.outputs.artifacts.sha256sums}}"',
            release_dag,
        )
        self.assertIn(
            'from: "{{tasks.compute-sha256sums.outputs.artifacts.release-archive}}"',
            release_dag,
        )
        self.assertIn(
            'from: "{{tasks.sign-sums.outputs.artifacts.sha256sums-sig}}"',
            release_dag,
        )
        self.assertIn(
            'from: "{{tasks.sign-sums.outputs.artifacts.sha256sums-pem}}"',
            release_dag,
        )

        homebrew_dag = self.homebrew.split(
            "    - name: homebrew-pipeline\n", 1
        )[1].split("    # === Versioned-Tag Gate ===", 1)[0]
        homebrew_task_positions = {
            name: homebrew_dag.index(f"          - name: {name}\n")
            for name in (
                "versioned-tag-gate",
                "verify-release-metadata",
                "render-formula",
                "push-tap",
            )
        }
        self.assertLess(
            homebrew_task_positions["versioned-tag-gate"],
            homebrew_task_positions["verify-release-metadata"],
        )
        self.assertLess(
            homebrew_task_positions["verify-release-metadata"],
            homebrew_task_positions["render-formula"],
        )
        self.assertLess(
            homebrew_task_positions["render-formula"],
            homebrew_task_positions["push-tap"],
        )
        self.assertIn(
            "dependencies: [versioned-tag-gate, verify-release-metadata, render-formula]",
            homebrew_dag,
        )
        for artifact in (
            "release-archive",
            "sha256sums",
            "sha256sums-sig",
            "sha256sums-pem",
        ):
            self.assertIn(f"- name: {artifact}", self.homebrew)
            self.assertIn(
                f'from: "{{{{inputs.artifacts.{artifact}}}}}"', self.homebrew
            )

    def test_homebrew_verifies_supplied_release_artifacts_without_fetching(self) -> None:
        """The cascade handoff, not a release URL, is the verifier's input."""
        self.assertIn(
            "path: /tmp/source/pdftract-v{{workflow.parameters.version}}.tar.gz",
            self.verify,
        )
        self.assertIn("path: /tmp/SHA256SUMS", self.verify)
        self.assertIn("sha256sum -c -", self.verify)
        self.assertIn(
            "release archive does not match its SHA256SUMS handoff", self.verify
        )
        self.assertNotIn("curl", self.verify)
        self.assertNotIn("releases/download", self.verify)

    def test_render_consumes_checksum_handoff_without_archive_fetch_or_rehash(self) -> None:
        self.assertIn("source-archive-sha256", self.homebrew)
        self.assertIn("{{tasks.verify-release-metadata.outputs.parameters.source-archive-sha256}}", self.homebrew)
        self.assertIn('SOURCE_ARCHIVE_SHA256="{{inputs.parameters.source-archive-sha256}}"', self.render)
        self.assertIn(
            "path: /tmp/source/pdftract-v{{workflow.parameters.version}}.tar.gz",
            self.render,
        )
        self.assertIn("path: /tmp/SHA256SUMS", self.render)
        self.assertIn(
            'from: "{{inputs.artifacts.release-archive}}"',
            self.homebrew.split("          - name: render-formula\n", 1)[1].split(
                "          - name: push-tap\n", 1
            )[0],
        )
        self.assertIn(
            'from: "{{inputs.artifacts.sha256sums}}"',
            self.homebrew.split("          - name: render-formula\n", 1)[1].split(
                "          - name: push-tap\n", 1
            )[0],
        )
        self.assertIn("HANDOFF_SHA256=", self.render)
        self.assertIn(
            'ERROR: render archive checksum does not match the verified SHA256SUMS handoff',
            self.render,
        )
        self.assertNotIn("curl -fsSL --retry 3 \"${ARCHIVE_URL}\"", self.render)
        self.assertNotIn("sha256sum /tmp/source.tar.gz", self.render)
        self.assertIn("python3 render_formula.py", self.render)
        self.assertIn("--output /tmp/Formula/pdftract.rb", self.render)

    def test_invalid_handoff_fails_before_renderer_and_push(self) -> None:
        self.assertIn(
            "refusing to render without the signed source-archive checksum handoff",
            self.render,
        )
        self.assertIn(
            'when: "{{tasks.versioned-tag-gate.outputs.parameters.proceed}} == true"',
            self.homebrew,
        )

    def test_dry_run_proves_archive_and_sha256sums_handoff_locally(self) -> None:
        """The dry run must exercise the archive/checksum binding, not fake a digest."""
        self.assertIn(
            'printf \'%s\\n\' "pdftract Homebrew render fixture" > "${SOURCE_ARCHIVE_NAME}"',
            self.verify,
        )
        self.assertIn(
            'sha256sum "${SOURCE_ARCHIVE_NAME}" > SHA256SUMS', self.verify
        )
        self.assertIn("sha256sum -c SHA256SUMS", self.verify)
        self.assertIn('SOURCE_ARCHIVE_SHA256="$(awk -v expected=', self.verify)
        self.assertIn(
            "matching local archive/checksum handoff accepted", self.verify
        )
        self.assertNotIn(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            self.verify,
        )
        self.assertIn(
            'when: "{{tasks.versioned-tag-gate.outputs.parameters.proceed}} == true && {{workflow.parameters.dry_run}} != true"',
            self.homebrew,
        )

    def test_non_versioned_tags_are_successful_noops(self) -> None:
        for tag in ("", "main", "v1.2.3-rc.1", ":latest", "deadbeef" * 5):
            with self.subTest(tag=tag):
                result, proceed_path = self._run_gate(tag)
                try:
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(
                        proceed_path.read_text(encoding="utf-8"), "false\n"
                    )
                finally:
                    proceed_path.unlink(missing_ok=True)

        result, proceed_path = self._run_gate("v1.2.3")
        try:
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(proceed_path.read_text(encoding="utf-8"), "true\n")
        finally:
            proceed_path.unlink(missing_ok=True)

        # The false gate result is a no-op for every publication task, not a
        # path around the gate.
        self.assertEqual(
            self.homebrew.count(
                'when: "{{tasks.versioned-tag-gate.outputs.parameters.proceed}} == true'
            ),
            5,
        )

    def test_approved_tap_urls_are_pinned_and_secrets_are_only_referenced(self) -> None:
        self.assertIn(
            'TAP_URL="https://git.ardenone.com/jedarden/homebrew-tap.git"',
            self.push,
        )
        self.assertIn(
            'MIRROR="https://github.com/jedarden/homebrew-tap.git"', self.homebrew
        )
        self.assertNotIn("releases/latest", self.homebrew)
        self.assertNotIn("archive/refs/heads", self.homebrew)
        for image in re.findall(
            r"^\s+image:\s*(\S+)", self.homebrew, re.MULTILINE
        ):
            self.assertNotRegex(image, r":latest(?:$|\s)")

        # The manifest may name the secret and its environment variable, but
        # must not carry a token-shaped literal or interpolate one into a URL.
        self.assertRegex(self.push, r"name: homebrew-tap-push-token")
        self.assertRegex(
            self.push, r"secretKeyRef:\n\s+name: homebrew-tap-push-token"
        )
        self.assertNotRegex(
            self.homebrew,
            r"(?:ghp_[A-Za-z0-9]+|github_pat_[A-Za-z0-9_]+|glpat-[A-Za-z0-9_-]+|xox[baprs]-[A-Za-z0-9-]+)",
        )
        self.assertNotRegex(self.push, r"TAP_TOKEN\s*=\s*[^$\s\"']")

    def test_render_failure_is_retryable_but_cannot_reach_tap(self) -> None:
        self.assertIn('limit: "1"\n        retryPolicy: OnFailure', self.render)
        self.assertIn("set -eu", self.render)
        self.assertIn(
            "refusing to render without the signed source-archive checksum handoff",
            self.render,
        )
        self.assertIn(
            "dependencies: [versioned-tag-gate, verify-release-metadata, render-formula]",
            self.homebrew,
        )
        self.assertNotIn("git push", self.render)

    def test_failed_tap_update_retries_without_false_success(self) -> None:
        """Execute the tap step with a fake git whose push fails."""
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fake_bin = root / "bin"
            fake_bin.mkdir()
            fake_git = fake_bin / "git"
            fake_git.write_text(
                """#!/bin/sh
case "$1" in
  clone|config|add|commit) exit 0 ;;
  diff)
    [ "$3" = "--name-only" ] && { echo Formula/pdftract.rb; exit 0; }
    exit 1
    ;;
  push) exit 42 ;;
  rev-parse) echo 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef ;;
  *) exit 0 ;;
esac
""",
                encoding="utf-8",
            )
            fake_git.chmod(0o700)

            script = self.push.split("          - |", 1)[1].split(
                "        env:", 1
            )[0]
            script = textwrap.dedent(script)
            script = script.replace("apk add --no-cache git", ":")
            script = script.replace("/tmp/tap-commit", "__TAP_COMMIT_OUTPUT__")
            script = script.replace("/tap", str(root / "tap"))
            script = script.replace("__TAP_COMMIT_OUTPUT__", str(root / "tap-commit"))
            script = script.replace(
                'VERSION="{{workflow.parameters.version}}"', 'VERSION="1.2.3"'
            )
            script = script.replace(
                'FORMULA_B64="{{inputs.parameters.formula-b64}}"',
                'FORMULA_B64="Zm9ybXVsYQ=="',
            )
            result = subprocess.run(
                ["sh", "-c", script],
                env={
                    **os.environ,
                    "PATH": f"{fake_bin}{os.pathsep}{os.environ['PATH']}",
                    "TAP_TOKEN": "fixture-token-not-in-workflow",
                },
                text=True,
                capture_output=True,
            )

            self.assertEqual(result.returncode, 42, result.stderr)
            self.assertNotIn("Pushed", result.stdout + result.stderr)
            self.assertFalse((root / "tap-commit").exists())

        self.assertIn(
            'retryStrategy:\n        limit: "1"\n        retryPolicy: OnFailure', self.push
        )
        self.assertIn("dependencies: [versioned-tag-gate, push-tap]", self.homebrew)
        self.assertIn("GitHub mirror did not reflect tap commit", self.wait)
        self.assertIn("exit 1", self.wait)

    def test_mirror_timeout_reports_failure_instead_of_false_success(self) -> None:
        """A mirror that never advances must exhaust its bounded poll and fail."""
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fake_bin = root / "bin"
            fake_bin.mkdir()
            fake_git = fake_bin / "git"
            fake_git.write_text(
                "#!/bin/sh\n"
                "case \"$1\" in\n"
                "  ls-remote) echo unrelated-commit; exit 0 ;;\n"
                "  *) exit 0 ;;\n"
                "esac\n",
                encoding="utf-8",
            )
            fake_git.chmod(0o700)
            fake_sleep = fake_bin / "sleep"
            fake_sleep.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            fake_sleep.chmod(0o700)

            script = self.wait.split("          - |", 1)[1].split(
                "        resources:", 1
            )[0]
            script = textwrap.dedent(script)
            script = script.replace("apk add --no-cache git", ":")
            script = script.replace(
                'COMMIT="{{inputs.parameters.tap-commit}}"',
                'COMMIT="0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"',
            )
            result = subprocess.run(
                ["sh", "-c", script],
                env={
                    **os.environ,
                    "PATH": f"{fake_bin}{os.pathsep}{os.environ['PATH']}",
                },
                text=True,
                capture_output=True,
            )

            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertIn("GitHub mirror did not reflect tap commit", result.stderr)
            self.assertIn("within 5 minutes", result.stderr)
            self.assertNotIn("Mirror reflects", result.stdout)

    def test_publication_is_pinned_to_the_approved_tap_and_formula_path(self) -> None:
        self.assertIn(
            'TAP_URL="https://git.ardenone.com/jedarden/homebrew-tap.git"',
            self.push,
        )
        self.assertNotIn("{{workflow.parameters.tap-url}}", self.push)
        self.assertNotIn("{{workflow.parameters.tap-repo}}", self.homebrew)
        self.assertIn('MIRROR="https://github.com/jedarden/homebrew-tap.git"', self.homebrew)
        self.assertIn("git add -- Formula/pdftract.rb", self.push)
        self.assertIn(
            'if [ -n "${STAGED_PATHS}" ] && [ "${STAGED_PATHS}" != "Formula/pdftract.rb" ]; then',
            self.push,
        )

    def test_unchanged_formula_is_an_idempotent_success(self) -> None:
        """A retry after a successful push must accept an empty staged path list."""
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            fake_bin = root / "bin"
            fake_bin.mkdir()
            fake_git = fake_bin / "git"
            fake_git.write_text(
                """#!/bin/sh
case "$1" in
  clone|config|add) exit 0 ;;
  diff)
    [ "$3" = "--name-only" ] && exit 0
    exit 0
    ;;
  push) exit 99 ;;
  *) exit 0 ;;
esac
""",
                encoding="utf-8",
            )
            fake_git.chmod(0o700)

            script = self.push.split("          - |", 1)[1].split(
                "        env:", 1
            )[0]
            script = textwrap.dedent(script)
            script = script.replace("apk add --no-cache git", ":")
            script = script.replace("/tmp/tap-commit", "__TAP_COMMIT_OUTPUT__")
            script = script.replace("/tap", str(root / "tap"))
            script = script.replace("__TAP_COMMIT_OUTPUT__", str(root / "tap-commit"))
            script = script.replace(
                'VERSION="{{workflow.parameters.version}}"', 'VERSION="1.2.3"'
            )
            script = script.replace(
                'FORMULA_B64="{{inputs.parameters.formula-b64}}"',
                'FORMULA_B64="Zm9ybXVsYQ=="',
            )
            result = subprocess.run(
                ["sh", "-c", script],
                env={
                    **os.environ,
                    "PATH": f"{fake_bin}{os.pathsep}{os.environ['PATH']}",
                    "TAP_TOKEN": "fixture-token-not-in-workflow",
                },
                text=True,
                capture_output=True,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("idempotent re-run", result.stdout)
            self.assertNotIn("Pushed", result.stdout + result.stderr)
            self.assertTrue((root / "tap-commit").exists())

    def test_publication_waits_for_render_validation_and_uses_bounded_retry(self) -> None:
        self.assertIn(
            "dependencies: [versioned-tag-gate, verify-release-metadata, render-formula]",
            self.homebrew,
        )
        self.assertIn('retryStrategy:\n        limit: "1"\n        retryPolicy: OnFailure', self.push)
        self.assertIn("set -eu", self.push)
        self.assertIn("git push --quiet origin main", self.push)
        self.assertIn('echo "=== Pushed ${COMMIT}', self.push)
        self.assertIn("homebrew-tap-push-token", self.push)
        self.assertIn("secretKeyRef:", self.push)
        self.assertIn("key: token", self.push)

    def test_workflow_images_are_not_floating(self) -> None:
        for workflow in (self.homebrew, self.release):
            for image in re.findall(
                r"^\s+image:\s*(\S+)", workflow, re.MULTILINE
            ):
                self.assertNotRegex(image, r":latest(?:$|\s)")


if __name__ == "__main__":
    unittest.main()
