package com.jedarden.pdftract.codegen;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.json.JsonMapper;
import com.fasterxml.jackson.databind.DeserializationFeature;

/**
 * ObjectMapper configured for pdftract JSON output.
 * Fails on unknown properties to catch schema changes early.
 */
public class Json {
    private static final ObjectMapper mapper = JsonMapper.builder()
        .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, true)
        .build()
        .setSerializationInclusion(JsonInclude.Include.NON_NULL);

    public static ObjectMapper mapper() {
        return mapper;
    }
}
