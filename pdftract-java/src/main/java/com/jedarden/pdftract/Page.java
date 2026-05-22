package com.jedarden.pdftract;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

/**
 * A single page in the document.
 */
public record Page(
    @JsonProperty("page_index") int pageIndex,
    @JsonProperty("width") double width,
    @JsonProperty("height") double height,
    @JsonProperty("rotation") int rotation,
    @JsonProperty("page_type") String pageType,
    @JsonProperty("spans") List<Span> spans,
    @JsonProperty("blocks") List<Block> blocks
) {
    public Page {
        spans = spans != null ? spans : List.of();
        blocks = blocks != null ? blocks : List.of();
    }
}
