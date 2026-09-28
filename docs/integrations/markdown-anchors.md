# Markdown Anchors Integration Guide

This document describes the positional HTML comment anchors feature in pdftract's Markdown output.

## Overview

When `--md-anchors` is enabled, each block in markdown output is preceded by a single-line HTML comment containing positional metadata. This allows downstream tools (LLM agents, audit tools, document Q&A systems) to map a Markdown excerpt back to a precise PDF location.

## Anchor Format

Each anchor is a single-line HTML comment:

```markdown
<!-- pdftract: page=3 block=12 bbox=[72.0,640.5,540.0,672.0] kind=heading -->
## Chapter 3
```

### Fields

- `page`: Zero-based page index (0, 1, 2, ...)
- `block`: Zero-based block index within the page (0, 1, 2, ...)
- `bbox`: Bounding box in PDF points `[x0, y0, x1, y1]` with 1 decimal place precision. Coordinates may be negative — pages with a negative MediaBox origin (e.g. bleed or crop boxes) are legal PDFs, so each component admits a leading `-`.
- `kind`: Block kind (`heading`, `paragraph`, `list`, `table`, `figure`, etc.)

### Regex Schema

The anchor format is parseable with this stable regex:

```regex
<!--\s*pdftract:\s*page=(\d+)\s+block=(\d+)\s+bbox=\[([-\d.,]+)\]\s+kind=(\w+)\s*-->
```

## Usage

### CLI

```bash
# Enable anchors in markdown output
pdftract extract input.pdf --format markdown -o output --md-anchors
```

### Rust API

```rust
use pdftract_core::markdown::{parse_anchors, Anchor};

// Parse anchors from markdown text
let md = std::fs::read_to_string("output.md")?;
let anchors = parse_anchors(&md);

for anchor in anchors {
    println!("Page {} Block {} at {:?}", anchor.page, anchor.block, anchor.bbox);
}
```

## Properties

### Stability

The anchor format is a **stable public API**. The regex schema will not change in a breaking way across minor versions. New fields may be added, but existing fields will remain compatible.

This contract — the published regex (kept byte-identical to the `ANCHOR_REGEX_PATTERN` constant and the Python/JavaScript examples above), per-page block indexing, 1-decimal bbox components admitting negative coordinates, fence-skipping in `parse_anchors`, empty-block anchors, and the round-trip property — is pinned by the regression tests in `crates/pdftract-core/tests/markdown_anchor_contract.rs` and `crates/pdftract-core/tests/markdown_anchor_edge_cases.rs`. The CLI `--md-anchors` emission path is pinned by `crates/pdftract-cli/tests/markdown_anchors.rs`.

### Passthrough

HTML comments are passthrough in every major Markdown renderer:
- GitHub
- GitLab
- Obsidian
- Notion import
- pulldown-cmark
- marked
- markdown-it

Anchored output remains human-readable while machines can recover positional metadata.

### Round-trip

A round-trip property holds: extracting → parsing anchors → recovering the original block list (modulo inline styling, which is lossy in Markdown).

## Edge Cases

### Code Fences

HTML comments inside code fences (```) are not recognized by Markdown renderers—they're emitted verbatim. This is a limitation of the Markdown spec, not pdftract.

pdftract never emits a real anchor *inside* a fence: every anchor is written before its block content at fence depth 0. Conversely, `parse_anchors` ignores anchor-like comments that appear *inside* fenced code blocks—a document whose code content quotes the anchor format (like this guide) must not gain phantom blocks in the recovered list. Integrators using the raw regex should apply the same rule: skip matches inside ``` / ~~~ fences.

### Empty Blocks

Empty blocks (e.g., blank pages) still emit anchors with empty content following.

### Block Index

Block index is **per-page**, not global. Each page starts at block 0. Use the `page` field to compute global indices if needed.

## Examples

### Heading with Anchor

```markdown
<!-- pdftract: page=0 block=0 bbox=[72.0,640.5,540.0,672.0] kind=heading -->
# Introduction
```

### Paragraph with Anchor

```markdown
<!-- pdftract: page=0 block=1 bbox=[72.0,600.0,540.0,630.0] kind=paragraph -->
This is the first paragraph of the document.
```

### Table with Anchor

```markdown
<!-- pdftract: page=1 block=0 bbox=[72.0,500.0,540.0,400.0] kind=table -->
| Column 1 | Column 2 |
|----------|----------|
| Cell 1   | Cell 2   |
```

## Integration Examples

### Python: Extract Anchors

```python
import re

ANCHOR_RE = re.compile(
    r'<!--\s*pdftract:\s*page=(\d+)\s+block=(\d+)\s+bbox=\[([-\d.,]+)\]\s+kind=(\w+)\s*-->'
)
FENCE_RE = re.compile(r'^( {0,3})(`{3,}|~{3,})(.*)$')

def extract_anchors(md_text):
    """Return list of (page, block, bbox, kind) tuples."""
    anchors = []
    fence = None
    for line in md_text.splitlines():
        marker = FENCE_RE.match(line)
        if fence:
            if (marker and marker.group(2)[0] == fence[0]
                    and len(marker.group(2)) >= fence[1]
                    and not marker.group(3).strip()):
                fence = None
            continue
        if marker:
            fence = (marker.group(2)[0], len(marker.group(2)))
            continue
        for match in ANCHOR_RE.finditer(line):
            page = int(match.group(1))
            block = int(match.group(2))
            bbox = [float(x) for x in match.group(3).split(',')]
            kind = match.group(4)
            anchors.append((page, block, bbox, kind))
    return anchors
```

### JavaScript: Parse Anchors

```javascript
const ANCHOR_RE = /<!--\s*pdftract:\s*page=(\d+)\s+block=(\d+)\s+bbox=\[([-\d.,]+)\]\s+kind=(\w+)\s*-->/g;
const FENCE_RE = /^( {0,3})(`{3,}|~{3,})(.*)$/;

function extractAnchors(md) {
    const anchors = [];
    let fence = null;
    for (const line of md.split(/\r?\n/)) {
        const marker = line.match(FENCE_RE);
        if (fence) {
            if (marker && marker[2][0] === fence[0]
                    && marker[2].length >= fence[1]
                    && marker[3].trim() === '') {
                fence = null;
            }
            continue;
        }
        if (marker) {
            fence = [marker[2][0], marker[2].length];
            continue;
        }
        for (const match of line.matchAll(ANCHOR_RE)) {
            anchors.push({
                page: parseInt(match[1]),
                block: parseInt(match[2]),
                bbox: match[3].split(',').map(Number),
                kind: match[4]
            });
        }
    }
    return anchors;
}
```

## Version History

- **v0.1.0**: Initial release with `--md-anchors` flag and stable regex schema.
