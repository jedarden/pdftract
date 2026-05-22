package com.jedarden.pdftract;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

/**
 * A text span with font and position information.
 */
public record Span(
    @JsonProperty("text") String text,
    @JsonProperty("font") String font,
    @JsonProperty("size") Double size,
    @JsonProperty("bbox") List<Double> bbox
) {
    public Span {
        bbox = bbox != null ? bbox : List.of();
    }
}
