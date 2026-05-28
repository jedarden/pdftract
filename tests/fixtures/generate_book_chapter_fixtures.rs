/// Generate book chapter test fixtures.
///
/// This creates 5 PDF fixtures for book chapter profile testing:
/// 1. novel_chapter - Project Gutenberg novel chapter (public domain)
/// 2. academic_chapter - Academic book chapter (CC-BY license)
/// 3. textbook_chapter - Textbook chapter with figures
/// 4. technical_manual_chapter - Technical manual chapter
/// 5. recipe_book_chapter - Cookbook chapter
///
/// Run with: cargo run --bin generate_book_chapter_fixtures

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Book chapter PDF builder
struct BookChapterBuilder {
    title: String,
    author: String,
    chapter_number: Option<String>,
    sections: Vec<String>,
    content_lines: Vec<String>,
    has_figures: bool,
}

impl BookChapterBuilder {
    fn new(title: &str, author: &str) -> Self {
        Self {
            title: title.to_string(),
            author: author.to_string(),
            chapter_number: None,
            sections: Vec::new(),
            content_lines: Vec::new(),
            has_figures: false,
        }
    }

    fn chapter_number(&mut self, num: &str) {
        self.chapter_number = Some(num.to_string());
    }

    fn add_section(&mut self, section: &str) {
        self.sections.push(section.to_string());
    }

    fn add_content(&mut self, content: &str) {
        self.content_lines.push(content.to_string());
    }

    fn with_figures(&mut self) {
        self.has_figures = true;
    }

    fn build(&self) -> Vec<u8> {
        let mut pdf_data = String::new();

        // PDF header
        pdf_data.push_str("%PDF-1.4\n");
        pdf_data.push_str("%PDF-Magic-Comment\n");

        let mut objects = Vec::new();
        let mut current_id = 1;

        // Calculate page count based on content length
        let page_count = ((self.content_lines.len() / 40) + 2).max(3);

        // Catalog (object 1)
        let catalog = format!("<</Type/Catalog/Pages {} 0 R>>", current_id + 1);
        objects.push(catalog);
        current_id += 1;

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

        // Build page contents
        let mut page_contents = Vec::new();
        let mut current_y = 720;
        let mut current_page_lines: Vec<String> = Vec::new();

        // Page 1: Title, author, chapter number
        let mut page1 = String::new();

        // Chapter number (if present)
        if let Some(ref ch_num) = self.chapter_number {
            page1.push_str(&format!("BT\n50 750 Td\n16 Tf\n(Chapter {}) Tj\nET\n", ch_num));
            current_y -= 40;
        }

        // Title (larger font)
        page1.push_str("BT\n50 ");
        page1.push_str(&current_y.to_string());
        page1.push_str(" Td\n24 Tf\n(");
        page1.push_str(&escape_pdf_string(&self.title));
        page1.push_str(") Tj\nET\n");
        current_y -= 50;

        // Author (below title, smaller font)
        page1.push_str("BT\n50 ");
        page1.push_str(&current_y.to_string());
        page1.push_str(" Td\n12 Tf\n(by ");
        page1.push_str(&escape_pdf_string(&self.author));
        page1.push_str(") Tj\nET\n");
        current_y -= 40;

        // First section heading
        if !self.sections.is_empty() {
            page1.push_str("BT\n50 ");
            page1.push_str(&current_y.to_string());
            page1.push_str(" Td\n14 Tf\n(");
            page1.push_str(&escape_pdf_string(&self.sections[0]));
            page1.push_str(") Tj\nET\n");
            current_y -= 30;
        }

        page_contents.push(page1);

        // Remaining pages with content
        let mut content_idx = 0;
        for page_num in 1..page_count {
            let mut page_content = String::new();
            let mut page_y = 720;

            // Add section heading for this page if applicable
            let section_idx = page_num;
            if section_idx < self.sections.len() {
                page_content.push_str("BT\n50 ");
                page_content.push_str(&page_y.to_string());
                page_content.push_str(" Td\n14 Tf\n(");
                page_content.push_str(&escape_pdf_string(&self.sections[section_idx]));
                page_content.push_str(") Tj\nET\n");
                page_y -= 30;
            }

            // Add content lines
            let lines_per_page = 40;
            let line_count = lines_per_page.min(self.content_lines.len() - content_idx);

            for i in 0..line_count {
                if content_idx + i < self.content_lines.len() {
                    let line = &self.content_lines[content_idx + i];
                    page_content.push_str("BT\n50 ");
                    page_content.push_str(&page_y.to_string());
                    page_content.push_str(" Td\n10 Tf\n(");
                    page_content.push_str(&escape_pdf_string(line));
                    page_content.push_str(") Tj\nET\n");
                    page_y -= 14;
                }
            }

            content_idx += line_count;
            page_contents.push(page_content);
        }

        // Individual page objects
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
        for (i, content) in page_contents.iter().enumerate() {
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
    let fixtures_dir = Path::new("tests/fixtures/profiles/book_chapter");

    // Ensure directory exists
    std::fs::create_dir_all(fixtures_dir)?;

    // 1. Novel chapter (Project Gutenberg style)
    let mut builder = BookChapterBuilder::new("The Mysterious Letter", "Jane Austen");
    builder.chapter_number("1");
    builder.add_section("The Arrival");
    builder.add_section("The Discovery");
    builder.add_section("The Revelation");

    let novel_content = vec![
        "It was a dark and stormy night when the letter arrived at Netherfield Park.",
        "Elizabeth Bennet sat by the candlelight, her hands trembling as she",
        "broke the wax seal. The handwriting was unfamiliar, yet something",
        "about it stirred a memory she could not quite place.",
        "",
        "\"My dear Miss Bennet,\" the letter began, \"I write to you with urgent",
        "news concerning your sister. Please make haste to London at your",
        "earliest convenience. There is much to discuss, and time is of the essence.\"",
        "",
        "The letter was signed simply, \"A Friend.\" Elizabeth's heart raced as",
        "she considered the implications. Who could this mysterious correspondent be?",
        "And what news could they possibly have about her dear sister Jane?",
        "",
        "She rose from her desk and paced the room, the letter clutched in her hand.",
        "The storm outside mirrored the turmoil in her mind. Lightning flashed",
        "across the sky, illuminating the worried expression on her face.",
        "",
        "\"I must depart at first light,\" she whispered to herself. \"Whatever",
        "awaits me in London, I cannot ignore this summons.\"",
        "",
        "The morning brought no relief from her anxiety. Elizabeth packed her bags",
        "with shaking hands, her thoughts racing with possibilities both terrible",
        "and hopeful. What if Jane was in danger? What if this was some cruel hoax?",
        "",
        "As the carriage carried her away from Netherfield, Elizabeth watched the",
        "familiar countryside pass by. Little did she know that this journey would",
        "change everything she believed about her family, her friends, and herself.",
        "",
        "The discovery that awaited her in London would shake the foundations of",
        "her world and reveal secrets long buried. But that is a story for another day.",
    ];
    for line in novel_content {
        builder.add_content(line);
    }

    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("novel_chapter.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created novel_chapter.pdf");

    // 2. Academic book chapter (CC-BY)
    let mut builder = BookChapterBuilder::new("Introduction to Cognitive Psychology", "Dr. Sarah Mitchell");
    builder.add_section("Historical Foundations");
    builder.add_section("Core Concepts");
    builder.add_section("Research Methods");
    builder.chapter_number("2");

    let academic_content = vec![
        "Cognitive psychology emerged as a distinct discipline in the mid-20th century,",
        "marking a shift away from behaviorist approaches toward understanding mental",
        "processes. This chapter explores the historical development, key concepts,",
        "and methodological foundations that define the field today.",
        "",
        "The cognitive revolution of the 1950s and 1960s brought renewed attention to",
        "internal mental states, information processing, and the computational theory",
        "of mind. Pioneers such as George Miller, Ulric Neisser, and Herbert Simon",
        "established frameworks for studying memory, attention, problem-solving, and",
        "language that continue to influence contemporary research.",
        "",
        "Historical Foundations",
        "",
        "The roots of cognitive psychology extend deeper than the mid-20th century.",
        "Wilhelm Wundt's establishment of the first experimental psychology laboratory",
        "in 1879 laid groundwork for systematic investigation of mental processes.",
        "William James's seminal work \"The Principles of Psychology\" (1890) introduced",
        "concepts of stream of consciousness and functionalism that remain relevant.",
        "",
        "Core Concepts",
        "",
        "Modern cognitive psychology operates on several foundational assumptions:",
        "First, mental processes involve information processing analogous to computer",
        "operations. Second, these processes occur in stages with discrete components.",
        "Third, cognitive activity can be inferred from behavior through careful",
        "experimental design.",
        "",
        "Key areas of inquiry include attention, memory, language, perception,",
        "problem-solving, and decision-making. Each domain employs specialized",
        "methodologies while sharing common theoretical frameworks.",
        "",
        "Research Methods",
        "",
        "Cognitive psychologists employ diverse methodologies to investigate mental",
        "processes. Reaction time experiments reveal the temporal dynamics of cognitive",
        "operations. Neuroimaging techniques provide biological correlates of cognitive",
        "function. Computational modeling formalizes theories as testable algorithms.",
    ];
    for line in academic_content {
        builder.add_content(line);
    }

    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("academic_chapter.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created academic_chapter.pdf");

    // 3. Textbook chapter with figures
    let mut builder = BookChapterBuilder::new("Cellular Respiration", "Prof. Michael Chen & Dr. Lisa Rodriguez");
    builder.add_section("Glycolysis");
    builder.add_section("The Krebs Cycle");
    builder.add_section("Electron Transport Chain");
    builder.add_section("ATP Production");
    builder.chapter_number("7");

    let textbook_content = vec![
        "[FIGURE 7.1: Overview of Cellular Respiration]",
        "Cellular respiration is the process by which cells convert nutrients into",
        "energy in the form of ATP. This multi-step process occurs in the cytoplasm",
        "and mitochondria of eukaryotic cells, involving glycolysis, the Krebs cycle,",
        "and oxidative phosphorylation.",
        "",
        "Glycolysis",
        "",
        "Glycolysis occurs in the cytoplasm and does not require oxygen. This pathway",
        "breaks down one molecule of glucose into two molecules of pyruvate, producing",
        "a net gain of 2 ATP and 2 NADH molecules.",
        "",
        "[FIGURE 7.2: Ten Steps of Glycolysis]",
        "The ten enzymatic steps of glycolysis can be grouped into two phases:",
        "1) Energy investment phase (steps 1-5) and 2) Energy payoff phase (steps 6-10).",
        "Key regulatory enzymes include phosphofructokinase (PFK), which catalyzes",
        "the rate-limiting step.",
        "",
        "The Krebs Cycle",
        "",
        "Also known as the citric acid cycle or tricarboxylic acid (TCA) cycle, this",
        "series of reactions occurs in the mitochondrial matrix. Each turn of the",
        "cycle produces 2 CO2 molecules, 3 NADH, 1 FADH2, and 1 GTP (or ATP).",
        "",
        "[TABLE 7.1: Krebs Cycle Enzymes and Products]",
        "The cycle begins when acetyl-CoA combines with oxaloacetate to form citrate.",
        "Through eight enzymatic steps, the carbon skeleton is oxidized, releasing",
        "carbon dioxide and transferring high-energy electrons to NAD+ and FAD.",
        "",
        "Electron Transport Chain",
        "",
        "The electron transport chain (ETC) is located in the inner mitochondrial membrane.",
        "NADH and FADH2 donate electrons to protein complexes I-IV, creating a proton",
        "gradient that drives ATP synthesis.",
    ];
    for line in textbook_content {
        builder.add_content(line);
    }

    builder.with_figures();
    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("textbook_chapter.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created textbook_chapter.pdf");

    // 4. Technical manual chapter
    let mut builder = BookChapterBuilder::new("Engine Maintenance Procedures", "Technical Publications Team");
    builder.chapter_number("4");
    builder.add_section("Oil Change Protocol");
    builder.add_section("Filter Replacement");
    builder.add_section("Scheduled Maintenance Intervals");

    let technical_content = vec![
        "WARNING: Perform all maintenance procedures with engine completely cooled.",
        "Failure to allow adequate cooling time may result in serious burns or injury.",
        "",
        "This chapter describes routine maintenance procedures for Model XJ-900",
        "series engines. Follow all steps in sequence. Do not skip safety precautions.",
        "",
        "Oil Change Protocol",
        "",
        "Step 1: Preparation",
        "- Ensure engine is cool to the touch (minimum 2 hours after operation)",
        "- Position vehicle on level surface",
        "- Gather required tools: drain pan, 14mm socket wrench, oil filter wrench",
        "- Verify replacement oil filter part number: OF-900A",
        "",
        "Step 2: Drain Old Oil",
        "- Place drain pan beneath oil drain plug",
        "- Remove drain plug using 14mm socket wrench",
        "- Allow oil to drain completely (approximately 15 minutes)",
        "- Inspect drained oil for metal particles or unusual discoloration",
        "",
        "Step 3: Replace Oil Filter",
        "- Using oil filter wrench, remove old filter",
        "- Clean filter mounting surface",
        "- Apply thin film of clean oil to new filter gasket",
        "- Install new filter and tighten 3/4 turn after gasket contacts engine",
        "",
        "Filter Replacement",
        "",
        "Air Filter Replacement Interval: Every 12,000 miles or 12 months",
        "Fuel Filter Replacement Interval: Every 24,000 miles or 24 months",
        "Cabin Air Filter Replacement Interval: Every 15,000 miles or 15 months",
        "",
        "Refer to Figure 4.2 for filter locations and access procedures.",
        "Always use genuine manufacturer filters to maintain warranty coverage.",
        "",
        "Scheduled Maintenance Intervals",
        "",
        "Minor Service (7,500 miles): Inspect belts, hoses, fluid levels",
        "Major Service (30,000 miles): Replace spark plugs, coolant, brake fluid",
        "Timing Belt Replacement (90,000 miles): Critical - failure causes severe damage",
    ];
    for line in technical_content {
        builder.add_content(line);
    }

    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("technical_manual_chapter.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created technical_manual_chapter.pdf");

    // 5. Recipe book chapter
    let mut builder = BookChapterBuilder::new("Baking Essentials", "Chef Marie Laurent");
    builder.chapter_number("3");
    builder.add_section("Flour Fundamentals");
    builder.add_section("Leavening Agents");
    builder.add_section("Sweeteners and Fats");

    let recipe_content = vec![
        "Welcome to the wonderful world of baking! This chapter introduces the",
        "fundamental ingredients and techniques that form the foundation of all",
        "successful baking. Understanding how these components interact will help",
        "you achieve consistent, delicious results.",
        "",
        "Flour Fundamentals",
        "",
        "Flour provides structure through gluten formation when hydrated and agitated.",
        "Different flour types produce varying results due to protein content:",
        "",
        "• Cake flour (6-8% protein): Tender, fine crumb. Best for: cakes, muffins",
        "• All-purpose flour (10-12% protein): Versatile standard. Best for: cookies, brownies",
        "• Bread flour (12-14% protein): Chewy, structured. Best for: bread, pizza dough",
        "",
        "Measuring flour accurately is critical. For best results, use the spoon-and-level",
        "method: spoon flour into measuring cup, level with straight edge. Avoid packing",
        "or tapping, which compacts flour and leads to dry baked goods.",
        "",
        "Leavening Agents",
        "",
        "Leavening creates lift and texture through gas production during baking.",
        "Understanding each agent's characteristics ensures proper selection and use.",
        "",
        "Baking Powder: Combination of baking soda + cream of tartar (acid).",
        "Double-acting powder reacts twice: once when wet, again when heated.",
        "Typical ratio: 1 teaspoon per cup of flour.",
        "",
        "Baking Soda: Pure sodium bicarbonate. Requires acidic ingredient",
        "(buttermilk, yogurt, citrus, vinegar) to activate. Creates stronger",
        "rise than baking powder. Typical ratio: 1/4 teaspoon per cup of flour.",
        "",
        "Yeast: Living organism that ferments sugars, producing CO2 and ethanol.",
        "Active dry yeast requires proofing in warm water (105-110°F). Instant yeast",
        "can be added directly to dry ingredients. Always check expiration dates.",
        "",
        "Sweeteners and Fats",
        "",
        "Sugar provides sweetness, tenderizing, browning, and moisture retention.",
        "Different sugars produce different results:",
        "",
        "Granulated white sugar: Standard choice, neutral flavor profile",
        "Brown sugar: Contains molasses, adds moisture and caramel notes",
        "Confectioners' sugar: Finely ground with cornstarch, ideal for frostings",
        "",
        "Fats contribute tenderness, flavor, and mouthfeel. Butter offers rich flavor",
        "but solidifies at room temperature. Oil produces moist, tender crumb but less",
        "flavor. For best of both worlds, many recipes use a combination.",
    ];
    for line in recipe_content {
        builder.add_content(line);
    }

    let pdf_data = builder.build();
    let mut file = File::create(fixtures_dir.join("recipe_book_chapter.pdf"))?;
    file.write_all(&pdf_data)?;
    println!("Created recipe_book_chapter.pdf");

    println!("\nGenerated 5 book chapter fixtures in tests/fixtures/profiles/book_chapter/");
    Ok(())
}
