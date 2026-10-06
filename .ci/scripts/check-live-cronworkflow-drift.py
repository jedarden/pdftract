#!/usr/bin/env python3
"""Compare a GitOps CronWorkflow manifest with a read-only live API response."""

import json
import sys
from pathlib import Path

import yaml


def differences(expected, actual, path="spec"):
    if type(expected) is not type(actual):
        yield path
    elif isinstance(expected, dict):
        for key in sorted(expected.keys() | actual.keys()):
            child = f"{path}.{key}"
            if key not in expected or key not in actual:
                yield child
            else:
                yield from differences(expected[key], actual[key], child)
    elif isinstance(expected, list):
        if len(expected) != len(actual):
            yield f"{path}.length"
        for index, (left, right) in enumerate(zip(expected, actual)):
            yield from differences(left, right, f"{path}[{index}]")
    elif expected != actual:
        yield path


def main():
    manifest_path, live_path, name, schedule = sys.argv[1:]
    expected = yaml.safe_load(Path(manifest_path).read_text())
    live = json.loads(Path(live_path).read_text())
    for label, document in (("GitOps", expected), ("live", live)):
        if (document.get("apiVersion") != "argoproj.io/v1alpha1"
                or document.get("kind") != "CronWorkflow"
                or document.get("metadata", {}).get("name") != name
                or document.get("metadata", {}).get("namespace") != "argo-workflows"):
            print(f"ERROR: {label} CronWorkflow identity differs: {name}", file=sys.stderr)
            return 1
        spec = document.get("spec", {})
        if spec.get("schedules") != [schedule] or spec.get("timezone") != "UTC":
            print(f"ERROR: {label} CronWorkflow schedule differs from {schedule} UTC: {name}", file=sys.stderr)
            return 1
    drift = list(differences(expected["spec"], live["spec"]))
    if drift:
        print(f"ERROR: live CronWorkflow spec differs from GitOps: {name}", file=sys.stderr)
        for path in drift[:12]:
            print(f"  {path}", file=sys.stderr)
        if len(drift) > 12:
            print(f"  ... {len(drift) - 12} more paths", file=sys.stderr)
        return 1
    print(f"Live CronWorkflow matches GitOps: {name} ({schedule} UTC)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
