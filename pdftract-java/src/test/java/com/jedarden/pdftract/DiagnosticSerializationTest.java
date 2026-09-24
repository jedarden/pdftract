package com.jedarden.pdftract;

import com.fasterxml.jackson.databind.JsonNode;
import com.jedarden.pdftract.codegen.Json;
import com.jedarden.pdftract.codegen.ObjectLocation;
import com.jedarden.pdftract.codegen.ProcessingError;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

/** Pins the Java binding to the canonical structured diagnostic envelope. */
class DiagnosticSerializationTest {
    @Test
    void preservesCanonicalFieldsAndOmitsAbsentContext() throws Exception {
        String input = """
            {"code":"STREAM_DECODE_ERROR","message":"zlib stream truncated mid-inflation",
             "severity":"warning","page_index":3,
             "location":{"object_number":42,"generation_number":7},
             "hint":"Inspect the source PDF for corrupt stream data"}
            """.replaceAll("\\s+", "");

        ProcessingError diagnostic = Json.mapper().readValue(input, ProcessingError.class);
        assertEquals("STREAM_DECODE_ERROR", diagnostic.code());
        assertEquals("warning", diagnostic.severity());
        assertEquals(3, diagnostic.pageIndex());
        assertEquals(42, diagnostic.location().objectNumber());
        assertEquals(7, diagnostic.location().generationNumber());
        assertEquals("Inspect the source PDF for corrupt stream data", diagnostic.hint());

        JsonNode encoded = Json.mapper().readTree(Json.mapper().writeValueAsString(diagnostic));
        assertEquals(3, encoded.get("page_index").asInt());
        assertEquals(42, encoded.get("location").get("object_number").asInt());

        ProcessingError withoutContext = new ProcessingError(
            "info", "XREF_REPAIRED", "Xref was reconstructed via forward scan");
        JsonNode compact = Json.mapper().readTree(Json.mapper().writeValueAsString(withoutContext));
        assertFalse(compact.has("page_index"));
        assertFalse(compact.has("location"));
        assertFalse(compact.has("hint"));

        // Keep the location type directly constructible for binding callers.
        assertEquals(7, new ObjectLocation(42, 7).generationNumber());
    }
}
