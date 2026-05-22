package com.jedarden.pdftract;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

/**
 * Source from raw bytes.
 * Writes bytes to a temporary file for subprocess execution.
 */
public record BytesSource(byte[] bytes) implements Source {
    @Override
    public List<String> toArgs() {
        try {
            Path tempFile = Files.createTempFile("pdftract-", ".pdf");
            Files.write(tempFile, bytes);
            tempFile.toFile().deleteOnExit();
            return List.of(tempFile.toString());
        } catch (java.io.IOException e) {
            throw new RuntimeException("Failed to create temp file for bytes source", e);
        }
    }
}
