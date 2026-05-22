package com.jedarden.pdftract

import com.jedarden.pdftract.codegen.*
import java.nio.file.Path
import java.util.stream.Stream

/**
 * Kotlin extension functions for pdftract.
 * These provide idiomatic Kotlin syntax while using the same jar as Java users.
 */

/**
 * Extract structured data from a PDF with Kotlin lambda syntax.
 *
 * Example:
 * ```kotlin
 * val doc = pdftract.extract(path.toPath()) {
 *     ocrLanguage = "eng"
 *     ocrThreshold = 0.7
 * }
 * ```
 */
fun Pdftract.extract(source: Path, init: ExtractOptions.() -> Unit = {}): Document {
    val options = ExtractOptions().apply(init)
    return extract(Source.fromPath(source), options)
}

/**
 * Extract from URL with Kotlin lambda syntax.
 */
fun Pdftract.extract(url: String, init: ExtractOptions.() -> Unit = {}): Document {
    val options = ExtractOptions().apply(init)
    return extract(Source.fromUrl(url), options)
}

/**
 * Extract from bytes with Kotlin lambda syntax.
 */
fun Pdftract.extract(bytes: ByteArray, init: ExtractOptions.() -> Unit = {}): Document {
    val options = ExtractOptions().apply(init)
    return extract(Source.fromBytes(bytes), options)
}

/**
 * Extract plain text with Kotlin lambda syntax.
 */
fun Pdftract.extractText(source: Path, init: ExtractOptions.() -> Unit = {}): String {
    val options = ExtractOptions().apply(init)
    return extractText(Source.fromPath(source), options)
}

/**
 * Extract Markdown with Kotlin lambda syntax.
 */
fun Pdftract.extractMarkdown(source: Path, init: ExtractOptions.() -> Unit = {}): String {
    val options = ExtractOptions().apply(init)
    return extractMarkdown(Source.fromPath(source), options)
}

/**
 * Stream extract pages with Kotlin lambda syntax.
 */
fun Pdftract.extractStream(source: Path, init: ExtractOptions.() -> Unit = {}): Sequence<Page> {
    val options = ExtractOptions().apply(init)
    val stream: Stream<Page> = extractStream(Source.fromPath(source), options)
    return stream.toSequence()
}

/**
 * Search with Kotlin lambda syntax.
 */
fun Pdftract.search(source: Path, pattern: String, init: SearchOptions.() -> Unit = {}): Sequence<Match> {
    val options = SearchOptions().apply(init)
    val stream: Stream<Match> = search(Source.fromPath(source), pattern, options)
    return stream.toSequence()
}

/**
 * Get metadata with Kotlin lambda syntax.
 */
fun Pdftract.getMetadata(source: Path, init: BaseOptions.() -> Unit = {}): Metadata {
    val options = BaseOptions().apply(init)
    return getMetadata(Source.fromPath(source), options)
}

/**
 * Compute fingerprint with Kotlin lambda syntax.
 */
fun Pdftract.hash(source: Path, init: BaseOptions.() -> Unit = {}): Fingerprint {
    val options = BaseOptions().apply(init)
    return hash(Source.fromPath(source), options)
}

/**
 * Invoke operator for use-with-resources pattern in Kotlin.
 *
 * Example:
 * ```kotlin
 * pdftract {
 *     val doc = extract(path.toPath())
 *     println(doc.pages.size)
 * }
 * ```
 */
inline operator fun Pdftract.invoke(block: Pdftract.() -> Unit) {
    use { it.block() }
}

/**
 * Extension to create ExtractOptions with DSL syntax.
 */
fun extractOptions(init: ExtractOptions.() -> Unit = {}): ExtractOptions {
    return ExtractOptions().apply(init)
}

/**
 * Extension to create SearchOptions with DSL syntax.
 */
fun searchOptions(init: SearchOptions.() -> Unit = {}): SearchOptions {
    return SearchOptions().apply(init)
}

/**
 * Extension to create BaseOptions with DSL syntax.
 */
fun baseOptions(init: BaseOptions.() -> Unit = {}): BaseOptions {
    return BaseOptions().apply(init)
}

/**
 * Convert Java Stream to Kotlin Sequence.
 */
private fun <T> Stream<T>.toSequence(): Sequence<T> {
    return Sequence { this.iterator() }
}
