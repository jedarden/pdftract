package com.jedarden.pdftract.codegen;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

/**
 * Classification result for a PDF document.
 */
public record Classification(
    @JsonProperty("category") String category,
    @JsonProperty("confidence") double confidence,
    @JsonProperty("labels") List<String> labels
) {
    public Classification {
        labels = labels != null ? labels : List.of();
    }
}
