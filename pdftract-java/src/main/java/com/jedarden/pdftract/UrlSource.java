package com.jedarden.pdftract;

import java.util.List;

/**
 * Source from a remote URL.
 */
public record UrlSource(String url) implements Source {
    @Override
    public List<String> toArgs() {
        return List.of(url);
    }
}
