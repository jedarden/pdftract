package com.jedarden.pdftract;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

/**
 * A semantic block (paragraph, heading, table, etc.).
 */
public record Block(
    @JsonProperty("kind") String kind,
    @JsonProperty("bbox") List<Double> bbox,
    @JsonProperty("lines") List<Line> lines
) {
    public Block {
        bbox = bbox != null ? bbox : List.of();
        lines = lines != null ? lines : List.of();
    }
}
