/// Generate legal filing test fixtures.
///
/// This creates 5 PDF fixtures for legal filing profile testing:
/// 1. federal_complaint - Federal district court complaint with case number, court, parties, filing date
/// 2. state_motion - State superior court motion to dismiss
/// 3. appellate_brief - Federal appellate brief
/// 4. court_order - Court order granting motion
/// 5. docket_sheet - Docket sheet with docket entries
///
/// Run with: cargo run --bin generate_legal_filing_fixtures

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Legal filing PDF builder
struct LegalFilingBuilder {
    title: String,
    court: String,
    case_number: String,
    parties: (String, String),
    filing_date: String,
    document_type: DocumentType,
    docket_entries: Vec<String>,
}

enum DocumentType {
    Complaint,
    Motion,
    AppellateBrief,
    Order,
    DocketSheet,
}

impl LegalFilingBuilder {
    fn new(
        title: &str,
        court: &str,
        case_number: &str,
        plaintiff: &str,
        defendant: &str,
        filing_date: &str,
        document_type: DocumentType,
    ) -> Self {
        Self {
            title: title.to_string(),
            court: court.to_string(),
            case_number: case_number.to_string(),
            parties: (plaintiff.to_string(), defendant.to_string()),
            filing_date: filing_date.to_string(),
            document_type,
            docket_entries: Vec::new(),
        }
    }

    fn with_docket_entries(mut self, entries: Vec<&str>) -> Self {
        self.docket_entries = entries.iter().map(|s| s.to_string()).collect();
        self
    }

    fn build(&self) -> Vec<u8> {
        let mut pdf_data = String::new();

        // PDF header
        pdf_data.push_str("%PDF-1.4\n");
        pdf_data.push_str("%Legal-Magic-Comment\n");

        let mut objects = Vec::new();
        let mut current_id = 1;

        // Catalog (object 1)
        let catalog = format!("<</Type/Catalog/Pages {} 0 R>>", current_id + 1);
        objects.push(catalog);
        current_id += 1;

        // Calculate page count
        let page_count = match self.document_type {
            DocumentType::DocketSheet => 2,
            DocumentType::Complaint | DocumentType::AppellateBrief => 3,
            _ => 2,
        };

        // Pages root (object 2)
        let kids: Vec<String> = (0..page_count)
            .map(|i| format!("{} 0 R", current_id + 1 + i))
            .collect();
        let pages = format!(
            "<</Type/Pages/Count {}/Kids[{}]/Resources<<//Font<</F1 {} 0 R>>>>/MediaBox[0 0 612 792]>>",
            page_count,
            kids.join(" "),
            current_id + page_count + 1
        );
        objects.push(pages);
        current_id += 1;

        // Font (will be after all pages)
        let font_id = current_id + page_count + 1;

        // Build pages based on document type
        let page_contents = match self.document_type {
            DocumentType::Complaint => self.build_complaint_pages(),
            DocumentType::Motion => self.build_motion_pages(),
            DocumentType::AppellateBrief => self.build_appellate_pages(),
            DocumentType::Order => self.build_order_pages(),
            DocumentType::DocketSheet => self.build_docket_pages(),
        };

        for (i, _) in page_contents.iter().enumerate() {
            let page = format!(
                "<</Type/Page/Parent {} 0 R/Contents {} 0 R>>",
                2,
                current_id + page_count + 2 + i
            );
            objects.push(page);
        }

        // Font object
        let font = "<</Type/Font/Subtype/Type1/BaseFont/Times-Roman>>";
        objects.push(font.to_string());

        // Content streams
        for content in &page_contents {
            if !content.is_empty() {
                let content_with_len = format!(
                    "<</Length {}>>\nstream\n{}\nendstream",
                    content.len(),
                    content
                );
                objects.push(content_with_len);
            }
        }

        // Info object
        let info = format!(
            "<</Title({})/Producer(pdftract-test)>>",
            escape_pdf_string(&self.title)
        );
        objects.push(info);

        // Write all objects
        let mut object_offsets = Vec::new();
        for obj in &objects {
            object_offsets.push(pdf_data.len());
            pdf_data.push_str(&format!("{} 0 obj\n", object_offsets.len() + 1));
            pdf_data.push_str(obj);
            pdf_data.push_str("\nendobj\n");
        }

        // xref table
        let xref_offset = pdf_data.len();
        pdf_data.push_str("xref\n");
        pdf_data.push_str("0 1\n");
        pdf_data.push_str("0000000000 65535 f \n");
        pdf_data.push_str(&format!("1 {}\n", objects.len()));
        for i in 0..objects.len() {
            pdf_data.push_str(&format!("{:010x} 00000 n \n", object_offsets[i]));
        }

        // Trailer
        pdf_data.push_str("trailer\n");
        pdf_data.push_str(&format!(
            "<</Size {} /Root 1 0 R /Info {} 0 R>>\n",
            objects.len() + 1,
            objects.len()
        ));
        pdf_data.push_str("startxref\n");
        pdf_data.push_str(&format!("{}\n", xref_offset));
        pdf_data.push_str("%%EOF\n");

        pdf_data.into_bytes()
    }

    fn build_header_content(&self) -> String {
        let mut content = String::new();

        // Court name (large font at top)
        content.push_str("BT\n50 750 Td\n16 Tf\n(");
        content.push_str(&escape_pdf_string(&self.court));
        content.push_str(") Tj\nET\n");

        // Case number
        content.push_str("BT\n50 720 Td\n12 Tf\n(");
        content.push_str(&escape_pdf_string(&format!("Case No.: {}", self.case_number)));
        content.push_str(") Tj\nET\n");

        // Title/heading
        content.push_str("BT\n50 680 Td\n14 Tf\n(");
        content.push_str(&escape_pdf_string(&self.title));
        content.push_str(") Tj\nET\n");

        // Parties
        content.push_str("BT\n50 640 Td\n12 Tf\n(");
        content.push_str(&escape_pdf_string(&format!(
            "{}, Plaintiff,\nv.\n{}, Defendant",
            self.parties.0, self.parties.1
        )));
        content.push_str(") Tj\nET\n");

        // Filing date
        content.push_str("BT\n50 580 Td\n10 Tf\n(");
        content.push_str(&escape_pdf_string(&format!("Filed: {}", self.filing_date)));
        content.push_str(") Tj\nET\n");

        content
    }

    fn build_complaint_pages(&self) -> Vec<String> {
        let mut pages = Vec::new();

        // Page 1: Header and complaint body
        let mut page1 = self.build_header_content();

        // Complaint heading
        page1.push_str("BT\n50 540 Td\n14 Tf\n(COMPLAINT) Tj\nET\n");

        // Jurisdiction
        page1.push_str("BT\n50 500 Td\n12 Tf\n(JURISDICTION AND VENUE) Tj\nET\n");
        page1.push_str("BT\n50 480 Td\n10 Tf\n(1. This Court has jurisdiction under 28 U.S.C. \\) Tj\nET\n");
        page1.push_str("BT\n50 466 Td\n10 Tf\\(\\) Tj\nET\n");
        page1.push_str("BT\n60 466 Td\n10 Tf\n(1332. Venue is proper under 28 U.S.C. \\) Tj\nET\n");
        page1.push_str("BT\n60 452 Td\n10 Tf\\(\\) Tj\nET\n");
        page1.push_str("BT\n70 452 Td\n10 Tf\n(1391.) Tj\nET\n");

        // Parties
        page1.push_str("BT\n50 410 Td\n12 Tf\n(PARTIES) Tj\nET\n");
        page1.push_str("BT\n50 390 Td\n10 Tf\n(2. Plaintiff ) Tj\nET\n");
        page1.push_str("BT\n130 390 Td\n10 Tf\n(");
        page1.push_str(&escape_pdf_string(&self.parties.0));
        page1.push_str(") Tj\nET\n");
        page1.push_str("BT\n50 376 Td\n10 Tf\n(is a corporation organized under the laws of Delaware) Tj\nET\n");
        page1.push_str("BT\n50 362 Td\n10 Tf\n(with its principal place of business in San Francisco, California.) Tj\nET\n");

        // Facts
        page1.push_str("BT\n50 320 Td\n12 Tf\n(FACTUAL BACKGROUND) Tj\nET\n");
        page1.push_str("BT\n50 300 Td\n10 Tf\n(3. On or about January 15, 2024, Plaintiff entered into a contract) Tj\nET\n");
        page1.push_str("BT\n50 286 Td\n10 Tf\n(with Defendant for the sale of goods. Defendant breached said contract) Tj\nET\n");
        page1.push_str("BT\n50 272 Td\n10 Tf\n(by failing to deliver the goods as agreed, causing damages in excess) Tj\nET\n");
        page1.push_str("BT\n50 258 Td\n10 Tf\n(of $100,000.) Tj\nET\n");

        // Prayer for relief
        page1.push_str("BT\n50 220 Td\n12 Tf\n(PRAYER FOR RELIEF) Tj\nET\n");
        page1.push_str("BT\n50 200 Td\n10 Tf\n(WHEREFORE, Plaintiff respectfully requests that this Court:) Tj\nET\n");
        page1.push_str("BT\n70 180 Td\n10 Tf\n(a) Enter judgment in favor of Plaintiff and against Defendant) Tj\nET\n");
        page1.push_str("BT\n70 166 Td\\(\\) Tj\nET\n");
        page1.push_str("BT\\(70 166 Td\\) 10 Tf\\(in the amount of $100,000 plus interest;\\) Tj\nET\n");
        page1.push_str("BT\\(70 152 Td\\) 10 Tf\\(b) Award Plaintiff its costs and attorneys\\(\\'\\) fees; and Tj\nET\n");
        page1.push_str("BT\\(70 138 Td\\) 10 Tf\\(c) Grant such other relief as the Court deems just. Tj\nET\n");

        // Signature block
        page1.push_str("BT\n50 80 Td\n10 Tf\\(Dated: \\) Tj\nET\n");
        page1.push_str("BT\\(110 80 Td\\) 10 Tf\\(");
        page1.push_str(&escape_pdf_string(&self.filing_date));
        page1.push_str("\\) Tj\nET\n");

        pages.push(page1);

        // Page 2: Verification
        let mut page2 = String::new();
        page2.push_str("BT\n50 750 Td\n12 Tf\n(VERIFICATION) Tj\nET\n");
        page2.push_str("BT\n50 720 Td\n10 Tf\\(I declare under penalty of perjury that the foregoing is true and\\) Tj\nET\n");
        page2.push_str("BT\\(50 706 Td\\) 10 Tf\\(correct to the best of my knowledge and belief.\\) Tj\nET\n");
        page2.push_str("BT\\(50 650 Td\\) 10 Tf\\(Respectfully submitted,\\) Tj\nET\n");
        page2.push_str("BT\\(50 600 Td\\) 10 Tf\\(/s/ John Smith\\) Tj\nET\n");
        page2.push_str("BT\\(50 586 Td\\) 10 Tf\\(John Smith\\) Tj\nET\n");
        page2.push_str("BT\\(50 572 Td\\) 10 Tf\\(Attorney for Plaintiff\\) Tj\nET\n");

        pages.push(page2);

        // Page 3: Certificate of service
        let mut page3 = String::new();
        page3.push_str("BT\n50 750 Td\n12 Tf\\(CERTIFICATE OF SERVICE\\) Tj\nET\n");
        page3.push_str("BT\\(50 720 Td\\) 10 Tf\\(I hereby certify that I served the foregoing document on all\\) Tj\nET\n");
        page3.push_str("BT\\(50 706 Td\\) 10 Tf\\(parties via the Court\\(\\'\\)s electronic filing system on \\) Tj\nET\n");
        page3.push_str("BT\\(50 692 Td\\) 10 Tf\\(");
        page3.push_str(&escape_pdf_string(&self.filing_date));
        page3.push_str(".\\) Tj\nET\n");

        pages.push(page3);

        pages
    }

    fn build_motion_pages(&self) -> Vec<String> {
        let mut pages = Vec::new();

        // Page 1: Motion header and body
        let mut page1 = self.build_header_content();

        // Motion heading
        page1.push_str("BT\n50 540 Td\n14 Tf\n(MOTION TO DISMISS) Tj\nET\n");

        // Notice of motion
        page1.push_str("BT\n50 500 Td\n12 Tf\\(NOTICE OF MOTION\\) Tj\nET\n");
        page1.push_str("BT\\(50 470 Td\\) 10 Tf\\(PLEASE TAKE NOTICE that Defendant will move this Court for an order\\) Tj\nET\n");
        page1.push_str("BT\\(50 456 Td\\) 10 Tf\\(dismissing the Complaint pursuant to Federal Rule of Civil Procedure\\) Tj\nET\n");
        page1.push_str("BT\\(50 442 Td\\) 10 Tf\\(12\\(\\)\\) Tj\\(b\\)\\(6). The motion will be heard on [Date] at [Time] in\\) Tj\nET\n");
        page1.push_str("BT\\(50 428 Td\\) 10 Tf\\(Courtroom [Number].\\) Tj\nET\n");

        // Legal standard
        page1.push_str("BT\n50 380 Td\n12 Tf\\(LEGAL STANDARD\\) Tj\nET\n");
        page1.push_str("BT\\(50 350 Td\\) 10 Tf\\(Under Rule 12\\(\\)\\) Tj\\(b\\)\\(6, a court may dismiss a complaint for failure\\) Tj\nET\n");
        page1.push_str("BT\\(50 336 Td\\) 10 Tf\\(to state a claim upon which relief can be granted.\\) Tj\nET\n");

        // Argument
        page1.push_str("BT\n50 290 Td\n12 Tf\\(ARGUMENT\\) Tj\nET\n");
        page1.push_str("BT\\(50 260 Td\\) 10 Tf\\(I. The Complaint fails to state a claim because Plaintiff has not\\) Tj\nET\n");
        page1.push_str("BT\\(50 246 Td\\) 10 Tf\\(alleged facts sufficient to support each element of the claimed cause\\) Tj\nET\n");
        page1.push_str("BT\\(50 232 Td\\) 10 Tf\\(of action.\\) Tj\nET\n");

        // Prayer for relief
        page1.push_str("BT\n50 180 Td\n12 Tf\\(PRAYER FOR RELIEF\\) Tj\nET\n");
        page1.push_str("BT\\(50 150 Td\\) 10 Tf\\(WHEREFORE, Defendant respectfully requests that this Court dismiss the\\) Tj\nET\n");
        page1.push_str("BT\\(50 136 Td\\) 10 Tf\\(Complaint with prejudice and grant such other relief as is just.\\) Tj\nET\n");

        // Dated
        page1.push_str("BT\n50 80 Td\n10 Tf\\(Dated: \\) Tj\nET\n");
        page1.push_str("BT\\(110 80 Td\\) 10 Tf\\(");
        page1.push_str(&escape_pdf_string(&self.filing_date));
        page1.push_str("\\) Tj\nET\n");

        pages.push(page1);

        // Page 2: Memorandum of law
        let mut page2 = String::new();
        page2.push_str("BT\n50 750 Td\n14 Tf\\(MEMORANDUM OF LAW\\) Tj\nET\n");

        page2.push_str("BT\n50 710 Td\n12 Tf\\(I. INTRODUCTION\\) Tj\nET\n");
        page2.push_str("BT\\(50 680 Td\\) 10 Tf\\(This motion challenges the sufficiency of Plaintiff\\(\\'\\)s complaint. The\\) Tj\nET\n");
        page2.push_str("BT\\(50 666 Td\\) 10 Tf\\(allegations are conclusory and fail to state a plausible claim for relief.\\) Tj\nET\n");

        page2.push_str("BT\n50 620 Td\n12 Tf\\(II. APPLICABLE LAW\\) Tj\nET\n");
        page2.push_str("BT\\(50 590 Td\\) 10 Tf\\(To survive a motion to dismiss, a complaint must contain sufficient\\) Tj\nET\n");
        page2.push_str("BT\\(50 576 Td\\) 10 Tf\\(factual matter, accepted as true, to state a claim that is plausible on\\) Tj\nET\n");
        page2.push_str("BT\\(50 562 Td\\) 10 Tf\\(its face. Bell Atlantic Corp. v. Twombly, 550 U.S. 544, 570 \\) Tj\\(\\) Tj\nET\n");
        page2.push_str("BT\\(50 548 Td\\) 10 Tf\\(2007).\\) Tj\nET\n");

        page2.push_str("BT\n50 500 Td\n12 Tf\\(III. ARGUMENT\\) Tj\nET\n");
        page2.push_str("BT\\(50 470 Td\\) 10 Tf\\(Plaintiff\\(\\'\\)s complaint consists of bare conclusions without factual\\) Tj\nET\n");
        page2.push_str("BT\\(50 456 Td\\) 10 Tf\\(support. The allegations do not permit the reasonable inference that\\) Tj\nET\n");
        page2.push_str("BT\\(50 442 Td\\) 10 Tf\\(Defendant is liable for the alleged misconduct.\\) Tj\nET\n");

        pages.push(page2);

        pages
    }

    fn build_appellate_pages(&self) -> Vec<String> {
        let mut pages = Vec::new();

        // Page 1: Appellate brief header
        let mut page1 = String::new();

        // Court name
        page1.push_str("BT\n50 750 Td\n16 Tf\n(");
        page1.push_str(&escape_pdf_string(&self.court));
        page1.push_str(") Tj\nET\n");

        // Case number
        page1.push_str("BT\n50 720 Td\n12 Tf\n(");
        page1.push_str(&escape_pdf_string(&format!("No. {}", self.case_number)));
        page1.push_str(") Tj\nET\n");

        // Title
        page1.push_str("BT\n50 680 Td\n14 Tf\n(");
        page1.push_str(&escape_pdf_string(&self.title));
        page1.push_str(") Tj\nET\n");

        // Parties on appeal
        page1.push_str("BT\n50 640 Td\n12 Tf\n(");
        page1.push_str(&escape_pdf_string(&format!(
            "{}, Appellant,\nv.\n{}, Appellee.",
            self.parties.0, self.parties.1
        )));
        page1.push_str(") Tj\nET\n");

        // Appeal from
        page1.push_str("BT\n50 580 Td\n10 Tf\n(");
        page1.push_str(&escape_pdf_string(&format!(
            "Appeal from the United States District Court\nfor the Northern District of California",
        )));
        page1.push_str(") Tj\nET\n");

        // Brief heading
        page1.push_str("BT\n50 540 Td\n14 Tf\n(BRIEF FOR APPELLANT) Tj\nET\n");

        // Table of contents placeholder
        page1.push_str("BT\n50 500 Td\n12 Tf\n(TABLE OF CONTENTS) Tj\nET\n");
        page1.push_str("BT\n50 470 Td\n10 Tf\\(I.   STATEMENT OF JURISDICTION ..................... 1\\) Tj\nET\n");
        page1.push_str("BT\\(50 456 Td\\) 10 Tf\\(II.  STATEMENT OF THE ISSUE ........................ 2\\) Tj\nET\n");
        page1.push_str("BT\\(50 442 Td\\) 10 Tf\\(III. SUMMARY OF ARGUMENT .......................... 3\\) Tj\nET\n");
        page1.push_str("BT\\(50 428 Td\\) 10 Tf\\(IV.  ARGUMENT ....................................... 4\\) Tj\nET\n");
        page1.push_str("BT\\(50 414 Td\\) 10 Tf\\(V.   CONCLUSION .................................... 10\\) Tj\nET\n");

        pages.push(page1);

        // Page 2: Jurisdiction statement
        let mut page2 = String::new();
        page2.push_str("BT\n50 750 Td\n14 Tf\\(I. STATEMENT OF JURISDICTION\\) Tj\nET\n");
        page2.push_str("BT\\(50 720 Td\\) 10 Tf\\(This Court has jurisdiction under 28 U.S.C. \\) Tj\\(\\) Tj\nET\n");
        page2.push_str("BT\\(50 706 Td\\) 10 Tf\\(1291. The notice of appeal was filed on \\) Tj\nET\n");
        page2.push_str("BT\\(50 692 Td\\) 10 Tf\\(");
        page2.push_str(&escape_pdf_string(&self.filing_date));
        page2.push_str(".\\) Tj\nET\n");

        page2.push_str("BT\n50 650 Td\n14 Tf\\(II. STATEMENT OF THE ISSUE\\) Tj\nET\n");
        page2.push_str("BT\\(50 620 Td\\) 10 Tf\\(Whether the district court erred in granting Defendant\\(\\'\\)s motion\\) Tj\nET\n");
        page2.push_str("BT\\(50 606 Td\\) 10 Tf\\(to dismiss for failure to state a claim.\\) Tj\nET\n");

        page2.push_str("BT\n50 560 Td\n14 Tf\\(III. SUMMARY OF ARGUMENT\\) Tj\nET\n");
        page2.push_str("BT\\(50 530 Td\\) 10 Tf\\(The district court committed reversible error by dismissing the\\) Tj\nET\n");
        page2.push_str("BT\\(50 516 Td\\) 10 Tf\\(complaint. Plaintiff alleged sufficient facts to state a plausible\\) Tj\nET\n");
        page2.push_str("BT\\(50 502 Td\\) 10 Tf\\(claim for relief under Twombly and Iqbal.\\) Tj\nET\n");

        pages.push(page2);

        // Page 3: Argument
        let mut page3 = String::new();
        page3.push_str("BT\n50 750 Td\n14 Tf\\(IV. ARGUMENT\\) Tj\nET\n");

        page3.push_str("BT\n50 720 Td\n12 Tf\\(A. Standard of Review\\) Tj\nET\n");
        page3.push_str("BT\\(50 690 Td\\) 10 Tf\\(This Court reviews de novo a district court\\(\\'\\)s grant of a motion\\) Tj\nET\n");
        page3.push_str("BT\\(50 676 Td\\) 10 Tf\\(to dismiss for failure to state a claim. See, e.g., Reyes v. Eggleston,\\) Tj\nET\n");
        page3.push_str("BT\\(50 662 Td\\) 10 Tf\\(901 F.3d 1148, 1151 (9th Cir. 2018).\\) Tj\nET\n");

        page3.push_str("BT\n50 620 Td\n12 Tf\\(B. The Complaint States a Claim\\) Tj\nET\n");
        page3.push_str("BT\\(50 590 Td\\) 10 Tf\\(Plaintiff\\(\\'\\)s complaint alleges: \\(1\\) formation of a contract; \\(2\\) breach\\) Tj\nET\n");
        page3.push_str("BT\\(50 576 Td\\) 10 Tf\\(of that contract; and \\(3\\) damages resulting from the breach. These\\) Tj\nET\n");
        page3.push_str("BT\\(50 562 Td\\) 10 Tf\\(allegations are sufficient to state a claim for breach of contract.\\) Tj\nET\n");

        page3.push_str("BT\n50 510 Td\n12 Tf\\(V. CONCLUSION\\) Tj\nET\n");
        page3.push_str("BT\\(50 480 Td\\) 10 Tf\\(For the foregoing reasons, the district court\\(\\'\\)s decision should be\\) Tj\nET\n");
        page3.push_str("BT\\(50 466 Td\\) 10 Tf\\(reversed and the case remanded for further proceedings.\\) Tj\nET\n");

        pages.push(page3);

        pages
    }

    fn build_order_pages(&self) -> Vec<String> {
        let mut pages = Vec::new();

        // Page 1: Order header and content
        let mut page1 = String::new();

        // Court name
        page1.push_str("BT\n50 750 Td\n16 Tf\n(");
        page1.push_str(&escape_pdf_string(&self.court));
        page1.push_str(") Tj\nET\n");

        // Case number
        page1.push_str("BT\n50 720 Td\n12 Tf\n(");
        page1.push_str(&escape_pdf_string(&format!("Case No.: {}", self.case_number)));
        page1.push_str(") Tj\nET\n");

        // Title
        page1.push_str("BT\n50 680 Td\n14 Tf\n(");
        page1.push_str(&escape_pdf_string(&self.title));
        page1.push_str(") Tj\nET\n");

        // Parties
        page1.push_str("BT\n50 640 Td\n12 Tf\n(");
        page1.push_str(&escape_pdf_string(&format!(
            "{}, Plaintiff,\nv.\n{}, Defendant",
            self.parties.0, self.parties.1
        )));
        page1.push_str(") Tj\nET\n");

        // Order heading
        page1.push_str("BT\n50 580 Td\n14 Tf\n(ORDER GRANTING MOTION TO DISMISS) Tj\nET\n");

        // Introduction
        page1.push_str("BT\n50 540 Td\n10 Tf\\(This matter comes before the Court on Defendant\\(\\'\\)s Motion to Dismiss\\) Tj\nET\n");
        page1.push_str("BT\\(50 526 Td\\) 10 Tf\\([ECF No. 10]. Plaintiff filed an opposition [ECF No. 15], and\\) Tj\nET\n");
        page1.push_str("BT\\(50 512 Td\\) 10 Tf\\(Defendant filed a reply [ECF No. 18]. Having considered the parties\\(\\'\\)\\) Tj\nET\n");
        page1.push_str("BT\\(50 498 Td\\) 10 Tf\\(briefing and the applicable law, the Court GRANTS the motion.\\) Tj\nET\n");

        // Background
        page1.push_str("BT\n50 450 Td\n12 Tf\\(I. BACKGROUND\\) Tj\nET\n");
        page1.push_str("BT\\(50 420 Td\\) 10 Tf\\(Plaintiff initiated this action on \\) Tj\nET\n");
        page1.push_str("BT\\(50 406 Td\\) 10 Tf\\(");
        page1.push_str(&escape_pdf_string(&self.filing_date));
        page1.push_str(". The complaint alleges\\) Tj\nET\n");
        page1.push_str("BT\\(50 392 Td\\) 10 Tf\\(breach of contract.\\) Tj\nET\n");

        // Legal standard
        page1.push_str("BT\n50 340 Td\n12 Tf\\(II. LEGAL STANDARD\\) Tj\nET\n");
        page1.push_str("BT\\(50 310 Td\\) 10 Tf\\(To survive a motion to dismiss, a complaint must contain sufficient\\) Tj\nET\n");
        page1.push_str("BT\\(50 296 Td\\) 10 Tf\\(factual matter to state a claim that is plausible on its face.\\) Tj\nET\n");

        // Analysis
        page1.push_str("BT\n50 250 Td\n12 Tf\\(III. ANALYSIS\\) Tj\nET\n");
        page1.push_str("BT\\(50 220 Td\\) 10 Tf\\(Plaintiff\\(\\'\\)s complaint consists of conclusory allegations without\\) Tj\nET\n");
        page1.push_str("BT\\(50 206 Td\\) 10 Tf\\(factual support. The complaint does not state a claim for relief.\\) Tj\nET\n");

        // Conclusion
        page1.push_str("BT\n50 160 Td\n12 Tf\\(IV. CONCLUSION\\) Tj\nET\n");
        page1.push_str("BT\\(50 130 Td\\) 10 Tf\\(For the foregoing reasons, Defendant\\(\\'\\)s Motion to Dismiss is GRANTED.\\) Tj\nET\n");

        // Date and signature
        page1.push_str("BT\n50 80 Td\n10 Tf\\(Dated: \\) Tj\nET\n");
        page1.push_str("BT\\(110 80 Td\\) 10 Tf\\(");
        page1.push_str(&escape_pdf_string(&self.filing_date));
        page1.push_str("\\) Tj\nET\n");

        pages.push(page1);

        // Page 2: Signature block
        let mut page2 = String::new();
        page2.push_str("BT\n50 750 Td\n10 Tf\\(HONORABLE JANE DOE\\) Tj\nET\n");
        page2.push_str("BT\\(50 736 Td\\) 10 Tf\\(United States District Judge\\) Tj\nET\n");

        page2.push_str("BT\n50 680 Td\n12 Tf\\(IT IS SO ORDERED.\\) Tj\nET\n");

        pages.push(page2);

        pages
    }

    fn build_docket_pages(&self) -> Vec<String> {
        let mut pages = Vec::new();

        // Page 1: Docket sheet header
        let mut page1 = String::new();

        // Court name
        page1.push_str("BT\n50 750 Td\n16 Tf\n(");
        page1.push_str(&escape_pdf_string(&self.court));
        page1.push_str(") Tj\nET\n");

        // Docket heading
        page1.push_str("BT\n50 720 Td\n14 Tf\n(DOCKET SHEET) Tj\nET\n");

        // Case number
        page1.push_str("BT\n50 690 Td\n12 Tf\n(");
        page1.push_str(&escape_pdf_string(&format!("Case No.: {}", self.case_number)));
        page1.push_str(") Tj\nET\n");

        // Parties
        page1.push_str("BT\n50 660 Td\n10 Tf\n(");
        page1.push_str(&escape_pdf_string(&format!(
            "{} v. {}",
            self.parties.0, self.parties.1
        )));
        page1.push_str(") Tj\nET\n");

        // Docket entries header
        page1.push_str("BT\n50 620 Td\n12 Tf\n(DOCKET ENTRIES) Tj\nET\n");

        // Docket entries
        let mut y = 580;
        for (i, entry) in self.docket_entries.iter().enumerate() {
            page1.push_str(&format!("BT\n50 {} Td\n10 Tf\n(", y));
            page1.push_str(&escape_pdf_string(&format!("[{}]", i + 1)));
            page1.push_str(") Tj\nET\n");

            let entry_lines = wrap_text(entry, 65);
            for (j, line) in entry_lines.iter().enumerate() {
                let entry_y = y - (j as i32 * 14) - 14;
                page1.push_str(&format!("BT\n70 {} Td\n10 Tf\n(", entry_y));
                page1.push_str(&escape_pdf_string(line));
                page1.push_str(") Tj\nET\n");
            }

            y -= 14 * (entry_lines.len() as i32 + 2);
            if y < 50 {
                break;
            }
        }

        pages.push(page1);

        // Page 2: Additional docket entries or case summary
        let mut page2 = String::new();
        page2.push_str("BT\n50 750 Td\n12 Tf\\(CASE SUMMARY\\) Tj\nET\n");

        page2.push_str("BT\n50 720 Td\n10 Tf\\(Date Filed: \\) Tj\nET\n");
        page2.push_str("BT\\(140 720 Td\\) 10 Tf\\(");
        page2.push_str(&escape_pdf_string(&self.filing_date));
        page2.push_str("\\) Tj\nET\n");

        page2.push_str("BT\n50 690 Td\n10 Tf\\(Case Type: Civil - Contract\\) Tj\nET\n");
        page2.push_str("BT\\(50 676 Td\\) 10 Tf\\(Assigned Judge: Honorable Jane Doe\\) Tj\nET\n");
        page2.push_str("BT\\(50 662 Td\\) 10 Tf\\(Magistrate Judge: Honorable John Smith\\) Tj\nET\n");

        page2.push_str("BT\n50 620 Td\n12 Tf\\(CASE STATUS\\) Tj\nET\n");
        page2.push_str("BT\\(50 590 Td\\) 10 Tf\\(Status: Pending\\) Tj\nET\n");
        page2.push_str("BT\\(50 576 Td\\) 10 Tf\\(Next Deadline: Motion Hearing - March 15, 2024\\) Tj\nET\n");

        pages.push(page2);

        pages
    }
}

/// Escape a string for PDF literal strings
fn escape_pdf_string(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '(' => vec!['\\', '('],
            ')' => vec!['\\', ')'],
            '\\' => vec!['\\', '\\'],
            '\'' => vec!['\\', '\''],
            _ => vec![c],
        })
        .collect()
}

/// Wrap text to fit within a column width
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in words {
        if current_line.is_empty() {
            current_line.push_str(word);
        } else if current_line.len() + word.len() + 1 <= width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

fn main() -> std::io::Result<()> {
    let fixtures_dir = Path::new("tests/fixtures/profiles/legal_filing");

    // Ensure directory exists
    std::fs::create_dir_all(fixtures_dir)?;

    // 1. Federal complaint
    let builder = LegalFilingBuilder::new(
        "COMPLAINT FOR BREACH OF CONTRACT",
        "UNITED STATES DISTRICT COURT\nFOR THE NORTHERN DISTRICT OF CALIFORNIA",
        "3:24-cv-00123",
        "Acme Corporation",
        "Beta LLC",
        "January 15, 2024",
        DocumentType::Complaint,
    );
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("federal_complaint.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created federal_complaint.pdf");

    // 2. State motion
    let builder = LegalFilingBuilder::new(
        "DEFENDANT'S MOTION TO DISMISS",
        "SUPERIOR COURT OF CALIFORNIA\nCOUNTY OF SAN FRANCISCO",
        "CGC-24-123456",
        "Smith Enterprises",
        "Johnson Construction Inc.",
        "February 1, 2024",
        DocumentType::Motion,
    );
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("state_motion.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created state_motion.pdf");

    // 3. Appellate brief
    let builder = LegalFilingBuilder::new(
        "APPELLANT'S OPENING BRIEF",
        "UNITED STATES COURT OF APPEALS\nFOR THE NINTH CIRCUIT",
        "24-1234",
        "TechCorp Inc.",
        "DataSystems LLC",
        "March 10, 2024",
        DocumentType::AppellateBrief,
    );
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("appellate_brief.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created appellate_brief.pdf");

    // 4. Court order
    let builder = LegalFilingBuilder::new(
        "ORDER GRANTING DEFENDANT'S MOTION TO DISMISS",
        "UNITED STATES DISTRICT COURT\nFOR THE SOUTHERN DISTRICT OF NEW YORK",
        "1:24-cv-04567",
        "Global Trade Inc.",
        "Pacific Shipping Corp.",
        "March 20, 2024",
        DocumentType::Order,
    );
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("court_order.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created court_order.pdf");

    // 5. Docket sheet
    let builder = LegalFilingBuilder::new(
        "DOCKET SHEET",
        "UNITED STATES DISTRICT COURT\nFOR THE EASTERN DISTRICT OF TEXAS",
        "2:24-cv-00890",
        "PatentHolder LLC",
        "Infringer Corp.",
        "April 1, 2024",
        DocumentType::DocketSheet,
    ).with_docket_entries(vec![
        "04/01/2024 - Complaint filed by PatentHolder LLC.",
        "04/05/2024 - Summons issued.",
        "04/15/2024 - Waiver of service filed by Infringer Corp.",
        "04/20/2024 - Defendant's Answer due.",
        "04/25/2024 - Motion to extend time to answer filed.",
        "04/28/2024 - Order granting extension to 05/20/2024.",
        "05/18/2024 - Defendant's Answer filed.",
        "06/01/2024 - Case management conference scheduled.",
    ]);
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("docket_sheet.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created docket_sheet.pdf");

    println!("\nGenerated 5 legal filing fixtures in tests/fixtures/profiles/legal_filing/");
    Ok(())
}
