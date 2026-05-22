package com.jedarden.pdftract.codegen;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Processing error information.
 */
public record ProcessingError(
    @JsonProperty("severity") String severity,
    @JsonProperty("code") String code,
    @JsonProperty("message") String message
) {}
