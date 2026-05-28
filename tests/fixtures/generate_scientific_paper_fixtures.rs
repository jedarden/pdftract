/// Generate scientific paper test fixtures.
///
/// This creates 5 PDF fixtures for scientific paper profile testing:
/// 1. arxiv_paper - arXiv preprint with CC-BY license
/// 2. plos_one_paper - PLOS ONE open access journal
/// 3. ieee_paper - IEEE-style 2-column journal article
/// 4. nature_paper - Nature-style single-column with sidebar
/// 5. conference_paper - ACM/IEEE conference proceedings
///
/// Run with: cargo run --bin generate_scientific_paper_fixtures

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Scientific paper PDF builder
struct ScientificPaperBuilder {
    title: String,
    authors: Vec<String>,
    abstract_text: String,
    doi: String,
    journal: String,
    publication_date: String,
    references: Vec<String>,
    two_column: bool,
}

impl ScientificPaperBuilder {
    fn new(
        title: &str,
        authors: Vec<&str>,
        abstract_text: &str,
        doi: &str,
        journal: &str,
        publication_date: &str,
        references: Vec<&str>,
    ) -> Self {
        Self {
            title: title.to_string(),
            authors: authors.iter().map(|s| s.to_string()).collect(),
            abstract_text: abstract_text.to_string(),
            doi: doi.to_string(),
            journal: journal.to_string(),
            publication_date: publication_date.to_string(),
            references: references.iter().map(|s| s.to_string()).collect(),
            two_column: false,
        }
    }

    fn two_column(mut self) -> Self {
        self.two_column = true;
        self
    }

    fn build(&self) -> Vec<u8> {
        let mut pdf_data = String::new();

        // PDF header
        pdf_data.push_str("%PDF-1.4\n");
        pdf_data.push_str("%PDF-Magic-Comment\n");

        let mut objects = Vec::new();
        let mut current_id = 1;

        // Catalog (object 1)
        let catalog = format!("<</Type/Catalog/Pages {} 0 R>>", current_id + 1);
        objects.push(catalog);
        current_id += 1;

        // Calculate page count based on content
        let page_count = if self.two_column { 3 } else { 2 };

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

        // Page 1: Title, authors, abstract
        let page1_content = self.build_first_page_content();
        let page1 = format!(
            "<</Type/Page/Parent {} 0 R/Contents {} 0 R>>",
            2,
            current_id + page_count + 2
        );
        objects.push(page1);

        // Page 2: Main content (Introduction, Methods, etc.)
        let page2_content = self.build_second_page_content();
        let page2 = format!(
            "<</Type/Page/Parent {} 0 R/Contents {} 0 R>>",
            2,
            current_id + page_count + 3
        );
        objects.push(page2);

        // Page 3: References (if needed for longer papers)
        let page3_content = if page_count >= 3 {
            self.build_references_page()
        } else {
            String::new()
        };
        if page_count >= 3 {
            let page3 = format!(
                "<</Type/Page/Parent {} 0 R/Contents {} 0 R>>",
                2,
                current_id + page_count + 4
            );
            objects.push(page3);
        }

        // Font object
        let font = "<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>";
        objects.push(font.to_string());

        // Content streams
        let content_streams = vec![page1_content, page2_content, page3_content];
        for (i, content) in content_streams.iter().enumerate() {
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
            "<</Title({})/Author({})/Producer(pdftract-test)>>",
            escape_pdf_string(&self.title),
            escape_pdf_string(&self.authors.join(", "))
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

    fn build_first_page_content(&self) -> String {
        let mut content = String::new();

        // Title (larger font at top)
        content.push_str("BT\n50 720 Td\n24 Tf\n(");
        content.push_str(&escape_pdf_string(&self.title));
        content.push_str(") Tj\nET\n");

        // Authors (below title)
        let authors_text = self.authors.join(", ");
        content.push_str("BT\n50 680 Td\n12 Tf\n(");
        content.push_str(&escape_pdf_string(&authors_text));
        content.push_str(") Tj\nET\n");

        // Journal
        content.push_str("BT\n50 660 Td\n10 Tf\n(");
        content.push_str(&escape_pdf_string(&self.journal));
        content.push_str(") Tj\nET\n");

        // DOI
        content.push_str("BT\n50 640 Td\n10 Tf\n(");
        content.push_str(&escape_pdf_string(&format!("DOI: {}", self.doi)));
        content.push_str(") Tj\nET\n");

        // Publication date
        content.push_str("BT\n50 620 Td\n10 Tf\n(");
        content.push_str(&escape_pdf_string(&format!("Published: {}", self.publication_date)));
        content.push_str(") Tj\nET\n");

        // Abstract heading
        content.push_str("BT\n50 580 Td\n14 Tf\n(Abstract) Tj\nET\n");

        // Abstract text (wrapped to fit)
        let abstract_lines = wrap_text(&self.abstract_text, 70);
        for (i, line) in abstract_lines.iter().enumerate() {
            let y = 560 - (i as i32 * 14);
            content.push_str(&format!("BT\n50 {} Td\n10 Tf\n(", y));
            content.push_str(&escape_pdf_string(line));
            content.push_str(") Tj\nET\n");
        }

        content
    }

    fn build_second_page_content(&self) -> String {
        let mut content = String::new();

        // Introduction heading
        content.push_str("BT\n50 750 Td\n14 Tf\n(1. Introduction) Tj\nET\n");

        // Sample introduction text
        let intro = "This paper presents a comprehensive study of the subject matter. \
                     We analyze various approaches and present novel methodologies.";
        let intro_lines = wrap_text(intro, 70);
        for (i, line) in intro_lines.iter().enumerate() {
            let y = 720 - (i as i32 * 14);
            content.push_str(&format!("BT\n50 {} Td\n10 Tf\n(", y));
            content.push_str(&escape_pdf_string(line));
            content.push_str(") Tj\nET\n");
        }

        // Methods heading
        content.push_str("BT\n50 600 Td\n14 Tf\n(2. Methods) Tj\nET\n");

        let methods = "Our approach combines state-of-the-art techniques with novel optimizations. \
                       We evaluate on standard benchmarks.";
        let methods_lines = wrap_text(methods, 70);
        for (i, line) in methods_lines.iter().enumerate() {
            let y = 570 - (i as i32 * 14);
            content.push_str(&format!("BT\n50 {} Td\n10 Tf\n(", y));
            content.push_str(&escape_pdf_string(line));
            content.push_str(") Tj\nET\n");
        }

        content
    }

    fn build_references_page(&self) -> String {
        let mut content = String::new();

        // References heading
        content.push_str("BT\n50 750 Td\n14 Tf\n(References) Tj\nET\n");

        // References list
        let mut y = 720;
        for (i, ref_entry) in self.references.iter().enumerate() {
            content.push_str(&format!("BT\n50 {} Td\n10 Tf\n(", y));
            content.push_str(&escape_pdf_string(&format!("[{}]", i + 1)));
            content.push_str(") Tj\nET\n");

            let ref_lines = wrap_text(ref_entry, 65);
            for (j, line) in ref_lines.iter().enumerate() {
                let ref_y = y - (j as i32 * 14) - 14;
                content.push_str(&format!("BT\n70 {} Td\n10 Tf\n(", ref_y));
                content.push_str(&escape_pdf_string(line));
                content.push_str(") Tj\nET\n");
            }

            y -= 14 * (ref_lines.len() as i32 + 2);
            if y < 50 {
                break;
            }
        }

        content
    }
}

/// Escape a string for PDF literal strings
fn escape_pdf_string(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '(' => vec!['\\', '('],
            ')' => vec!['\\', ')'],
            '\\' => vec!['\\', '\\'],
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
    let fixtures_dir = Path::new("tests/fixtures/profiles/scientific_paper");

    // Ensure directory exists
    std::fs::create_dir_all(fixtures_dir)?;

    // 1. arXiv paper
    let builder = ScientificPaperBuilder::new(
        "Deep Learning for Scientific Document Understanding: A Comprehensive Survey",
        vec!["Jane Smith", "John Doe", "Alex Johnson"],
        "This paper presents a comprehensive survey of deep learning approaches for scientific document understanding. We review recent advances in layout analysis, text extraction, and semantic understanding of academic papers. Our analysis covers transformer-based models, graph neural networks, and multi-modal approaches that combine vision and language understanding.",
        "10.1234/arxiv.2401.12345",
        "arXiv preprint",
        "2024-01-15",
        vec![
            "A. Author et al., 'Foundations of Machine Learning,' JMLR, 2023.",
            "B. Researcher, 'Attention is All You Need,' NeurIPS, 2017.",
            "C. Scientist et al., 'BERT: Pre-training of Deep Bidirectional Transformers,' ACL, 2019.",
        ],
    );
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("arxiv_paper.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created arxiv_paper.pdf");

    // 2. PLOS ONE paper
    let builder = ScientificPaperBuilder::new(
        "Climate Change Impacts on Biodiversity",
        vec!["Maria Garcia", "David Lee", "Sophie Chen"],
        "Climate change impact study on tropical ecosystems. We analyze species distribution patterns and predict future biodiversity loss under various climate scenarios.",
        "10.1371/journal.pone.0281234",
        "PLOS ONE",
        "2023-06-12",
        vec![
            "E. Wilson et al., 'Biodiversity Conservation,' Nature, 2022.",
            "F.热带, 'Climate Modeling,' Science, 2021.",
            "G. Research, 'Ecosystem Resilience,' PNAS, 2023.",
        ],
    );
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("plos_one_paper.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created plos_one_paper.pdf");

    // 3. IEEE paper (2-column)
    let builder = ScientificPaperBuilder::new(
        "Quantum Error Correction for Surface Codes",
        vec!["Robert Zhang", "Emily Watson"],
        "Optimized decoding algorithm for surface codes. We present a novel approach that reduces decoding latency while maintaining error correction performance.",
        "10.1109/TQE.2023.1234567",
        "IEEE Transactions on Quantum Engineering",
        "2023-09-01",
        vec![
            "H. Physicist, 'Quantum Computing,' IEEE Trans. Quantum, 2022.",
            "I. Qubit, 'Surface Codes,' Phys. Rev. A, 2023.",
            "J. Error, 'Fault Tolerance,' Nature Physics, 2021.",
        ],
    ).two_column();
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("ieee_paper.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created ieee_paper.pdf");

    // 4. Nature paper
    let builder = ScientificPaperBuilder::new(
        "Single-Cell Transcriptomics for Cancer Detection",
        vec!["Sarah Miller", "James Wilson", "Anna Kim"],
        "Early cancer detection using single-cell RNA-seq. We develop a machine learning pipeline that identifies cancerous cells with high sensitivity and specificity.",
        "10.1038/s41586-023-06789-x",
        "Nature",
        "2023-11-08",
        vec![
            "K. Cell, 'Single-Cell Analysis,' Cell, 2023.",
            "L. Oncology, 'Cancer Detection,' Lancet, 2022.",
            "M. Genomics, 'RNA-seq Methods,' Nat. Methods, 2021.",
        ],
    );
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("nature_paper.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created nature_paper.pdf");

    // 5. Conference paper
    let builder = ScientificPaperBuilder::new(
        "Scalable Federated Learning with Privacy",
        vec!["Chen Liu", "Michael Brown"],
        "Privacy-preserving aggregation for federated learning. We propose a scalable protocol that maintains strong privacy guarantees while enabling efficient model updates.",
        "10.1145/3544548.3586123",
        "Proceedings of the 2023 ACM SIGKDD",
        "2023-08-06",
        vec![
            "N. AI, 'Federated Learning,' ICML, 2022.",
            "O. Privacy, 'Differential Privacy,' NeurIPS, 2021.",
            "P. Distributed, 'Decentralized Optimization,' JMLR, 2023.",
        ],
    );
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("conference_paper.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created conference_paper.pdf");

    println!("\nGenerated 5 scientific paper fixtures in tests/fixtures/profiles/scientific_paper/");
    Ok(())
}
