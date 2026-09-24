import XCTest
@testable import Pdftract

final class DiagnosticSerializationTests: XCTestCase {
    func testDiagnosticPreservesCanonicalFieldsAndOmitsAbsentContext() throws {
        let input = """
        {"code":"STREAM_DECODE_ERROR","message":"zlib stream truncated mid-inflation","severity":"warning","page_index":3,"location":{"object_number":42,"generation_number":7},"hint":"Inspect the source PDF for corrupt stream data"}
        """.data(using: .utf8)!

        let diagnostic = try JSONDecoder().decode(Diagnostic.self, from: input)
        XCTAssertEqual(diagnostic.code, "STREAM_DECODE_ERROR")
        XCTAssertEqual(diagnostic.severity, "warning")
        XCTAssertEqual(diagnostic.pageIndex, 3)
        XCTAssertEqual(diagnostic.location?.objectNumber, 42)
        XCTAssertEqual(diagnostic.location?.generationNumber, 7)
        XCTAssertEqual(diagnostic.hint, "Inspect the source PDF for corrupt stream data")

        let encoded = try JSONSerialization.jsonObject(
            with: JSONEncoder().encode(diagnostic)) as! [String: Any]
        XCTAssertEqual(encoded["page_index"] as? Int, 3)
        XCTAssertEqual((encoded["location"] as? [String: Any])?["object_number"] as? Int, 42)

        let bare = Diagnostic(code: "XREF_REPAIRED", message: "repaired", severity: "info")
        let bareObject = try JSONSerialization.jsonObject(
            with: JSONEncoder().encode(bare)) as! [String: Any]
        XCTAssertNil(bareObject["page_index"])
        XCTAssertNil(bareObject["location"])
        XCTAssertNil(bareObject["hint"])
    }
}
