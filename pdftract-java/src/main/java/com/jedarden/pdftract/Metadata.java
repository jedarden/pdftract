package com.jedarden.pdftract;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Document metadata.
 */
public record Metadata(
    @JsonProperty("page_count") int pageCount,
    @JsonProperty("title") String title,
    @JsonProperty("author") String author,
    @JsonProperty("creator") String creator,
    @JsonProperty("has_xmp") Boolean hasXmp
) {}
