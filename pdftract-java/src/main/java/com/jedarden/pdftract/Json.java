package com.jedarden.pdftract;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.json.JsonMapper;

/**
 * ObjectMapper configured for pdftract JSON output.
 */
public class Json {
    private static final ObjectMapper mapper = JsonMapper.builder()
        .build();

    public static ObjectMapper mapper() {
        return mapper;
    }
}
