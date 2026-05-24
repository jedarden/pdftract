//! PDF fixture generator for xref testing.
//!
//! This tool generates minimal PDF files with specific xref structures
//! for testing the pdftract xref resolver.

use std::fs::File;
use std::io::{BufWriter, Write, Seek};
use std::path::PathBuf;
use std::process;

/// PDF fixture type.
#[derive(Debug, Clone, Copy)]
enum FixtureType {
    /// Well-formed PDF with traditional xref table.
    WellFormedTraditional,
    /// Well-formed PDF with xref stream (PDF 1.5).
    WellFormedStream,
    /// Hybrid file with traditional xref + /XRefStm.
    HybridFile,
    /// PDF with 3 incremental revisions (/Prev chain).
    PrevChain3Revisions,
    /// Linearized PDF (50 pages).
    Linearized,
    /// File truncated at the start of xref.
    TruncatedAfterXref,
    /// File with startxref offset off by one.
    StartxrefOffByOne,
    /// File with one corrupt xref entry.
    CorruptXrefEntry,
    /// File with circular /Prev reference.
    CircularPrev,
    /// File with 50 incremental revisions (tests depth limit).
    DeepPrevChain,
}

impl FixtureType {
    fn name(&self) -> &'static str {
        match self {
            Self::WellFormedTraditional => "well_formed_traditional.pdf",
            Self::WellFormedStream => "well_formed_stream.pdf",
            Self::HybridFile => "hybrid_file.pdf",
            Self::PrevChain3Revisions => "prev_chain_3_revisions.pdf",
            Self::Linearized => "linearized.pdf",
            Self::TruncatedAfterXref => "truncated_after_xref.pdf",
            Self::StartxrefOffByOne => "startxref_off_by_one.pdf",
            Self::CorruptXrefEntry => "corrupt_xref_entry.pdf",
            Self::CircularPrev => "circular_prev.pdf",
            Self::DeepPrevChain => "deep_prev_chain.pdf",
        }
    }
}

/// Fixture generator context.
struct Generator {
    output_dir: PathBuf,
}

impl Generator {
    fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    /// Generate a single fixture.
    fn generate(&self, fixture_type: FixtureType) {
        let filename = PathBuf::from(fixture_type.name());
        let output_path = self.output_dir.join(filename);

        match fixture_type {
            FixtureType::WellFormedTraditional => {
                self.generate_well_formed_traditional(&output_path);
            }
            FixtureType::WellFormedStream => {
                self.generate_well_formed_stream(&output_path);
            }
            FixtureType::HybridFile => {
                self.generate_hybrid_file(&output_path);
            }
            FixtureType::PrevChain3Revisions => {
                self.generate_prev_chain_3(&output_path);
            }
            FixtureType::Linearized => {
                self.generate_linearized(&output_path);
            }
            FixtureType::TruncatedAfterXref => {
                // Start with well-formed, then truncate
                let base_path = self.output_dir.join(FixtureType::WellFormedTraditional.name());
                self.generate_truncated(&base_path, &output_path);
            }
            FixtureType::StartxrefOffByOne => {
                // Start with well-formed, then modify startxref
                let base_path = self.output_dir.join(FixtureType::WellFormedTraditional.name());
                self.generate_startxref_off_by_one(&base_path, &output_path);
            }
            FixtureType::CorruptXrefEntry => {
                // Start with well-formed, then corrupt one entry
                let base_path = self.output_dir.join(FixtureType::WellFormedTraditional.name());
                self.generate_corrupt_entry(&base_path, &output_path);
            }
            FixtureType::CircularPrev => {
                self.generate_circular_prev(&output_path);
            }
            FixtureType::DeepPrevChain => {
                self.generate_deep_prev_chain(&output_path);
            }
        }

        println!("Generated: {:?}", output_path);
    }

    /// Generate a well-formed PDF with traditional xref table.
    fn generate_well_formed_traditional(&self, output_path: &PathBuf) {
        let file = File::create(output_path).unwrap_or_else(|e| {
            panic!("Failed to create {:?}: {}", output_path, e);
        });
        let mut w = BufWriter::new(file);

        // PDF header
        writeln!(w, "%PDF-1.4").unwrap();

        // Object 1: Catalog
        writeln!(w, "1 0 obj").unwrap();
        writeln!(w, "<< /Type /Catalog").unwrap();
        writeln!(w, "   /Pages 2 0 R").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 2: Page tree root
        writeln!(w, "2 0 obj").unwrap();
        writeln!(w, "<< /Type /Pages").unwrap();
        writeln!(w, "   /Kids [3 0 R]").unwrap();
        writeln!(w, "   /Count 1").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 3: Page
        writeln!(w, "3 0 obj").unwrap();
        writeln!(w, "<< /Type /Page").unwrap();
        writeln!(w, "   /Parent 2 0 R").unwrap();
        writeln!(w, "   /MediaBox [0 0 612 792]").unwrap();
        writeln!(w, "   /Resources << /Font << >> >>").unwrap();
        writeln!(w, "   /Contents 4 0 R").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 4: Contents (empty stream)
        writeln!(w, "4 0 obj").unwrap();
        writeln!(w, "<< /Length 0 >>").unwrap();
        writeln!(w, "stream").unwrap();
        writeln!(w, "endstream").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 5: Info
        writeln!(w, "5 0 obj").unwrap();
        writeln!(w, "<< /Title (Test Document)").unwrap();
        writeln!(w, "   /Producer (build-xref-fixture)").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Track xref offset
        let xref_offset = w.stream_position().unwrap();

        // Traditional xref table
        writeln!(w, "xref").unwrap();
        writeln!(w, "0 6").unwrap();
        writeln!(w, "0000000000 65535 f ").unwrap();
        writeln!(w, "0000000017 00000 n ").unwrap();  // Object 1
        writeln!(w, "0000000082 00000 n ").unwrap();  // Object 2
        writeln!(w, "0000000160 00000 n ").unwrap(); // Object 3
        writeln!(w, "0000000269 00000 n ").unwrap(); // Object 4
        writeln!(w, "0000000341 00000 n ").unwrap(); // Object 5

        // Trailer
        writeln!(w, "trailer").unwrap();
        writeln!(w, "<< /Size 6").unwrap();
        writeln!(w, "   /Root 1 0 R").unwrap();
        writeln!(w, "   /Info 5 0 R").unwrap();
        writeln!(w, ">>").unwrap();

        // startxref
        writeln!(w, "startxref").unwrap();
        writeln!(w, "{}", xref_offset).unwrap();

        // EOF
        writeln!(w, "%%EOF").unwrap();

        w.flush().unwrap();
    }

    /// Generate a well-formed PDF with xref stream (PDF 1.5).
    fn generate_well_formed_stream(&self, output_path: &PathBuf) {
        let file = File::create(output_path).unwrap_or_else(|e| {
            panic!("Failed to create {:?}: {}", output_path, e);
        });
        let mut w = BufWriter::new(file);

        // PDF header (1.5 for xref stream support)
        writeln!(w, "%PDF-1.5").unwrap();

        // Object 1: Catalog
        writeln!(w, "1 0 obj").unwrap();
        writeln!(w, "<< /Type /Catalog").unwrap();
        writeln!(w, "   /Pages 2 0 R").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 2: Page tree root
        writeln!(w, "2 0 obj").unwrap();
        writeln!(w, "<< /Type /Pages").unwrap();
        writeln!(w, "   /Kids [3 0 R]").unwrap();
        writeln!(w, "   /Count 1").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 3: Page
        writeln!(w, "3 0 obj").unwrap();
        writeln!(w, "<< /Type /Page").unwrap();
        writeln!(w, "   /Parent 2 0 R").unwrap();
        writeln!(w, "   /MediaBox [0 0 612 792]").unwrap();
        writeln!(w, "   /Resources << /Font << >> >>").unwrap();
        writeln!(w, "   /Contents 4 0 R").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 4: Contents (empty stream)
        writeln!(w, "4 0 obj").unwrap();
        writeln!(w, "<< /Length 0 >>").unwrap();
        writeln!(w, "stream").unwrap();
        writeln!(w, "endstream").unwrap();
        writeln!(w, "endobj").unwrap();

        // Track xref stream offset
        let xref_stream_offset = w.stream_position().unwrap();

        // Object 5: XRef stream
        // /W = [1 4 2] means: type=1 byte, offset=4 bytes, gen=2 bytes
        writeln!(w, "5 0 obj").unwrap();
        writeln!(w, "<< /Type /XRef").unwrap();
        writeln!(w, "   /Size 6").unwrap();
        writeln!(w, "   /W [1 4 2]").unwrap();
        writeln!(w, "   /Index [0 6]").unwrap();
        writeln!(w, "   /Root 1 0 R").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "stream").unwrap();

        // Xref stream data:
        // Entry 0: type 0 (free), next_free=0, gen=65535
        // Entry 1: type 1 (in-use), offset=17, gen=0
        // Entry 2: type 1 (in-use), offset=82, gen=0
        // Entry 3: type 1 (in-use), offset=160, gen=0
        // Entry 4: type 1 (in-use), offset=269, gen=0
        // Entry 5: type 1 (in-use), offset=348, gen=0
        let xref_data = [
            // Type=1 byte, Offset=4 bytes (big-endian), Gen=2 bytes (big-endian)
            0u8, 0, 0, 0, 0, 255, 255,     // Entry 0: free
            1, 0, 0, 0, 17, 0, 0,           // Entry 1: in-use at offset 17
            1, 0, 0, 0, 82, 0, 0,           // Entry 2: in-use at offset 82
            1, 0, 0, 0, 160, 0, 0,          // Entry 3: in-use at offset 160
            1, 0, 0, 1, 13, 0, 0,           // Entry 4: in-use at offset 269
            1, 0, 0, 1, 92, 0, 0,           // Entry 5: in-use at offset 348 (this stream itself)
        ];

        w.write_all(&xref_data).unwrap();
        writeln!(w, "\nendstream").unwrap();
        writeln!(w, "endobj").unwrap();

        // startxref
        writeln!(w, "startxref").unwrap();
        writeln!(w, "{}", xref_stream_offset).unwrap();

        // EOF
        writeln!(w, "%%EOF").unwrap();

        w.flush().unwrap();
    }

    /// Generate a hybrid file with traditional xref + /XRefStm.
    fn generate_hybrid_file(&self, output_path: &PathBuf) {
        let file = File::create(output_path).unwrap_or_else(|e| {
            panic!("Failed to create {:?}: {}", output_path, e);
        });
        let mut w = BufWriter::new(file);

        // PDF header (1.5 for hybrid support)
        writeln!(w, "%PDF-1.5").unwrap();

        // Object 1: Catalog
        writeln!(w, "1 0 obj").unwrap();
        writeln!(w, "<< /Type /Catalog").unwrap();
        writeln!(w, "   /Pages 2 0 R").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 2: Page tree root
        writeln!(w, "2 0 obj").unwrap();
        writeln!(w, "<< /Type /Pages").unwrap();
        writeln!(w, "   /Kids [3 0 R]").unwrap();
        writeln!(w, "   /Count 1").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 3: Page
        writeln!(w, "3 0 obj").unwrap();
        writeln!(w, "<< /Type /Page").unwrap();
        writeln!(w, "   /Parent 2 0 R").unwrap();
        writeln!(w, "   /MediaBox [0 0 612 792]").unwrap();
        writeln!(w, "   /Resources << /Font << >> >>").unwrap();
        writeln!(w, "   /Contents 4 0 R").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 4: Contents (empty stream)
        writeln!(w, "4 0 obj").unwrap();
        writeln!(w, "<< /Length 0 >>").unwrap();
        writeln!(w, "stream").unwrap();
        writeln!(w, "endstream").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 5: XRef stream (will be referenced from /XRefStm)
        writeln!(w, "5 0 obj").unwrap();
        writeln!(w, "<< /Type /XRef").unwrap();
        writeln!(w, "   /Size 7").unwrap();
        writeln!(w, "   /W [1 4 2]").unwrap();
        writeln!(w, "   /Index [0 7]").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "stream").unwrap();

        // Xref stream data with one overlapping entry (object 6)
        let xref_data = [
            0u8, 0, 0, 0, 0, 255, 255,     // Entry 0: free
            0, 0, 0, 0, 0, 0, 0,            // Entry 1: free (overlaps traditional)
            0, 0, 0, 0, 0, 0, 0,            // Entry 2: free
            0, 0, 0, 0, 0, 0, 0,            // Entry 3: free
            0, 0, 0, 0, 0, 0, 0,            // Entry 4: free
            0, 0, 0, 0, 0, 0, 0,            // Entry 5: free
            1, 0, 0, 1, 244, 0, 0,          // Entry 6: new object in stream only (offset 500)
        ];

        w.write_all(&xref_data).unwrap();
        writeln!(w, "\nendstream").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 6: Additional object (only in xref stream)
        writeln!(w, "6 0 obj").unwrap();
        writeln!(w, "(Additional object)").unwrap();
        writeln!(w, "endobj").unwrap();

        // Track xref offset
        let xref_offset = w.stream_position().unwrap();

        // Traditional xref table (covers objects 0-5)
        writeln!(w, "xref").unwrap();
        writeln!(w, "0 6").unwrap();
        writeln!(w, "0000000000 65535 f ").unwrap();
        writeln!(w, "0000000017 00000 n ").unwrap();  // Object 1 (overlaps with stream's free entry)
        writeln!(w, "0000000082 00000 n ").unwrap();  // Object 2
        writeln!(w, "0000000160 00000 n ").unwrap(); // Object 3
        writeln!(w, "0000000269 00000 n ").unwrap(); // Object 4
        writeln!(w, "0000000341 00000 n ").unwrap(); // Object 5

        // Trailer with /XRefStm
        writeln!(w, "trailer").unwrap();
        writeln!(w, "<< /Size 7").unwrap();
        writeln!(w, "   /Root 1 0 R").unwrap();
        writeln!(w, "   /XRefStm 341").unwrap();  // Points to object 5 (xref stream)
        writeln!(w, ">>").unwrap();

        // startxref
        writeln!(w, "startxref").unwrap();
        writeln!(w, "{}", xref_offset).unwrap();

        // EOF
        writeln!(w, "%%EOF").unwrap();

        w.flush().unwrap();
    }

    /// Generate a PDF with 3 incremental revisions.
    fn generate_prev_chain_3(&self, output_path: &PathBuf) {
        let file = File::create(output_path).unwrap_or_else(|e| {
            panic!("Failed to create {:?}: {}", output_path, e);
        });
        let mut w = BufWriter::new(file);

        // PDF header
        writeln!(w, "%PDF-1.4").unwrap();

        // === Revision 1 (baseline) ===

        // Object 1: Catalog
        writeln!(w, "1 0 obj").unwrap();
        writeln!(w, "<< /Type /Catalog").unwrap();
        writeln!(w, "   /Pages 2 0 R").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 2: Page tree root
        writeln!(w, "2 0 obj").unwrap();
        writeln!(w, "<< /Type /Pages").unwrap();
        writeln!(w, "   /Kids [3 0 R]").unwrap();
        writeln!(w, "   /Count 1").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 3: Page
        writeln!(w, "3 0 obj").unwrap();
        writeln!(w, "<< /Type /Page").unwrap();
        writeln!(w, "   /Parent 2 0 R").unwrap();
        writeln!(w, "   /MediaBox [0 0 612 792]").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 4: Info
        writeln!(w, "4 0 obj").unwrap();
        writeln!(w, "<< /Title (Revision 1)>>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 5: Will be modified in revision 2
        writeln!(w, "5 0 obj").unwrap();
        writeln!(w, "(Original value)").unwrap();
        writeln!(w, "endobj").unwrap();

        let xref1_offset = w.stream_position().unwrap();

        // First xref + trailer
        writeln!(w, "xref").unwrap();
        writeln!(w, "0 6").unwrap();
        writeln!(w, "0000000000 65535 f ").unwrap();
        writeln!(w, "0000000017 00000 n ").unwrap();
        writeln!(w, "0000000082 00000 n ").unwrap();
        writeln!(w, "0000000160 00000 n ").unwrap();
        writeln!(w, "0000000249 00000 n ").unwrap();
        writeln!(w, "0000000290 00000 n ").unwrap();

        writeln!(w, "trailer").unwrap();
        writeln!(w, "<< /Size 6").unwrap();
        writeln!(w, "   /Root 1 0 R").unwrap();
        writeln!(w, ">>").unwrap();

        writeln!(w, "startxref").unwrap();
        writeln!(w, "{}", xref1_offset).unwrap();
        writeln!(w, "%%EOF").unwrap();

        // === Revision 2 (incremental update) ===

        // Modify object 5
        writeln!(w, "5 1 obj").unwrap();
        writeln!(w, "(Modified in revision 2)").unwrap();
        writeln!(w, "endobj").unwrap();

        // Add object 6
        writeln!(w, "6 0 obj").unwrap();
        writeln!(w, "(Added in revision 2)").unwrap();
        writeln!(w, "endobj").unwrap();

        let xref2_offset = w.stream_position().unwrap();

        // Second xref + trailer with /Prev
        writeln!(w, "xref").unwrap();
        writeln!(w, "5 2").unwrap();
        writeln!(w, "0000000341 00001 n ").unwrap();  // Object 5, gen 1
        writeln!(w, "0000000382 00000 n ").unwrap();  // Object 6, gen 0

        writeln!(w, "trailer").unwrap();
        writeln!(w, "<< /Size 7").unwrap();
        writeln!(w, "   /Root 1 0 R").unwrap();
        writeln!(w, "   /Prev {}", xref1_offset).unwrap();
        writeln!(w, ">>").unwrap();

        writeln!(w, "startxref").unwrap();
        writeln!(w, "{}", xref2_offset).unwrap();
        writeln!(w, "%%EOF").unwrap();

        // === Revision 3 (another incremental update) ===

        // Modify object 5 again
        writeln!(w, "5 2 obj").unwrap();
        writeln!(w, "(Modified in revision 3)").unwrap();
        writeln!(w, "endobj").unwrap();

        let xref3_offset = w.stream_position().unwrap();

        // Third xref + trailer with /Prev
        writeln!(w, "xref").unwrap();
        writeln!(w, "5 1").unwrap();
        writeln!(w, "0000000433 00002 n ").unwrap();  // Object 5, gen 2

        writeln!(w, "trailer").unwrap();
        writeln!(w, "<< /Size 7").unwrap();
        writeln!(w, "   /Root 1 0 R").unwrap();
        writeln!(w, "   /Prev {}", xref2_offset).unwrap();
        writeln!(w, ">>").unwrap();

        writeln!(w, "startxref").unwrap();
        writeln!(w, "{}", xref3_offset).unwrap();
        writeln!(w, "%%EOF").unwrap();

        w.flush().unwrap();
    }

    /// Generate a linearized PDF (50 pages).
    fn generate_linearized(&self, output_path: &PathBuf) {
        let file = File::create(output_path).unwrap_or_else(|e| {
            panic!("Failed to create {:?}: {}", output_path, e);
        });
        let mut w = BufWriter::new(file);

        // PDF header
        writeln!(w, "%PDF-1.4").unwrap();

        let _lin_dict_offset = w.stream_position().unwrap();

        // Linearized dictionary (object 1)
        writeln!(w, "1 0 obj").unwrap();
        writeln!(w, "<< /Linearized 1.0").unwrap();
        writeln!(w, "   /L 10000").unwrap();  // Placeholder file length
        writeln!(w, "   /H [1010 50]").unwrap();  // Hint stream offset/length
        writeln!(w, "   /O 4").unwrap();  // First page object number
        writeln!(w, "   /E 500").unwrap();  // End of first page
        writeln!(w, "   /N 50").unwrap();  // Number of pages
        writeln!(w, "   /T 6000").unwrap();  // Offset of first-page xref
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 2: First-page xref (partial, for linearized viewing)
        writeln!(w, "2 0 obj").unwrap();
        writeln!(w, "<< /Type /XRef").unwrap();
        writeln!(w, "   /Size 6").unwrap();
        writeln!(w, "   /W [1 4 2]").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "stream").unwrap();
        // Minimal xref data for first page objects
        let first_page_xref = [
            0u8, 0, 0, 0, 0, 255, 255,
            1, 0, 0, 0, 17, 0, 0,
            1, 0, 0, 0, 120, 0, 0,
            1, 0, 0, 0, 210, 0, 0,
            1, 0, 0, 1, 44, 0, 0,
        ];
        w.write_all(&first_page_xref).unwrap();
        writeln!(w, "\nendstream").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 3: Hint stream
        writeln!(w, "3 0 obj").unwrap();
        writeln!(w, "<< /Length 0 >>").unwrap();
        writeln!(w, "stream").unwrap();
        writeln!(w, "endstream").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 4: First page
        writeln!(w, "4 0 obj").unwrap();
        writeln!(w, "<< /Type /Page").unwrap();
        writeln!(w, "   /MediaBox [0 0 612 792]").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Object 5: Catalog
        writeln!(w, "5 0 obj").unwrap();
        writeln!(w, "<< /Type /Catalog").unwrap();
        writeln!(w, "   /Pages 6 0 R").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Placeholder for remaining pages...
        for i in 6..60 {
            writeln!(w, "{} 0 obj", i).unwrap();
            writeln!(w, "(Page {})", i).unwrap();
            writeln!(w, "endobj").unwrap();
        }

        // Full xref at EOF (placeholder offset)
        let full_xref_offset = w.stream_position().unwrap();

        writeln!(w, "xref").unwrap();
        writeln!(w, "0 60").unwrap();
        writeln!(w, "0000000000 65535 f ").unwrap();
        for i in 1..60 {
            writeln!(w, "0000000{} 00000 n ", i).unwrap();
        }

        writeln!(w, "trailer").unwrap();
        writeln!(w, "<< /Size 60").unwrap();
        writeln!(w, "   /Root 5 0 R").unwrap();
        writeln!(w, ">>").unwrap();

        writeln!(w, "startxref").unwrap();
        writeln!(w, "{}", full_xref_offset).unwrap();
        writeln!(w, "%%EOF").unwrap();

        w.flush().unwrap();
    }

    /// Generate a truncated file from a base file.
    fn generate_truncated(&self, base_path: &PathBuf, output_path: &PathBuf) {
        // Read base file
        let base_data = std::fs::read(base_path).unwrap_or_else(|e| {
            panic!("Failed to read base file {:?}: {}", base_path, e);
        });

        // Find the xref keyword
        let xref_pos = base_data.windows(4).rposition(|w| w == b"xref")
            .expect("xref keyword not found in base file");

        // Truncate just before the xref table
        let truncated_len = xref_pos;

        let file = File::create(output_path).unwrap_or_else(|e| {
            panic!("Failed to create {:?}: {}", output_path, e);
        });
        let mut w = BufWriter::new(file);

        w.write_all(&base_data[..truncated_len]).unwrap();
        w.flush().unwrap();
    }

    /// Generate a file with startxref offset off by one.
    fn generate_startxref_off_by_one(&self, base_path: &PathBuf, output_path: &PathBuf) {
        // Read base file
        let base_data = std::fs::read(base_path).unwrap_or_else(|e| {
            panic!("Failed to read base file {:?}: {}", base_path, e);
        });

        // Find "startxref" and modify the offset after it
        let startxref_pos = base_data.windows(9).rposition(|w| w == b"startxref")
            .expect("startxref keyword not found in base file");

        // Parse the offset after startxref
        let after_startxref = &base_data[startxref_pos + 9..];
        let offset_str_end = after_startxref.iter()
            .position(|&b| b == b'\n' || b == b'\r')
            .unwrap_or(after_startxref.len());

        let offset_str = std::str::from_utf8(&after_startxref[..offset_str_end])
            .unwrap_or("0");

        if let Ok(mut offset) = offset_str.parse::<u64>() {
            // Modify offset by +1
            offset += 1;

            // Replace the offset in the data
            let new_offset_str = offset.to_string();
            let new_bytes = new_offset_str.as_bytes();

            // Ensure we have enough space
            let replacement_start = startxref_pos + 9;
            let replacement_end = replacement_start + offset_str_end;

            let mut new_data = base_data.to_vec();
            new_data[replacement_start..replacement_end].copy_from_slice(new_bytes);

            let file = File::create(output_path).unwrap_or_else(|e| {
                panic!("Failed to create {:?}: {}", output_path, e);
            });
            let mut w = BufWriter::new(file);
            w.write_all(&new_data).unwrap();
            w.flush().unwrap();
        }
    }

    /// Generate a file with one corrupt xref entry.
    fn generate_corrupt_entry(&self, base_path: &PathBuf, output_path: &PathBuf) {
        // Read base file
        let mut base_data = std::fs::read(base_path).unwrap_or_else(|e| {
            panic!("Failed to read base file {:?}: {}", base_path, e);
        });

        // Find the xref table
        let xref_pos = base_data.windows(4).rposition(|w| w == b"xref")
            .expect("xref keyword not found in base file");

        // Find the first xref entry (after "0 6\n")
        let entries_start = xref_pos + 4;

        // Find the first newline after the subsection header
        let header_end = base_data[entries_start..].iter()
            .position(|&b| b == b'\n')
            .map(|p| entries_start + p)
            .unwrap_or(entries_start);

        // Corrupt the first non-zero entry (object 1)
        // Each entry is 20 bytes, skip object 0 (free entry)
        let entry1_start = header_end + 1 + 20;

        if entry1_start + 10 <= base_data.len() {
            // Modify the offset to be invalid
            base_data[entry1_start..entry1_start + 10].copy_from_slice(b"9999999999");
        }

        let file = File::create(output_path).unwrap_or_else(|e| {
            panic!("Failed to create {:?}: {}", output_path, e);
        });
        let mut w = BufWriter::new(file);
        w.write_all(&base_data).unwrap();
        w.flush().unwrap();
    }

    /// Generate a file with circular /Prev reference.
    fn generate_circular_prev(&self, output_path: &PathBuf) {
        let file = File::create(output_path).unwrap_or_else(|e| {
            panic!("Failed to create {:?}: {}", output_path, e);
        });
        let mut w = BufWriter::new(file);

        // PDF header
        writeln!(w, "%PDF-1.4").unwrap();

        // Minimal objects
        writeln!(w, "1 0 obj").unwrap();
        writeln!(w, "<< /Type /Catalog").unwrap();
        writeln!(w, "   /Pages 2 0 R").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        writeln!(w, "2 0 obj").unwrap();
        writeln!(w, "<< /Type /Pages").unwrap();
        writeln!(w, "   /Kids [3 0 R]").unwrap();
        writeln!(w, "   /Count 1").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        writeln!(w, "3 0 obj").unwrap();
        writeln!(w, "<< /Type /Page").unwrap();
        writeln!(w, "   /Parent 2 0 R").unwrap();
        writeln!(w, "   /MediaBox [0 0 612 792]").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Calculate the offset of Xref B by generating it first to an in-memory buffer
        let mut xref_b_data = Vec::new();
        {
            let mut w_b = BufWriter::new(&mut xref_b_data);
            writeln!(w_b, "xref").unwrap();
            writeln!(w_b, "0 1").unwrap();
            writeln!(w_b, "0000000000 65535 f ").unwrap();

            writeln!(w_b, "trailer").unwrap();
            writeln!(w_b, "<< /Size 4").unwrap();
            writeln!(w_b, "   /Root 1 0 R").unwrap();
            writeln!(w_b, ">>").unwrap();  // /Prev will be added later

            writeln!(w_b, "startxref").unwrap();
            writeln!(w_b, "0").unwrap();  // Placeholder
            writeln!(w_b, "%%EOF").unwrap();
            w_b.flush().unwrap();
        }

        // Now we know the approximate size of Xref B
        // Calculate Xref A offset (current position)
        let xref_a_offset = w.stream_position().unwrap();

        // Calculate Xref B offset (Xref A offset + size of Xref A)
        let xref_a_size = 200; // Approximate size of first xref + trailer
        let xref_b_offset = xref_a_offset + xref_a_size;

        // Xref A points to Xref B
        writeln!(w, "xref").unwrap();
        writeln!(w, "0 4").unwrap();
        writeln!(w, "0000000000 65535 f ").unwrap();
        writeln!(w, "0000000017 00000 n ").unwrap();
        writeln!(w, "0000000082 00000 n ").unwrap();
        writeln!(w, "0000000160 00000 n ").unwrap();

        writeln!(w, "trailer").unwrap();
        writeln!(w, "<< /Size 4").unwrap();
        writeln!(w, "   /Root 1 0 R").unwrap();
        writeln!(w, "   /Prev {}", xref_b_offset).unwrap();  // Points to Xref B
        writeln!(w, ">>").unwrap();

        writeln!(w, "startxref").unwrap();
        writeln!(w, "{}", xref_a_offset).unwrap();
        writeln!(w, "%%EOF").unwrap();

        // Xref B points back to Xref A (creates cycle)
        // Get the actual offset now
        let actual_xref_b_offset = w.stream_position().unwrap();

        writeln!(w, "xref").unwrap();
        writeln!(w, "0 1").unwrap();
        writeln!(w, "0000000000 65535 f ").unwrap();

        writeln!(w, "trailer").unwrap();
        writeln!(w, "<< /Size 4").unwrap();
        writeln!(w, "   /Root 1 0 R").unwrap();
        writeln!(w, "   /Prev {}", xref_a_offset).unwrap();  // Points back to Xref A
        writeln!(w, ">>").unwrap();

        writeln!(w, "startxref").unwrap();
        writeln!(w, "{}", actual_xref_b_offset).unwrap();
        writeln!(w, "%%EOF").unwrap();

        w.flush().unwrap();
    }

    /// Generate a file with 50 incremental revisions (tests depth limit).
    fn generate_deep_prev_chain(&self, output_path: &PathBuf) {
        let file = File::create(output_path).unwrap_or_else(|e| {
            panic!("Failed to create {:?}: {}", output_path, e);
        });
        let mut w = BufWriter::new(file);

        // PDF header
        writeln!(w, "%PDF-1.4").unwrap();

        // Minimal baseline objects
        writeln!(w, "1 0 obj").unwrap();
        writeln!(w, "<< /Type /Catalog").unwrap();
        writeln!(w, "   /Pages 2 0 R").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        writeln!(w, "2 0 obj").unwrap();
        writeln!(w, "<< /Type /Pages").unwrap();
        writeln!(w, "   /Kids [3 0 R]").unwrap();
        writeln!(w, "   /Count 1").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        writeln!(w, "3 0 obj").unwrap();
        writeln!(w, "<< /Type /Page").unwrap();
        writeln!(w, "   /Parent 2 0 R").unwrap();
        writeln!(w, "   /MediaBox [0 0 612 792]").unwrap();
        writeln!(w, ">>").unwrap();
        writeln!(w, "endobj").unwrap();

        // Baseline xref
        let mut prev_offset = w.stream_position().unwrap();

        writeln!(w, "xref").unwrap();
        writeln!(w, "0 4").unwrap();
        writeln!(w, "0000000000 65535 f ").unwrap();
        writeln!(w, "0000000017 00000 n ").unwrap();
        writeln!(w, "0000000082 00000 n ").unwrap();
        writeln!(w, "0000000160 00000 n ").unwrap();

        writeln!(w, "trailer").unwrap();
        writeln!(w, "<< /Size 4").unwrap();
        writeln!(w, "   /Root 1 0 R").unwrap();
        writeln!(w, ">>").unwrap();

        writeln!(w, "startxref").unwrap();
        writeln!(w, "{}", prev_offset).unwrap();
        writeln!(w, "%%EOF").unwrap();

        // Generate 50 incremental revisions
        for i in 1..=50 {
            // Add a new object in each revision
            writeln!(w, "{} 0 obj", 3 + i).unwrap();
            writeln!(w, "(Revision {})", i).unwrap();
            writeln!(w, "endobj").unwrap();

            let new_offset = w.stream_position().unwrap();

            writeln!(w, "xref").unwrap();
            writeln!(w, "{} 1", 3 + i).unwrap();
            let offset = i * 50 + 200;
            let offset_str = format!("{:010}", offset);
            writeln!(w, "{} 00000 n ", offset_str).unwrap();

            writeln!(w, "trailer").unwrap();
            writeln!(w, "<< /Size {}", 4 + i).unwrap();
            writeln!(w, "   /Root 1 0 R").unwrap();
            writeln!(w, "   /Prev {}", prev_offset).unwrap();
            writeln!(w, ">>").unwrap();

            writeln!(w, "startxref").unwrap();
            writeln!(w, "{}", new_offset).unwrap();
            writeln!(w, "%%EOF").unwrap();

            prev_offset = new_offset;
        }

        w.flush().unwrap();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <output-dir>", args[0]);
        eprintln!("\nGenerates PDF fixtures for xref testing.");
        process::exit(1);
    }

    let output_dir = PathBuf::from(&args[1]);

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&output_dir).unwrap_or_else(|e| {
        panic!("Failed to create output directory {:?}: {}", output_dir, e);
    });

    let gen = Generator::new(output_dir);

    // Generate all fixture types
    for fixture_type in [
        FixtureType::WellFormedTraditional,
        FixtureType::WellFormedStream,
        FixtureType::HybridFile,
        FixtureType::PrevChain3Revisions,
        FixtureType::Linearized,
        FixtureType::TruncatedAfterXref,
        FixtureType::StartxrefOffByOne,
        FixtureType::CorruptXrefEntry,
        FixtureType::CircularPrev,
        FixtureType::DeepPrevChain,
    ] {
        gen.generate(fixture_type);
    }

    println!("\nAll fixtures generated successfully!");
    println!("Run with BLESS=1 to generate golden files:");
    println!("  BLESS=1 cargo test -p pdftract-core --test integration -- xref");
}
