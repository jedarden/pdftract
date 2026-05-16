# PDF Resource Dictionaries, Resource Inheritance, and Namespace Isolation

## Overview

Every operator in a PDF content stream that references a named resource — a font, an image, a graphics state, a pattern — resolves that name through a layered lookup mechanism defined by the PDF specification. Implementing this mechanism correctly is a prerequisite for accurate text extraction, because a single character of text depends on identifying the right font object, and font naming in PDF is strictly local, not global. This document describes the full resource resolution semantics that pdftract must implement, from the structure of resource dictionaries through inheritance traversal, namespace isolation in Form XObjects, and graceful handling of malformed references.

## Resource Dictionary Structure

The `/Resources` key on a page dictionary (or Form XObject dictionary) holds a resource dictionary that organizes all named resources available to that object's content stream. The PDF specification defines six typed sub-dictionaries within a resource dictionary:

- `/Font` — maps local font names to font object references
- `/XObject` — maps local names to image or Form XObjects
- `/ExtGState` — maps local names to graphics state parameter dictionaries
- `/ColorSpace` — maps local names to color space definitions
- `/Pattern` — maps local names to tiling pattern or shading pattern dictionaries
- `/Shading` — maps local names to shading dictionaries used by the `sh` operator

Each sub-dictionary is a PDF dictionary whose keys are name objects (the local resource names used in the content stream) and whose values are either inline dictionaries or indirect object references. A seventh entry, `/ProcSet`, historically listed the set of procedure sets required to interpret the stream. Modern PDF processors ignore `/ProcSet` entirely; it carries no semantic weight and pdftract must accept its presence without acting on it.

## Resource Inheritance Through the Page Tree

PDF page dictionaries are organized in a tree structure under the document catalog's `/Pages` node. Interior nodes of this tree (page tree nodes) may carry a `/Resources` entry that applies to all their descendant pages. A page dictionary that omits `/Resources` inherits the nearest ancestor's resource dictionary, found by traversing the `/Parent` chain upward until a node with `/Resources` is encountered.

pdftract must implement this traversal explicitly. When resolving a resource name during content stream processing, the lookup begins with the page's own dictionary. If `/Resources` is absent, the processor walks the `/Parent` references — each `/Parent` pointer leads to a page tree node — and checks each ancestor in turn until a `/Resources` dictionary is found or the root is reached. The first `/Resources` encountered wins; shadowing does not propagate further up the chain.

This has a practical implication for implementation: the resource dictionary associated with a page must be resolved before content stream processing begins, not lazily during operator dispatch. pdftract should perform the full inheritance walk at page-load time and cache the resolved resource dictionary for the lifetime of content stream processing for that page.

## Font Resource Lookup

Within the `/Font` sub-dictionary, each entry maps a local name (such as `/F1`, `/TT0`, or any arbitrary PDF name) to an indirect reference to a font object. The `Tf` operator in a content stream selects the current font by supplying one of these local names along with a size: `Tf /F1 12`.

The critical property of font naming is that local names are scoped to the resource dictionary that contains them. The name `/F1` in page A's `/Font` sub-dictionary and the name `/F1` in page B's `/Font` sub-dictionary are entirely independent and may reference different font objects — different encodings, different glyph sets, different CIDFont definitions. There is no global font namespace in PDF. pdftract must never assume that a font name seen on one page carries any meaning on another page. Each page's font lookup must go through that page's (or its inherited) `/Font` sub-dictionary.

## XObject Resource Lookup

The `/XObject` sub-dictionary maps local names to XObject streams. An XObject is either an image (subtype `/Image`) or a Form XObject (subtype `/Form`). The `Do` operator in a content stream takes a single name operand, looks it up in the current context's `/XObject` sub-dictionary, and either renders the image or recursively processes the Form XObject's content stream. During text extraction, image XObjects are irrelevant to the character stream, but encountering a `Do` operator referencing a Form XObject requires that pdftract recursively enter and process that Form XObject's content stream to capture any text it contains.

## Form XObject Resource Isolation

Form XObjects are the primary source of complexity in resource resolution. A Form XObject has its own `/Resources` dictionary, embedded in its stream dictionary, that is entirely separate from the invoking page's resource dictionary. When pdftract enters a Form XObject via a `Do` operator, the resource context must switch completely to the Form XObject's own resources. Operators inside the Form XObject's content stream — including `Tf` operators that select fonts — resolve all names against the Form XObject's `/Resources`, not the page's.

This isolation is absolute: the Form XObject's `/Font` sub-dictionary is the authoritative namespace for all `Tf` operators inside that Form XObject. A font named `/F1` inside the Form XObject refers to the object listed under `/F1` in the Form XObject's `/Font` sub-dictionary, regardless of what `/F1` means in the enclosing page's resources.

## ExtGState Lookup

The `/ExtGState` sub-dictionary maps local names to graphics state parameter dictionaries. The `gs` operator takes a local name, looks it up in `/ExtGState`, and applies the parameters in the referenced dictionary to the current graphics state. Several entries within an ExtGState dictionary are relevant to text extraction or rendering state:

- `/Font` — a two-element array `[font-object size]` that sets the current font and size, equivalent to a `Tf` operation. pdftract must handle this the same way it handles an explicit `Tf` operator.
- `/ca` — non-stroking (fill) opacity; relevant for determining whether text is visually transparent.
- `/CA` — stroking opacity.
- `/BM` — blend mode; affects compositing but rarely changes text extraction logic.
- `/SMask` — soft mask; may affect visibility but is secondary to text position extraction.

When a `gs` operator is encountered, pdftract must resolve the name through the current context's `/ExtGState` sub-dictionary (respecting the resource stack described below) and process any `/Font` entry within the resulting dictionary.

## Nested Form XObjects

A Form XObject's content stream may itself contain `Do` operators, referencing further Form XObjects listed in the Form XObject's own `/XObject` sub-dictionary. Nesting depth is unlimited by the specification. This means resource context switching is recursive: entering a second-level Form XObject switches to that object's `/Resources`, and returning from it restores the first-level Form XObject's resources.

Although the PDF specification prohibits cycles in the XObject reference graph, malformed PDFs may include them. A Form XObject that directly or indirectly contains a `Do` reference to itself would cause infinite recursion if pdftract does not detect and break the cycle. pdftract must maintain a set of currently-active Form XObject object numbers during recursive processing. Before entering a Form XObject, its object number is checked against this set. If it is already present, pdftract skips the `Do` operation and records a warning. On return from a Form XObject, its object number is removed from the set.

## Pattern and ColorSpace Resources

The `/Pattern` sub-dictionary names tiling patterns and shading patterns, referenced by painting operators when a pattern color space is active. The `/ColorSpace` sub-dictionary names color spaces such as `DeviceN` or `Separation` that can be referenced by name in color selection operators. Neither of these resource types directly contributes to text extraction. However, pdftract must not crash when content streams reference pattern or color space names. A lookup that returns a pattern or color space object should be handled gracefully: apply no text-relevant state change, continue processing, and do not surface an error to the caller.

## ResourceStack Implementation

The correct abstraction for resource context during content stream processing is a stack of resource dictionaries. pdftract should define a `ResourceStack` structure that supports three operations:

1. **Push** — called when entering a Form XObject; pushes the Form XObject's `/Resources` dictionary onto the top of the stack.
2. **Pop** — called when returning from a Form XObject; removes the top entry.
3. **Lookup(type, name)** — resolves a resource name within a given sub-dictionary type (`/Font`, `/XObject`, `/ExtGState`, etc.) by searching from the top of the stack downward, returning the first match found.

At the start of content stream processing for a page, the stack is initialized with a single entry: the page's resolved (inheritance-applied) resource dictionary. Each `Do` operator that invokes a Form XObject pushes that Form XObject's `/Resources` onto the stack before recursing into the Form XObject's content stream, and pops it on return.

The top-to-bottom search order in `Lookup` ensures that Form XObject-local names shadow any identically named resources in enclosing contexts. This is the correct behavior: a Form XObject's author controls the names within that Form XObject's scope without any obligation to avoid conflicts with names in the enclosing page.

## Missing Resources and Graceful Degradation

Malformed PDFs may contain content streams that reference resource names absent from the applicable `/Resources` dictionary. A `Tf` operator naming `/F3` when `/F3` does not appear in the current context's `/Font` sub-dictionary is not a fatal error; it is a recoverable condition. pdftract should treat a missing font lookup as an unknown font: the current font state is set to an unresolved sentinel, glyph-to-character mapping falls through to the fallback pipeline (Unicode inference from glyph names, ToUnicode CMap if a font object is eventually identified, or raw codepoint passthrough), and processing continues. The missing reference should be recorded in the extraction diagnostic log.

Similarly, a `Do` operator referencing an XObject name absent from `/XObject` should log the missing reference and skip the operation rather than halting extraction. Pattern, color space, and ExtGState lookup failures follow the same pattern: log and continue.

## Summary

Correct PDF resource resolution requires implementing three interlocking mechanisms: inheritance traversal up the page tree to find the applicable `/Resources` dictionary, per-page (and per-Form-XObject) namespace isolation that prevents any cross-page or cross-object name aliasing, and a resource stack that tracks context switches as Form XObjects are entered and exited during recursive content stream processing. Font name resolution is the most critical path for text extraction; every `Tf` operator and every `/Font` entry in an ExtGState must resolve through the stack's current top context. Cycle detection prevents malformed inputs from causing unbounded recursion. Missing resources must degrade gracefully into the fallback pipeline rather than surfacing as hard failures. Together these behaviors allow pdftract to process content streams from simple single-page documents and deeply nested Form XObject hierarchies alike, extracting text accurately regardless of the resource structure the PDF author chose.
