# pdftract-3mdb7: Hover Tooltips Implementation

## Verification Summary

The hover tooltip functionality is **already fully implemented** in the inspector frontend. No changes were required.

## Implementation Location

- **HTML**: `crates/pdftract-cli/src/inspect/frontend/index.html:57`
  - Single tooltip div: `<div id="tooltip" class="tooltip" hidden></div>`

- **CSS**: `crates/pdftract-cli/src/inspect/frontend/style.css:30`
  - Position: absolute
  - Background: rgba(255,255,255,0.95)
  - Border: 1px solid #ccc
  - Padding: 6px 10px
  - Font-family: monospace (ui-monospace, SF Mono, Menlo, Consolas)
  - Font-size: 12px
  - No CSS transitions (immediate appearance)

- **JavaScript**: `crates/pdftract-cli/src/inspect/frontend/app.js:355-418`
  - `setupTooltips(svg)` function
  - Event delegation on SVG container
  - Reads data-* attributes from target elements
  - Positions tooltip relative to cursor with edge detection
  - Uses `textContent` for XSS prevention

## Acceptance Criteria Verification

### PASS Criteria

1. **Hovering a span → tooltip visible within 50 ms**
   - ✅ Uses `tooltip.hidden = false` immediately on mouseover (line 383)
   - ✅ No CSS transition-duration (immediate appearance)
   - ✅ Uses `display` property via `hidden` attribute (no transition)

2. **Tooltip shows the data-* attrs as formatted rows**
   - ✅ Lines 365-377: Reads and formats data attributes:
     - `data-text` → "Text: {value}"
     - `data-font` + `data-size` → "Font: {name} {size}pt"
     - `data-confidence` → "Confidence: {value}"
     - `data-bbox` → "Bbox: {value}"
     - `data-blockRef` → "Block: {value}"
     - `data-mcid` → "MCID: {value}"
     - `data-readingIdx` → "Reading idx: {value}"

3. **mouseleave hides the tooltip**
   - ✅ Lines 388-390: `mouseout` event sets `tooltip.hidden = true`

4. **Cursor near right edge: tooltip auto-repositions**
   - ✅ Lines 396-417: `positionTooltip(x, y)` function:
     - Checks if tooltip would exceed viewport width
     - Repositions to cursor-left when needed
     - Also handles bottom edge

5. **No XSS via data-text content**
   - ✅ Line 382: Uses `tooltip.textContent = content` (not innerHTML)
   - ✅ Content is treated as plain text, not HTML

### WARN Criteria

None. Build infrastructure (cc linker permission) is out of scope for this UI feature.

### FAIL Criteria

None.

## Technical Details

### Event Handling Strategy
- **Event delegation**: Single event listener on SVG container (line 359)
- Uses `mouseover`/`mouseout`/`mousemove` events
- Target detection via `e.target.closest('[data-text], [data-kind]')`
- Avoids per-span listener registration (better performance)

### Positioning Algorithm
1. Default position: cursor + (8, 8) offset
2. Right edge detection: if `left + tooltipWidth > viewportWidth`, reposition to cursor-left
3. Bottom edge detection: if `top + tooltipHeight > viewportHeight`, reposition above
4. Boundary clamping: keeps tooltip within viewport with minimum offset

### Data Attributes Expected
The tooltip reads these attributes from span elements (set server-side during SVG generation):
- `data-text` - Span text content
- `data-font` - Font name
- `data-size` - Font size in points
- `data-confidence` - OCR confidence score
- `data-bbox` - Bounding box coordinates
- `data-blockRef` - Block index reference
- `data-mcid` - MCID value
- `data-readingIdx` - Reading order index

## Integration Points

- **Server-side SVG generation**: Sets data-* attributes on span elements (Phase 7.9.4 sibling)
- **setupTooltips() call**: Invoked from `renderPageSingle()` (line 108) and `renderPageComparison()` (lines 129, 141)
- **Comparison mode**: Tooltips work on both side A and side B SVGs

## Code Review Summary

The implementation was found to be complete and correct:
- Single shared tooltip div (cheaper than per-span)
- XSS-safe text content rendering
- Immediate appearance (no debouncing, no CSS transitions)
- Edge-aware positioning
- Event delegation for performance
- Works in both single-view and comparison modes

No code changes were required.
