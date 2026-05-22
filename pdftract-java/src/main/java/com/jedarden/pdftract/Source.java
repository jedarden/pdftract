package com.jedarden.pdftract;

import java.net.URI;
import java.nio.file.Path;
import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;

/**
 * Sealed interface for PDF input sources.
 * Supports file paths, URLs, and raw bytes.
 */
public sealed interface Source permits PathSource, UrlSource, BytesSource {
    /**
     * Converts this source to CLI arguments.
     */
    List<String> toArgs();

    /**
     * Creates a Source from a file path.
     */
    static PathSource fromPath(Path path) {
        return new PathSource(path.toString());
    }

    /**
     * Creates a Source from a file path string.
     */
    static PathSource fromPath(String path) {
        return new PathSource(path);
    }

    /**
     * Creates a Source from a URL.
     */
    static UrlSource fromUrl(URI url) {
        return new UrlSource(url.toString());
    }

    /**
     * Creates a Source from a URL string.
     */
    static UrlSource fromUrl(String url) {
        return new UrlSource(url);
    }

    /**
     * Creates a Source from raw bytes.
     * Note: Writes bytes to a temporary file.
     */
    static BytesSource fromBytes(byte[] bytes) {
        return new BytesSource(bytes);
    }
}
