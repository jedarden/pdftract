/// Generate slide deck test fixtures.
///
/// This creates 5 PDF fixtures for slide deck profile testing:
/// 1. pitch_deck - Sales pitch deck (10 slides)
/// 2. academic_lecture - Academic lecture (40 slides)
/// 3. corporate_kickoff - Corporate kickoff (15 slides)
/// 4. bilingual_deck - Bilingual English/Spanish (12 slides)
/// 5. googleslides_handout - Google Slides handout mode (4 pages, 3 slides per page)
///
/// Run with: cargo run --bin generate_slide_deck_fixtures

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Simple slide deck PDF builder
struct SlideDeckBuilder {
    slide_titles: Vec<String>,
    title: String,
    author: String,
}

impl SlideDeckBuilder {
    fn new(title: &str, author: &str) -> Self {
        Self {
            slide_titles: Vec::new(),
            title: title.to_string(),
            author: author.to_string(),
        }
    }

    fn add_slide(&mut self, title: &str) {
        self.slide_titles.push(title.to_string());
    }

    fn build(&self) -> Vec<u8> {
        let mut pdf_data = String::new();

        // PDF header (use a simpler comment to avoid UTF-8 issues)
        pdf_data.push_str("%PDF-1.4\n");
        pdf_data.push_str("%PDF-Magic-Comment\n");

        // We'll build a simple PDF with:
        // - Object 1: Catalog
        // - Object 2: Pages (root)
        // - Objects 3+: Individual pages
        // - Each page has its own content stream

        let page_count = self.slide_titles.len();
        let mut objects = Vec::new();
        let mut current_id = 1;

        // Catalog (will be object 1)
        let catalog = format!("<</Type/Catalog/Pages {} 0 R>>", current_id + 1);
        objects.push(catalog);
        current_id += 1;

        // Pages root (will be object 2)
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

        // Individual pages
        for (i, slide_title) in self.slide_titles.iter().enumerate() {
            let page_num = i + 1;
            let content_stream = format!(
                "BT\n50 {} Td\n24 Tf\n({}) Tj\nET\n",
                700 - (i % 3) * 50, // Vary position slightly for visual distinction
                escape_pdf_string(slide_title)
            );

            let content_id = current_id + page_count + 1 + (page_num as usize);

            let page = format!(
                "<</Type/Page/Parent {} 0 R/Contents {} 0 R>>",
                2, // Parent is always object 2
                content_id
            );
            objects.push(page);
        }

        // Font object
        let font = "<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>";
        objects.push(font.to_string());

        // Content streams (one per page)
        for slide_title in &self.slide_titles {
            let content = format!(
                "BT\n50 700 Td\n24 Tf\n({}) Tj\nET\n",
                escape_pdf_string(slide_title)
            );
            let content_with_len = format!(
                "<</Length {}>>\nstream\n{}\nendstream",
                content.len(),
                content
            );
            objects.push(content_with_len);
        }

        // Info object
        let info = format!(
            "<</Title({})/Author({})/Producer(pdftract-test)>>",
            escape_pdf_string(&self.title),
            escape_pdf_string(&self.author)
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

fn main() -> std::io::Result<()> {
    let fixtures_dir = Path::new("tests/fixtures/profiles/slide_deck");

    // Ensure directory exists
    std::fs::create_dir_all(fixtures_dir)?;

    // 1. Pitch deck (10 slides)
    let mut builder = SlideDeckBuilder::new("Q3 2024 Product Roadmap", "Jane Smith, VP Product");
    let pitch_titles = vec![
        "Q3 2024 Product Roadmap",
        "Agenda",
        "Market Overview",
        "Product Vision",
        "Key Features",
        "Technical Architecture",
        "Go-to-Market Strategy",
        "Pricing & Packaging",
        "Next Steps",
        "Q&A",
    ];
    for title in &pitch_titles {
        builder.add_slide(title);
    }
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("pitch_deck.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created pitch_deck.pdf");

    // 2. Academic lecture (40 slides)
    let mut builder = SlideDeckBuilder::new("Introduction to Machine Learning", "Prof. Robert Chen, PhD");
    let academic_titles = vec![
        "Introduction to Machine Learning",
        "Overview",
        "What is a Neural Network?",
        "Perceptrons",
        "Multi-Layer Networks",
        "Activation Functions",
        "Backpropagation",
        "Loss Functions",
        "Optimization",
        "Regularization",
        "Convolutional Networks",
        "Recurrent Networks",
        "Transformer Architecture",
        "Attention Mechanisms",
        "Training Strategies",
        "Hyperparameter Tuning",
        "Evaluation Metrics",
        "Case Studies",
        "Current Research",
        "Future Directions",
        "Summary",
        "References",
        "Q1",
        "Q2",
        "Q3",
        "Q4",
        "Q5",
        "Q6",
        "Q7",
        "Q8",
        "Q9",
        "Q10",
        "Q11",
        "Q12",
        "Q13",
        "Q14",
        "Q15",
        "Q16",
        "Thank You",
    ];
    for title in &academic_titles {
        builder.add_slide(title);
    }
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("academic_lecture.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created academic_lecture.pdf");

    // 3. Corporate kickoff (15 slides)
    let mut builder = SlideDeckBuilder::new("2025 Annual Kickoff", "Michael Johnson, CEO");
    let corporate_titles = vec![
        "2025 Annual Kickoff",
        "Welcome",
        "2024 Recap",
        "Financial Highlights",
        "Customer Success Stories",
        "Product Roadmap 2025",
        "Market Expansion",
        "Team Growth",
        "Strategic Priorities",
        "OKR Framework",
        "Investment Areas",
        "Culture & Values",
        "Events Calendar",
        "Leadership Team",
        "Thank You",
    ];
    for title in &corporate_titles {
        builder.add_slide(title);
    }
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("corporate_kickoff.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created corporate_kickoff.pdf");

    // 4. Bilingual deck (12 slides)
    let mut builder = SlideDeckBuilder::new("Informe Anual 2024", "Maria Garcia / Director General");
    let bilingual_titles = vec![
        "Informe Anual 2024",
        "Resumen Ejecutivo",
        "Logros 2024",
        "Crecimiento de Ingresos",
        "Expansión Global",
        "Productos Nuevos",
        "Sostenibilidad",
        "Compromiso Social",
        "Perspectivas 2025",
        "Estrategia",
        "Próximos Pasos",
        "Gracias",
    ];
    for title in &bilingual_titles {
        builder.add_slide(title);
    }
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("bilingual_deck.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created bilingual_deck.pdf");

    // 5. Google Slides handout (4 pages with multiple titles each)
    let mut builder = SlideDeckBuilder::new("Team Onboarding Guide", "HR Department");
    let handout_titles = vec![
        "Welcome!",
        "Company Values",
        "Our Mission",
        "Tools & Resources",
        "Benefits Overview",
        "Who's Who",
        "First Week Checklist",
        "Questions?",
        "Contact HR",
        "Thank You",
        "Insurance",
        "401k",
        "PTO Policy",
        "Remote Work",
        "Emergency Contacts",
    ];
    // For handout mode, each page shows multiple slide titles
    let handout_pages = vec![
        "Welcome! - Company Values - Our Mission",
        "Tools & Resources - Benefits Overview - Who's Who",
        "First Week Checklist - Questions? - Contact HR",
        "Thank You - Insurance - 401k - PTO Policy - Remote Work - Emergency Contacts",
    ];
    for page_title in &handout_pages {
        builder.add_slide(page_title);
    }
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("googleslides_handout.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created googleslides_handout.pdf");

    println!("\nGenerated 5 slide deck fixtures in tests/fixtures/profiles/slide_deck/");
    Ok(())
}
