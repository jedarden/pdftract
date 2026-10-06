#!/usr/bin/env python3
"""Exercise the mounted collector with terminal pod, log, and fuzz fixtures."""
import io
import json
import os
from pathlib import Path
import shutil
import ssl
import sys
import tempfile
import urllib.request


def collector_source():
    manifest = Path(__file__).resolve().parents[1] / "argo-workflows/pdftract-nightly-evidence-configmap.yaml"
    lines = manifest.read_text().splitlines()
    start = lines.index("  capture.py: |") + 1
    return "\n".join(line[4:] for line in lines[start:]) + "\n"


source = collector_source()
compile(source, "capture.py", "exec")
with tempfile.TemporaryDirectory(prefix="nightly-evidence-test.", dir=".ci") as directory:
    root = Path(directory).resolve()
    evidence = root / "evidence"
    sa = root / "sa"
    sa.mkdir()
    (sa / "token").write_text("fixture-token")
    (sa / "namespace").write_text("argo-workflows")
    (sa / "ca.crt").write_text("fixture-ca")
    coverage = root / "workspace/pdftract/fuzz/coverage/lexer/coverage.profdata"
    coverage.parent.mkdir(parents=True)
    coverage.write_bytes(b"coverage-profile")
    crash = root / "fuzz-artifacts/lexer/crash-abc"
    crash.parent.mkdir(parents=True)
    crash.write_bytes(b"crash-input")
    source = source.replace('Path("/evidence")', 'Path(%r)' % str(evidence))
    source = source.replace('Path("/var/run/secrets/kubernetes.io/serviceaccount")', 'Path(%r)' % str(sa))
    source = source.replace('Path("/workspace/pdftract/fuzz/coverage")',
                            'Path(%r)' % str(root / "workspace/pdftract/fuzz/coverage"))
    source = source.replace('Path("/fuzz-artifacts")', 'Path(%r)' % str(root / "fuzz-artifacts"))

    name = "pdftract-nightly-fuzz-12345"
    pod = name + "-setup-123"
    pods = {"items": [
        {"metadata": {"name": pod}, "status": {"phase": "Failed",
         "initContainerStatuses": [{"name": "clone-source", "state": {"terminated": {"exitCode": 0}}}],
         "containerStatuses": [{"name": "main", "state": {"terminated": {"exitCode": 5}}}]}},
        {"metadata": {"name": name + "-capture-123"}, "status": {"phase": "Running"}},
    ]}
    revision = "a" * 40

    def urlopen(request, context, timeout):
        url = request.full_url
        if "?labelSelector=" in url:
            return io.BytesIO(json.dumps(pods).encode())
        if "container=clone-source" in url:
            return io.BytesIO((revision + "\n").encode())
        if "container=main" in url:
            return io.BytesIO(b"run failed: token=private-value\n")
        raise AssertionError("unexpected API request " + url)

    urllib.request.urlopen = urlopen
    ssl.create_default_context = lambda **kwargs: None
    os.environ["KUBERNETES_SERVICE_HOST"] = "test-api"
    os.environ["KUBERNETES_SERVICE_PORT"] = "443"
    sys.argv = ["capture.py", name, "Failed", "pdftract-nightly-fuzz"]
    exec(compile(source, "capture.py", "exec"), {"__name__": "__main__"})

    manifest = json.loads((evidence / "manifest.json").read_text())
    assert manifest["workflow"] == name
    assert manifest["final_phase"] == "Failed"
    assert manifest["source_revision"] == revision
    assert manifest["pods"][0]["containers"][1]["exit_code"] == 5
    assert len(manifest["pods"]) == 1
    assert len(manifest["coverage"]) == 1
    assert len(manifest["crash_set"]) == 1
    assert "private-value" not in (evidence / "logs" / (pod + "-main.log")).read_text()

    crash.unlink()
    shutil.rmtree(evidence)
    exec(compile(source, "capture.py", "exec"), {"__name__": "__main__"})
    empty_manifest = json.loads((evidence / "manifest.json").read_text())
    assert empty_manifest["crash_set"] == []
    assert len(empty_manifest["coverage"]) == 1

print("nightly evidence collector fixture passed")
