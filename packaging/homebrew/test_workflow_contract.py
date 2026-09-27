#!/usr/bin/env python3
"""Static contract checks for the release-cascade Homebrew handoff."""

from __future__ import annotations

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
        cls.render = cls.homebrew.split("    - name: render-formula", 1)[1].split(
            "    # === Push to Tap ===", 1
        )[0]
        cls.push = cls.homebrew.split("    - name: push-tap", 1)[1].split(
            "    # === Wait for Mirror ===", 1
        )[0]

    def test_release_producer_hands_off_signed_source_archive_digest(self) -> None:
        self.assertIn(
            'SOURCE_ARCHIVE="source/pdftract-v${VERSION}.tar.gz"', self.release
        )
        self.assertIn(
            'SOURCE_ARCHIVE_URL="https://github.com/${REPO}/archive/refs/tags/${TAG}.tar.gz"',
            self.release,
        )
        self.assertIn('sha256sum "${SOURCE_ARCHIVE}" >> SHA256SUMS', self.release)

    def test_render_consumes_checksum_handoff_without_archive_fetch_or_rehash(self) -> None:
        self.assertIn("source-archive-sha256", self.homebrew)
        self.assertIn("{{tasks.verify-release-metadata.outputs.parameters.source-archive-sha256}}", self.homebrew)
        self.assertIn('SOURCE_ARCHIVE_SHA256="{{inputs.parameters.source-archive-sha256}}"', self.render)
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
        self.assertIn(
            'when: "{{tasks.versioned-tag-gate.outputs.parameters.proceed}} == true && {{workflow.parameters.dry_run}} != true"',
            self.homebrew,
        )

    def test_publication_is_pinned_to_the_approved_tap_and_formula_path(self) -> None:
        self.assertIn(
            'TAP_URL="https://git.ardenone.com/jedarden/homebrew-tap.git"',
            self.push,
        )
        self.assertNotIn("{{workflow.parameters.tap-url}}", self.push)
        self.assertIn('MIRROR="https://github.com/jedarden/homebrew-tap.git"', self.homebrew)
        self.assertIn("git add -- Formula/pdftract.rb", self.push)
        self.assertIn('[ "${STAGED_PATHS}" = "Formula/pdftract.rb" ]', self.push)

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
            self.assertNotIn("image: alpine:latest", workflow)
            self.assertNotIn("image: orhunp/git-cliff:latest", workflow)


if __name__ == "__main__":
    unittest.main()
