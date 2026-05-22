package com.jedarden.pdftract.codegen;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Receipt data for verification.
 */
public record Receipt(
    @JsonProperty("fingerprint") String fingerprint,
    @JsonProperty("signature") String signature
) {}
