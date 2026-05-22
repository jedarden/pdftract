package com.jedarden.pdftract;

import java.util.List;

/**
 * Source from a local file path.
 */
public record PathSource(String path) implements Source {
    @Override
    public List<String> toArgs() {
        return List.of(path);
    }
}
