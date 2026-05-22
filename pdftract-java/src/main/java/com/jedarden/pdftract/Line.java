package com.jedarden.pdftract;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

/**
 * A line within a block, referencing span indices.
 */
public record Line(
    @JsonProperty("spans") List<Integer> spans
) {
    public Line {
        spans = spans != null ? spans : List.of();
    }
}
