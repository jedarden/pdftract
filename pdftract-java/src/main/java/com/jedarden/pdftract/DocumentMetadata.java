package com.jedarden.pdftract;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Document metadata from PDF info dictionary.
 */
public record DocumentMetadata(
    @JsonProperty("page_count") Integer pageCount,
    @JsonProperty("is_encrypted") Boolean isEncrypted,
    @JsonProperty("title") String title,
    @JsonProperty("author") String author,
    @JsonProperty("creator") String creator
) {}
