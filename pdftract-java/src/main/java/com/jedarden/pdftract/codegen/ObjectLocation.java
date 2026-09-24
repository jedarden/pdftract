package com.jedarden.pdftract.codegen;

import com.fasterxml.jackson.annotation.JsonProperty;

/** PDF indirect-object location attached to a structured diagnostic. */
public record ObjectLocation(
    @JsonProperty("object_number") int objectNumber,
    @JsonProperty("generation_number") int generationNumber
) {}
