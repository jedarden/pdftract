package pdftract

import (
	"encoding/json"
	"testing"
)

func TestDiagnosticJSONRoundTrip(t *testing.T) {
	var document Document
	if err := json.Unmarshal([]byte(`{
		"errors":[{"code":"STREAM_DECODE_ERROR","message":"zlib stream truncated mid-inflation","severity":"warning","page_index":3,"location":{"object_number":42,"generation_number":7},"hint":"Inspect the source PDF for corrupt stream data"}]
	}`), &document); err != nil {
		t.Fatal(err)
	}

	diagnostic := document.Errors[0]
	if diagnostic.Code != "STREAM_DECODE_ERROR" || diagnostic.Severity != "warning" {
		t.Fatalf("unexpected diagnostic identity: %#v", diagnostic)
	}
	if diagnostic.PageIndex == nil || *diagnostic.PageIndex != 3 {
		t.Fatalf("unexpected page index: %#v", diagnostic.PageIndex)
	}
	if diagnostic.Location == nil || diagnostic.Location.ObjectNumber != 42 || diagnostic.Location.GenerationNumber != 7 {
		t.Fatalf("unexpected location: %#v", diagnostic.Location)
	}
	if diagnostic.Hint == nil || *diagnostic.Hint == "" {
		t.Fatal("diagnostic hint was not preserved")
	}

	encoded, err := json.Marshal(Diagnostic{Code: "XREF_REPAIRED", Message: "repaired", Severity: "info"})
	if err != nil {
		t.Fatal(err)
	}
	if string(encoded) != `{"code":"XREF_REPAIRED","message":"repaired","severity":"info"}` {
		t.Fatalf("optional fields should be omitted: %s", encoded)
	}
}
