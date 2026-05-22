package com.jedarden.pdftract;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

/**
 * A search match result.
 */
public record Match(
    @JsonProperty("page") int page,
    @JsonProperty("text") String text,
    @JsonProperty("bbox") List<Double> bbox
) {
    public Match {
        bbox = bbox != null ? bbox : List.of();
    }
}
