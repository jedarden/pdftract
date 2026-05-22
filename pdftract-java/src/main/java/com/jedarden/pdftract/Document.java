package com.jedarden.pdftract;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.jedarden.pdftract.codegen.ProcessingError;
import java.util.List;

/**
 * Complete document extraction result.
 */
public record Document(
    @JsonProperty("schema_version") String schemaVersion,
    @JsonProperty("metadata") DocumentMetadata metadata,
    @JsonProperty("pages") List<Page> pages,
    @JsonProperty("errors") List<ProcessingError> errors
) {
    public Document {
        metadata = metadata != null ? metadata : new DocumentMetadata(null, false, null, null, null);
        pages = pages != null ? pages : List.of();
        errors = errors != null ? errors : List.of();
    }
}
