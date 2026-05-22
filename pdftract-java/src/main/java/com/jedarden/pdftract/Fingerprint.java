package com.jedarden.pdftract;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Document fingerprint for verification.
 */
public record Fingerprint(
    @JsonProperty("hash") String hash,
    @JsonProperty("fast_hash") String fastHash,
    @JsonProperty("page_count") int pageCount,
    @JsonProperty("is_encrypted") Boolean isEncrypted
) {}
