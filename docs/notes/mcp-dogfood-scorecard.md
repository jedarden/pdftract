# pdftract MCP phase 2 dogfood scorecard

Date: 2026-09-26
Pilot plan: [`mcp-dogfood-pilot.md`](mcp-dogfood-pilot.md)
Client: Claude Code 2.1.283
Server: project-scoped `.mcp.json`, `pdftract mcp --stdio --root ${PWD}`

## Scope and method

This is the phase-2 acceptance snapshot. It covers 20 repository-owned PDFs in
four stress classes, five documents per class. The rows use anonymized IDs so
this note contains metrics only; no PDF text, query terms, or private document
paths are recorded.

Each row was run through the MCP stdio server with newline-delimited JSON after
the Claude Code compatibility fix. The paired baseline was PyMuPDF 1.27.2.2
opening the same PDF and extracting page text with `sort=True`.

- `extract`: MCP `extract_text` returned a non-error result.
- `U+FFFD`: replacement-character count in the MCP text result.
- `headings`: Markdown heading fidelity against PyMuPDF outline/heading
  candidates; `na` means the baseline had no heading candidate.
- `page-1 order`: first-page baseline tokens appeared in the same order in MCP
  text; `na` means the baseline had no checkable tokens.
- `search recall`: MCP matches divided by case-insensitive baseline grep hits.
  `0% (error)` is an MCP `-32000` result from the intentionally loud,
  unimplemented search tool; `na` means the baseline query had no hit.

## Per-document metrics

| ID | Class | Baseline pages | Baseline chars | Extract | U+FFFD | Headings | Page-1 order | Search recall |
|---|---|---:|---:|---|---:|---|---|---|
| D01 | signed-legal | 4 | 2459 | pass | 0 | pass 4/4 | pass | 0% (error) |
| D02 | signed-legal | 1 | 2049 | pass | 0 | na | pass | 0% (error) |
| D03 | signed-legal | 2 | 2049 | pass | 0 | na | pass | 0% (error) |
| D04 | signed-legal | 6 | 1217 | pass | 0 | pass 6/6 | pass | 0% (error) |
| D05 | signed-legal | 1 | 20 | pass | 0 | na | pass | 0% (error) |
| D06 | business-report | 1 | 12 | pass | 0 | pass 1/1 | pass | 0% (error) |
| D07 | business-report | 1 | 243 | pass | 0 | pass 6/6 | pass | 0% (error) |
| D08 | business-report | 1 | 242 | pass | 0 | pass 3/3 | pass | 0% (error) |
| D09 | business-report | 1 | 66 | pass | 0 | pass 3/3 | pass | 0% (error) |
| D10 | business-report | 1 | 195 | pass | 0 | pass 1/1 | pass | 0% (error) |
| D11 | manual | 1 | 44 | pass | 0 | pass 1/1 | pass | 0% (error) |
| D12 | manual | 1 | 803 | pass | 0 | pass 1/1 | pass | 0% (error) |
| D13 | manual | 1 | 119 | pass | 0 | pass 2/2 | pass | 0% (error) |
| D14 | manual | 1 | 202 | pass | 0 | pass 1/1 | pass | 0% (error) |
| D15 | manual | 1 | 95 | pass | 0 | na | pass | 0% (error) |
| D16 | research-note | 1 | 33 | pass | 0 | na | pass | 0% (error) |
| D17 | research-note | 1 | 8 | pass | 7 | na | na | na |
| D18 | research-note | 1 | 10 | pass | 0 | na | fail | 0% (error) |
| D19 | research-note | 1 | 0 | pass | 0 | na | na | na |
| D20 | research-note | 1 | 633 | pass | 0 | fail 5/6 | pass | 0% (error) |

## Gate summary

| Class | Documents | Extraction parity | Zero-wedge runs |
|---|---:|---:|---:|
| signed-legal | 5 | 5/5 (100%) | yes |
| business-report | 5 | 5/5 (100%) | yes |
| manual | 5 | 5/5 (100%) | yes |
| research-note | 5 | 5/5 (100%) | yes |
| **Total** | **20** | **20/20 (100%)** | **yes** |

The extraction gate is met: every class is above the 90% parity bar and the
20-document run produced zero server wedges. MCP initialization and `tools/list`
also passed in the same run. The Claude Code session successfully discovered
the seven advertised tools and called `get_metadata` on a real PDF.

## Defects and recommendation

Every defect found in this run has an owning bead:

- `pdftract-3b44b696` — existing pilot defect: `search` is advertised but not
  implemented, so search recall is unavailable and recorded as an error.
- `pdftract-72d07d4a` — one Markdown heading-fidelity miss.
- `pdftract-91971f16` — one page-1 reading-order miss.
- `pdftract-47b82001` — seven U+FFFD characters in the CJK stress document.
- `pdftract-e417eaea` — Claude Code initially exposed an additional protocol
  defect: it sends newline-delimited JSON; pdftract accepted only LSP framing.
  The compatibility fix is included with this pilot wiring and preserves both
  framings.

Recommendation: allow controlled Claude Code dogfooding for extraction, but
descope release sign-off and any v0.1.0 registry publication until the
search-dependent scorecard metric is implementable, the three new quality
beads are triaged, and a full fourteen-calendar-day observation window has
elapsed. This snapshot establishes the required parity and no-wedge baseline;
it does not claim that the shorter run is itself a completed two-week window.
