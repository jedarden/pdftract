//
//  PdftractTests.swift
//  PdftractTests
//
//  Unit tests for the Pdftract Swift SDK.
//

import XCTest
@testable import Pdftract

/// Test cases for Document model.
final class DocumentTests: XCTestCase {
    func testDocumentInitialization() {
        let metadata = Metadata(
            title: "Test Document",
            author: "Test Author",
            pageCount: 1
        )

        let document = Document(
            schemaVersion: "1.0",
            metadata: metadata,
            pages: []
        )

        XCTAssertEqual(document.schemaVersion, "1.0")
        XCTAssertEqual(document.metadata.title, "Test Document")
        XCTAssertEqual(document.metadata.author, "Test Author")
        XCTAssertEqual(document.metadata.pageCount, 1)
        XCTAssertTrue(document.pages.isEmpty)
    }

    func testDocumentJSONEncoding() throws {
        let metadata = Metadata(
            title: "Test",
            pageCount: 1
        )

        let document = Document(
            metadata: metadata,
            pages: [
                Page(
                    pageIndex: 0,
                    pageNumber: 1,
                    width: 612,
                    height: 792,
                    rotation: 0,
                    pageType: "text"
                )
            ]
        )

        let encoder = JSONEncoder()
        encoder.outputFormatting = .prettyPrinted
        let jsonData = try encoder.encode(document)
        let jsonString = String(data: jsonData, encoding: .utf8)!

        XCTAssertTrue(jsonString.contains("\"schema_version\" : \"1.0\""))
        XCTAssertTrue(jsonString.contains("\"page_count\" : 1"))
        XCTAssertTrue(jsonString.contains("\"page_index\" : 0"))
    }

    func testDocumentJSONDecoding() throws {
        let jsonString = """
        {
            "schema_version": "1.0",
            "metadata": {
                "page_count": 2,
                "is_tagged": false,
                "is_encrypted": false,
                "conformance": "none",
                "contains_javascript": false,
                "contains_xfa": false,
                "ocg_present": false,
                "javascript_actions": []
            },
            "outline": [],
            "threads": [],
            "attachments": [],
            "signatures": [],
            "form_fields": [],
            "links": [],
            "pages": [
                {
                    "page_index": 0,
                    "page_number": 1,
                    "width": 612.0,
                    "height": 792.0,
                    "rotation": 0,
                    "type": "text",
                    "spans": [],
                    "blocks": [],
                    "tables": [],
                    "annotations": []
                }
            ],
            "extraction_quality": {
                "overall_quality": "none"
            },
            "errors": []
        }
        """

        let decoder = JSONDecoder()
        let document = try decoder.decode(Document.self, from: jsonString.data(using: .utf8)!)

        XCTAssertEqual(document.schemaVersion, "1.0")
        XCTAssertEqual(document.metadata.pageCount, 2)
        XCTAssertEqual(document.pages.count, 1)
        XCTAssertEqual(document.pages[0].pageIndex, 0)
        XCTAssertEqual(document.pages[0].pageNumber, 1)
    }
}

/// Test cases for Page model.
final class PageTests: XCTestCase {
    func testPageInitialization() {
        let page = Page(
            pageIndex: 0,
            pageNumber: 1,
            pageLabel: "i",
            width: 612,
            height: 792,
            rotation: 0,
            pageType: "text"
        )

        XCTAssertEqual(page.pageIndex, 0)
        XCTAssertEqual(page.pageNumber, 1)
        XCTAssertEqual(page.pageLabel, "i")
        XCTAssertEqual(page.width, 612)
        XCTAssertEqual(page.height, 792)
        XCTAssertEqual(page.rotation, 0)
        XCTAssertEqual(page.pageType, "text")
    }

    func testSpanInitialization() {
        let span = Span(
            text: "Hello, World!",
            bbox: [100.0, 200.0, 300.0, 220.0],
            font: "Helvetica",
            size: 12.0,
            color: "#000000",
            lang: "en",
            flags: ["bold"]
        )

        XCTAssertEqual(span.text, "Hello, World!")
        XCTAssertEqual(span.bbox.count, 4)
        XCTAssertEqual(span.font, "Helvetica")
        XCTAssertEqual(span.size, 12.0)
        XCTAssertEqual(span.color, "#000000")
        XCTAssertEqual(span.lang, "en")
        XCTAssertEqual(span.flags.count, 1)
    }

    func testBlockInitialization() {
        let block = Block(
            kind: "paragraph",
            text: "This is a paragraph.",
            bbox: [72.0, 600.0, 540.0, 580.0],
            level: nil,
            tableIndex: nil,
            spans: [0, 1, 2]
        )

        XCTAssertEqual(block.kind, "paragraph")
        XCTAssertEqual(block.text, "This is a paragraph.")
        XCTAssertEqual(block.spans.count, 3)
    }

    func testHeadingBlockWithLevel() {
        let block = Block(
            kind: "heading",
            text: "Chapter 1",
            bbox: [72.0, 700.0, 540.0, 750.0],
            level: 1,
            tableIndex: nil,
            spans: []
        )

        XCTAssertEqual(block.kind, "heading")
        XCTAssertEqual(block.level, 1)
    }
}

/// Test cases for Table model.
final class TableTests: XCTestCase {
    func testTableInitialization() {
        let table = Table(
            id: "table_0",
            bbox: [50.0, 100.0, 550.0, 400.0],
            rows: [],
            headerRows: 1,
            detectionMethod: "line_based",
            pageIndex: 0
        )

        XCTAssertEqual(table.id, "table_0")
        XCTAssertEqual(table.headerRows, 1)
        XCTAssertEqual(table.detectionMethod, "line_based")
        XCTAssertEqual(table.pageIndex, 0)
    }

    func testCellInitialization() {
        let cell = Cell(
            bbox: [100.0, 400.0, 200.0, 380.0],
            text: "Cell content",
            spans: [0],
            row: 0,
            col: 0,
            rowspan: 1,
            colspan: 1,
            isHeaderRow: true
        )

        XCTAssertEqual(cell.text, "Cell content")
        XCTAssertEqual(cell.row, 0)
        XCTAssertEqual(cell.col, 0)
        XCTAssertEqual(cell.rowspan, 1)
        XCTAssertEqual(cell.colspan, 1)
        XCTAssertTrue(cell.isHeaderRow)
    }

    func testTableWithMergedCells() {
        let cell = Cell(
            bbox: [200.0, 100.0, 400.0, 150.0],
            text: "Merged cell",
            spans: [1],
            row: 0,
            col: 1,
            rowspan: 2,
            colspan: 2,
            isHeaderRow: false
        )

        XCTAssertEqual(cell.rowspan, 2)
        XCTAssertEqual(cell.colspan, 2)
    }
}

/// Test cases for Annotation model.
final class AnnotationTests: XCTestCase {
    func testLinkInitialization() {
        let link = Link(
            pageIndex: 0,
            rect: [100.0, 700.0, 200.0, 720.0],
            uri: "https://example.com"
        )

        XCTAssertEqual(link.pageIndex, 0)
        XCTAssertEqual(link.uri, "https://example.com")
        XCTAssertEqual(link.rect.count, 4)
    }

    func testInternalLink() {
        let link = Link(
            pageIndex: 0,
            rect: [100.0, 700.0, 200.0, 720.0],
            dest: "section1"
        )

        XCTAssertEqual(link.dest, "section1")
        XCTAssertNil(link.uri)
    }

    func testAnnotationInitialization() {
        let annotation = Annotation(
            subtype: "Highlight",
            rect: [100.0, 700.0, 200.0, 720.0],
            contents: "Important text",
            author: "Reviewer"
        )

        XCTAssertEqual(annotation.subtype, "Highlight")
        XCTAssertEqual(annotation.contents, "Important text")
        XCTAssertEqual(annotation.author, "Reviewer")
    }
}

/// Test cases for FormField model.
final class FormFieldTests: XCTestCase {
    func testTextField() {
        let field = FormField(
            name: "employee_name",
            fieldType: .text,
            value: .text("John Doe"),
            required: true,
            readOnly: false
        )

        XCTAssertEqual(field.name, "employee_name")
        XCTAssertEqual(field.fieldType, .text)
        XCTAssertEqual(field.value, .text("John Doe"))
        XCTAssertTrue(field.required)
        XCTAssertFalse(field.readOnly)
    }

    func testButtonField() {
        let field = FormField(
            name: "agree_checkbox",
            fieldType: .button,
            value: .button(true),
            selected: true
        )

        XCTAssertEqual(field.fieldType, .button)
        XCTAssertEqual(field.value, .button(true))
        XCTAssertTrue(field.selected ?? false)
    }

    func testChoiceFieldSingle() {
        let field = FormField(
            name: "department",
            fieldType: .choice,
            value: .choice(.single("Engineering")),
            options: [["engineering", "Engineering"], ["sales", "Sales"]]
        )

        XCTAssertEqual(field.fieldType, .choice)
        XCTAssertEqual(field.value, .choice(.single("Engineering")))
        XCTAssertEqual(field.options?.count, 2)
    }

    func testChoiceFieldMultiple() {
        let field = FormField(
            name: "skills",
            fieldType: .choice,
            value: .choice(.multiple(["Swift", "Python", "Rust"])),
            multiSelect: true
        )

        case .choice(.multiple(let skills)) = field.value
        XCTAssertEqual(skills.count, 3)
        XCTAssertTrue(field.multiSelect ?? false)
    }
}

/// Test cases for Signature model.
final class SignatureTests: XCTestCase {
    func testSignatureInitialization() {
        let signature = Signature(
            fieldName: "employer_sig",
            signerName: "John Doe",
            signingDate: "2023-01-15T14:30:45Z",
            reason: "Contract approval",
            location: "New York, NY",
            validationStatus: "not_checked"
        )

        XCTAssertEqual(signature.fieldName, "employer_sig")
        XCTAssertEqual(signature.signerName, "John Doe")
        XCTAssertEqual(signature.signingDate, "2023-01-15T14:30:45Z")
        XCTAssertEqual(signature.reason, "Contract approval")
        XCTAssertEqual(signature.location, "New York, NY")
        XCTAssertEqual(signature.validationStatus, "not_checked")
    }

    func testUnsignedSignature() {
        let signature = Signature(
            fieldName: "blank_sig",
            signerName: "",
            validationStatus: "not_checked"
        )

        XCTAssertEqual(signature.fieldName, "blank_sig")
        XCTAssertEqual(signature.signerName, "")
        XCTAssertNil(signature.signingDate)
    }
}

/// Test cases for Attachment model.
final class AttachmentTests: XCTestCase {
    func testAttachmentInitialization() {
        let attachment = Attachment(
            name: "contract.pdf",
            description: "Signed contract",
            mimeType: "application/pdf",
            size: 1024000,
            truncated: false
        )

        XCTAssertEqual(attachment.name, "contract.pdf")
        XCTAssertEqual(attachment.description, "Signed contract")
        XCTAssertEqual(attachment.mimeType, "application/pdf")
        XCTAssertEqual(attachment.size, 1024000)
        XCTAssertFalse(attachment.truncated)
    }

    func testTruncatedAttachment() {
        let attachment = Attachment(
            name: "large_file.bin",
            size: 52428801, // > 50 MB
            truncated: true
        )

        XCTAssertEqual(attachment.name, "large_file.bin")
        XCTAssertTrue(attachment.truncated)
        XCTAssertNil(attachment.data)
    }
}

/// Test cases for ExtractionQuality model.
final class ExtractionQualityTests: XCTestCase {
    func testQualityInitialization() {
        let quality = ExtractionQuality(
            overallQuality: "high",
            dpiUsed: 300,
            ocrFraction: 0.25,
            minConfidence: 0.95,
            avgConfidence: 0.98
        )

        XCTAssertEqual(quality.overallQuality, "high")
        XCTAssertEqual(quality.dpiUsed, 300)
        XCTAssertEqual(quality.ocrFraction, 0.25, accuracy: 0.001)
        XCTAssertEqual(quality.minConfidence, 0.95, accuracy: 0.001)
        XCTAssertEqual(quality.avgConfidence, 0.98, accuracy: 0.001)
    }

    func testDefaultQuality() {
        let quality = ExtractionQuality()
        XCTAssertEqual(quality.overallQuality, "none")
        XCTAssertNil(quality.dpiUsed)
    }
}

/// Test cases for Diagnostic model.
final class DiagnosticTests: XCTestCase {
    func testDiagnosticInitialization() {
        let diagnostic = Diagnostic(
            code: "FONT_GLYPH_UNMAPPED",
            message: "Glyph 0x20 not found in font encoding",
            severity: "warning",
            pageIndex: 0,
            hint: "Install missing font pack"
        )

        XCTAssertEqual(diagnostic.code, "FONT_GLYPH_UNMAPPED")
        XCTAssertEqual(diagnostic.severity, "warning")
        XCTAssertEqual(diagnostic.pageIndex, 0)
        XCTAssertEqual(diagnostic.hint, "Install missing font pack")
    }
}

/// Test cases for Source enum.
final class SourceTests: XCTestCase {
    func testPathSource() {
        let source = Source.path("/path/to/document.pdf")
        switch source {
        case .path(let path):
            XCTAssertEqual(path, "/path/to/document.pdf")
        default:
            XCTFail("Expected path source")
        }
    }

    func testUrlSource() {
        let source = Source.url("https://example.com/doc.pdf")
        switch source {
        case .url(let urlString):
            XCTAssertEqual(urlString, "https://example.com/doc.pdf")
        default:
            XCTFail("Expected URL source")
        }
    }

    func testBytesSource() {
        let data = Data("PDF content".utf8)
        let source = Source.bytes(data)
        switch source {
        case .bytes(let bytes):
            XCTAssertEqual(bytes, data)
        default:
            XCTFail("Expected bytes source")
        }
    }
}

/// Test cases for ExtractionOptions.
final class ExtractionOptionsTests: XCTestCase {
    func testDefaultOptions() {
        let options = ExtractionOptions()
        XCTAssertTrue(options.extractSpans)
        XCTAssertTrue(options.extractBlocks)
        XCTAssertTrue(options.extractTables)
        XCTAssertTrue(options.extractAnnotations)
        XCTAssertTrue(options.extractFormFields)
        XCTAssertTrue(options.extractSignatures)
        XCTAssertTrue(options.extractAttachments)
        XCTAssertTrue(options.extractOutline)
        XCTAssertTrue(options.extractThreads)
        XCTAssertTrue(options.extractLinks)
        XCTAssertNil(options.ocrDpi)
        XCTAssertTrue(options.includeQuality)
        XCTAssertTrue(options.includeErrors)
    }

    func testCustomOptions() {
        let options = ExtractionOptions(
            extractSpans: false,
            extractTables: false,
            ocrDpi: 400,
            maxAttachmentSize: 10_000_000
        )

        XCTAssertFalse(options.extractSpans)
        XCTAssertFalse(options.extractTables)
        XCTAssertEqual(options.ocrDpi, 400)
        XCTAssertEqual(options.maxAttachmentSize, 10_000_000)
    }
}

/// Test cases for PdftractError.
final class ErrorTests: XCTestCase {
    func testErrorDescriptions() {
        let errors: [PdftractError] = [
            .invalidPdf("Not a PDF file"),
            .ioError("File not found"),
            .networkError("Connection refused"),
            .outOfMemory,
            .parseError("Invalid xref table"),
            .ocrError("Tesseract not found"),
            .renderingError("Cannot render page"),
            .internalError("Unknown failure")
        ]

        for error in errors {
            XCTAssertFalse(error.localizedDescription.isEmpty)
        }

        XCTAssertEqual(PdftractError.invalidPdf("test").code, "INVALID_PDF")
        XCTAssertEqual(PdftractError.ioError("test").code, "IO_ERROR")
    }

    func testErrorEquality() {
        XCTAssertEqual(PdftractError.invalidPdf("test"), PdftractError.invalidPdf("test"))
        XCTAssertNotEqual(PdftractError.invalidPdf("a"), PdftractError.invalidPdf("b"))
        XCTAssertEqual(PdftractError.outOfMemory, PdftractError.outOfMemory)
    }
}
