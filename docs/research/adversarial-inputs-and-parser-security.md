# Adversarial PDF Inputs, Resource Exhaustion, and Parser Security

PDF is a rich, decades-old format designed for faithful document reproduction. That richness makes it an attractive attack surface: a single file can encode compressed streams, cross-referenced object graphs, recursive content descriptions, and embedded attachments — all with a parser that must handle malformed or hostile inputs without crashing, hanging, or consuming unbounded resources. pdftract is designed to run in production environments where the input is fully untrusted. This document catalogs the concrete threats and the specific defensive techniques required to handle them safely.

## Decompression Bombs

FlateDecode (zlib) compressed streams are the most common compression method in PDF. A decompression bomb is a stream where a small compressed payload — sometimes under 1 KB — expands to an enormous output, often multiple gigabytes. The PDF specification places no upper bound on the decompression ratio, so a naive implementation that decompresses the full stream into memory before inspecting it will exhaust available RAM.

pdftract must enforce limits during incremental decompression. The correct approach is a streaming decompress loop that writes into a fixed-size output buffer, tracking total bytes emitted. If the decompressed size exceeds an absolute ceiling — 512 MB per stream is a reasonable production default — the stream is truncated and an error is recorded on the page or object being parsed. A ratio check provides an earlier warning: if the decompressed output reaches 1,000 times the compressed size before hitting the absolute cap, that stream is suspicious and can be flagged for logging even if it is still within the absolute limit. The key invariant is that the limit is enforced incrementally, not retroactively; pdftract must never allocate a buffer sized to the expected decompressed length before any data has been read.

## Deeply Nested Object Structures

PDF dictionaries and arrays can nest arbitrarily. A crafted file can encode an array of dictionaries of arrays thousands of levels deep. A recursive parser descending into this structure will overflow the thread stack before reaching the leaf values. The same hazard appears in object reference chains: object 1 references object 2, which references object 3, continuing to object N, where N can be crafted to exceed the stack depth available.

pdftract must parse object structures iteratively, maintaining an explicit stack on the heap rather than using the call stack. A maximum nesting depth of 512 levels is enforced at parse time; attempting to descend past this limit causes the current container to be returned as-is with its remaining children omitted. This depth limit applies uniformly to dictionaries, arrays, and the implicit nesting created by following indirect object references during value resolution.

## Circular Object References

Circular references — where object A references B, B references C, and C references A — create infinite loops during resolution if not detected. PDF's cross-reference mechanism makes these straightforward to construct: any object can reference any other by number, and the specification does not prohibit cycles.

Detection requires a thread-local resolution stack implemented as a `HashSet<u32>` of object numbers currently being resolved. Before following any indirect reference, pdftract inserts the target object number into the set. If the insert fails because the number is already present, a cycle has been detected; the lookup returns a null value immediately and the cycle is broken without recursing. The object number is removed from the set when resolution returns — this is a depth-first visited set, not a permanent memoization table, so different call paths can legitimately visit the same object independently.

## Enormous Object Counts

A PDF trailer's `/Size` entry declares the number of objects in the cross-reference table. A hostile file can set this to an extreme value such as 10,000,000, hoping the parser allocates a dense array of that size at startup. At even 8 bytes per slot, that is 80 MB of zero-initialized memory for a file that may contain only a dozen real objects.

pdftract uses a lazy, sparse object table. During startup, xref entries are recorded in a `HashMap<u32, u64>` mapping object number to byte offset — a structure that grows proportionally to actual entries, not to the declared `/Size`. Objects are not loaded or deserialized until they are explicitly requested. An LRU object cache bounds the number of simultaneously resident deserialized objects; objects evicted from the cache are re-parsed from their byte offsets on next access. This architecture means that a file with a fraudulent `/Size` of ten million but only a hundred real objects costs only the memory for those hundred cache entries.

## Malformed Stream Lengths

A PDF stream dictionary must contain a `/Length` key indicating the number of bytes before the `endstream` marker. Two failure modes exist. First, a `/Length` that is smaller than the actual stream content — perhaps `/Length 0` — leaves a parser that trusts the declared length stopping before the real data ends, potentially treating stream body bytes as top-level syntax and generating confusing parse errors downstream. Second, a `/Length` that is larger than the actual file — perhaps `/Length 100000000` pointing past end-of-file — causes a parser that naively allocates or reads that many bytes to either crash or consume excessive I/O.

pdftract validates stream lengths by never allocating more than is available from the current file position to EOF. When a declared length would overrun the file, pdftract clamps the read to the remaining bytes and searches forward for the `endstream` token to determine the actual boundary. For under-declared lengths, pdftract scans ahead for `endstream` as well, reading up to a configurable scan limit beyond the declared end. The correct stream boundary is always determined by physical token search, with the `/Length` value treated as a hint for buffer sizing only — never as an authoritative allocation size.

## Path Traversal in Embedded Filenames

PDF supports embedded file attachments via file specification dictionaries. These dictionaries include a `/F` (filename) entry that may contain arbitrary string data, including path traversal sequences such as `../../etc/passwd` or absolute paths like `/etc/shadow`. A consumer that writes attachment metadata or extracted files using the raw embedded filename without sanitization creates an arbitrary write primitive.

pdftract sanitizes all embedded filenames at the point of extraction. Only the final path component is retained — equivalent to `Path::file_name()` in Rust — and any string containing a path separator character (forward slash, backslash, or null byte) is rejected outright rather than stripped. Filenames that resolve to an empty string after sanitization are replaced with a generated placeholder. This sanitization is applied uniformly to both the `/F` and `/UF` (Unicode filename) fields; the Unicode form must be decoded before the separator check is applied.

## Content Stream Infinite Loops via Form XObjects

Page content streams can invoke reusable Form XObjects with the `Do` operator. A Form XObject is itself a content stream, and it can invoke other Form XObjects — including itself. A cycle in this graph causes a recursive descent that terminates only when the stack overflows.

pdftract tracks Form XObject invocations per page render using two complementary mechanisms. A `HashSet<u32>` of currently active Form XObject object numbers detects direct and indirect cycles: before invoking a Form XObject's content stream, its object number is inserted; if already present, the invocation is skipped. A separate hard counter on total `Do` operator invocations per page is enforced at 10,000 invocations; beyond this threshold, all further `Do` calls on the page are no-ops. The invocation count is not reset when descending into a Form XObject — it is a global budget for the entire page render.

## Integer Overflow in Coordinate Calculations

PDF coordinate spaces use arbitrary-precision floating-point values. A page may declare a coordinate transformation matrix with values on the order of 1e30, or glyph positions may be expressed as extremely large or small numbers. When these values are composed through a chain of transformations and ultimately converted to output coordinates, naive f32 arithmetic overflows silently, and even f64 can produce infinities or NaN values that propagate through a rendering pipeline.

pdftract uses `f64` throughout all coordinate calculations. After each transformation step, coordinates are clamped to a finite range before they are used in further arithmetic; any value that is infinite or NaN is replaced with zero. At the point where coordinates are mapped to output page dimensions, values are clamped to the page bounding box. This ensures that no coordinate value can exceed representable range at any stage, and that extreme input values produce bounded, predictable output rather than silent integer wraparound or floating-point exceptions.

## Time-Based Denial of Service

Quadratic or worse algorithms are a common source of parser DoS. A concrete example is span merging during text extraction: if each character operator on a page produces an individual span, and the merging pass compares each span against all previous spans to find candidates for concatenation, a page with 100,000 character operators requires 10 billion comparisons. Processing time becomes catastrophic on crafted inputs while remaining imperceptible on typical documents.

pdftract requires that all hot-path algorithms on per-page data run in O(n log n) or better. Span merging uses a sort-then-linear-scan approach: spans are sorted by position once, then a single left-to-right pass merges adjacent spans in O(n) time after the O(n log n) sort. Cross-reference table parsing, object graph traversal, and font encoding resolution are each analyzed for worst-case complexity before inclusion. Where complexity cannot be bounded analytically, per-operation counters with configurable ceilings are used to enforce a total work budget.

## Output Sanitization

Text extracted from PDF content streams may contain null bytes, control characters in the C0 and C1 ranges, bidirectional override sequences, or other byte sequences that are benign in PDF context but harmful when passed to downstream consumers. JSON parsers, databases, and logging systems all have different sensitivities; a null byte that terminates a C string in one layer may produce a truncated record that corrupts a lookup in another.

pdftract treats itself as the last trust boundary before extracted text enters a system. The output post-processing pipeline strips null bytes unconditionally, replaces control characters outside the normal whitespace set (tab, newline, carriage return) with the Unicode replacement character U+FFFD, and normalizes line endings. Unicode bidirectional override characters are logged and stripped by default, with an option to preserve them for applications that handle them explicitly. This is not optional behavior applied only when a caller opts in — it is applied to all text output because the cost of missing a hostile sequence in an edge case is higher than the marginal cost of cleaning safe inputs.

## Summary

Safe PDF parsing in a production environment is not primarily a correctness problem — it is a resource and trust boundary problem. The ten threat categories above each have a specific, bounded mitigation: decompression limits enforced during streaming, iterative parsing with depth caps, cycle detection via hash sets, sparse lazy object tables, physical token search for stream boundaries, filename sanitization at extraction time, invocation budgets for content stream recursion, f64 arithmetic with finite clamping, O(n log n) algorithm requirements on hot paths, and unconditional output sanitization. pdftract implements all of these as non-optional defaults, ensuring that untrusted PDF input cannot be used to exhaust memory, saturate CPU, traverse the filesystem, or inject hostile byte sequences into downstream systems.
