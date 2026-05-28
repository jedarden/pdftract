# Book Chapter Profile Fixtures - Provenance

## novel_chapter.pdf

**Source**: Synthetic fixture inspired by Project Gutenberg public domain novels
**Type**: Narrative fiction chapter in the style of 19th-century English literature
**License**: CC0 (public domain - synthetic content)
**PII**: None - fictional content with period-appropriate style
**Key Fields**:
- Title: The Mysterious Letter
- Chapter Number: 1
- Author: Jane Austen (period-appropriate attribution style)
- Sections: The Arrival, The Discovery, The Revelation
- Content: Narrative fiction with period language, dialogue, and descriptive passages
- Length: ~3 pages of narrative text

## academic_chapter.pdf

**Source**: Synthetic academic book chapter
**Type**: Scholarly monograph chapter with structured academic content
**License**: CC-BY 4.0
**PII**: None - synthetic academic content with realistic structure
**Key Fields**:
- Title: Introduction to Cognitive Psychology
- Chapter Number: 2
- Author: Dr. Sarah Mitchell
- Sections: Historical Foundations, Core Concepts, Research Methods
- Content: Academic prose with citations, theoretical frameworks, methodological discussion
- References to: George Miller, Ulric Neisser, Herbert Simon, Wilhelm Wundt, William James

## textbook_chapter.pdf

**Source**: Synthetic educational textbook chapter
**Type**: Biology textbook chapter with pedagogical structure
**License**: CC-BY 4.0
**PII**: None - synthetic educational content
**Key Fields**:
- Title: Cellular Respiration
- Chapter Number: 7
- Author: Prof. Michael Chen & Dr. Lisa Rodriguez
- Sections: Glycolysis, The Krebs Cycle, Electron Transport Chain, ATP Production
- Content: Educational content with figure references, table references, numbered steps
- Features: Figure placeholders (FIGURE 7.1, FIGURE 7.2), table references (TABLE 7.1)

## technical_manual_chapter.pdf

**Source**: Synthetic technical manual chapter
**Type**: Engine maintenance procedures with safety warnings
**License**: CC0 (public domain - synthetic technical content)
**PII**: None - generic technical procedures
**Key Fields**:
- Title: Engine Maintenance Procedures
- Chapter Number: 4
- Author: Technical Publications Team
- Sections: Oil Change Protocol, Filter Replacement, Scheduled Maintenance Intervals
- Content: Procedural instructions with numbered steps, warnings, specifications
- Features: Safety warnings (WARNING:), numbered lists, part numbers (OF-900A)

## recipe_book_chapter.pdf

**Source**: Synthetic cookbook chapter
**Type**: Baking fundamentals with instructional content
**License**: CC-BY 4.0
**PII**: None - synthetic culinary content
**Key Fields**:
- Title: Baking Essentials
- Chapter Number: 3
- Author: Chef Marie Laurent
- Sections: Flour Fundamentals, Leavening Agents, Sweeteners and Fats
- Content: Culinary instruction with ingredient lists, technique descriptions, measurements
- Features: Ingredient types (cake flour, all-purpose flour, bread flour), ratios, temperatures

## Notes

- All fixtures are synthetic PDFs created programmatically via `generate_book_chapter_fixtures.rs`
- Expected outputs document the ground truth for profile field extraction
- Chapter numbers follow numeric format (1, 2, 3, etc.) - Roman numerals and non-numeric formats are known limitations
- Sections are extracted as per-page heading collections - nested section hierarchies are flattened
- Author attribution follows the format specified in the fixture (single author, multiple authors, institutional authors)
