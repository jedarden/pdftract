package com.jedarden.pdftract.codegen;

import com.fasterxml.jackson.annotation.JsonProperty;
import com.fasterxml.jackson.annotation.JsonInclude;

/**
 * Canonical structured extraction diagnostic.
 *
 * The full JSON {@code errors} array and NDJSON footer use this object. The
 * compact Rust metadata API also keeps its legacy {@code Vec<String>} message
 * array; callers that need typed context should use this model instead.
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
public record ProcessingError(
    @JsonProperty("severity") String severity,
    @JsonProperty("code") String code,
    @JsonProperty("message") String message,
    @JsonProperty("page_index") Integer pageIndex,
    @JsonProperty("location") ObjectLocation location,
    @JsonProperty("hint") String hint
) {
    /** Preserve the original three-argument binding constructor. */
    public ProcessingError(String severity, String code, String message) {
        this(severity, code, message, null, null, null);
    }
}
