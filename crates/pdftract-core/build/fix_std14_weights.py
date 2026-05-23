#!/usr/bin/env python3
"""Fix std14-metrics.json to ensure all fonts have exactly 256 weights."""

import json
import sys

def main():
    json_path = "crates/pdftract-core/build/std14-metrics.json"

    with open(json_path, 'r') as f:
        data = json.load(f)

    for font_name, font_data in data["fonts"].items():
        weights = font_data["weights"]
        if len(weights) < 256:
            print(f"Padding {font_name}: {len(weights)} -> 256")
            # Pad with zeros
            font_data["weights"] = weights + [0] * (256 - len(weights))
        elif len(weights) > 256:
            print(f"Truncating {font_name}: {len(weights)} -> 256")
            font_data["weights"] = weights[:256]

    # Write back
    with open(json_path, 'w') as f:
        json.dump(data, f, indent=2)

    print("Fixed!")

if __name__ == "__main__":
    main()
