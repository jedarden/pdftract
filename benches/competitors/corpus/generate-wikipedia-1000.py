#!/usr/bin/env python3
"""
Generate a synthetic Wikipedia-like PDF for the grep benchmark.

This creates a PDF with 1000 pages, each containing a repeated pattern
that includes common words like "the" for grep benchmarking.
"""

from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import getSampleStyleSheet
from reportlab.lib.units import inch
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer
from reportlab.lib.enums import TA_JUSTIFY
import sys

def generate_wikipedia_1000(output_path):
    """Generate a 1000-page Wikipedia-like PDF."""

    doc = SimpleDocTemplate(
        output_path,
        pagesize=letter,
        rightMargin=72,
        leftMargin=72,
        topMargin=72,
        bottomMargin=18
    )

    styles = getSampleStyleSheet()
    style_normal = styles["BodyText"]
    style_normal.alignment = TA_JUSTIFY
    style_normal.fontName = "Helvetica"
    style_normal.fontSize = 10

    # Generate content for 1000 pages
    # Each page will have a Wikipedia-like article structure
    # with common words like "the", "and", "of", "in", etc.

    story = []

    # Wikipedia-like article template
    article_templates = [
        """
        <b>Article {page}</b>

        The quick brown fox jumps over the lazy dog. This is a sample sentence that contains
        the word "the" multiple times. The purpose of this document is to provide a consistent
        benchmark for testing grep functionality across different PDF extraction tools.

        The Wikipedia encyclopedia is a free online encyclopedia that anyone can edit. The
        word "the" appears frequently in English text, making it an ideal search term for
        benchmarking purposes. The grep command searches for patterns in text files.
        """,

        """
        <b>History of {page}</b>

        The history of the world is the record of the past events and the memory of those
        events. The study of history is important for understanding the present and planning
        for the future. The word "history" comes from the Greek word "historia" meaning
        inquiry or investigation.

        The development of writing systems allowed civilizations to record their history.
        The invention of the printing press in the 15th century revolutionized the way
        information was disseminated. The internet has transformed access to historical
        records in the modern era.
        """,

        """
        <b>Science and {page}</b>

        The scientific method is a systematic approach to acquiring knowledge about the
        natural world. The method involves making observations, forming hypotheses, conducting
        experiments, and drawing conclusions. The principles of science are based on evidence
        and logical reasoning.

        The fields of physics, chemistry, and biology form the foundation of natural science.
        The applications of scientific knowledge have led to technological advances that
        have transformed society. The pursuit of scientific understanding continues to drive
        innovation and discovery.
        """
    ]

    # Generate 1000 pages
    for page_num in range(1, 1001):
        template = article_templates[(page_num - 1) % len(article_templates)]
        content = template.format(page=page_num)

        # Add the content as a paragraph
        p = Paragraph(content, style_normal)
        story.append(p)
        story.append(Spacer(1, 0.2 * inch))

        # Add some filler text to fill the page
        filler = """
        The quick brown fox jumps over the lazy dog. The five boxing wizards jump quickly.
        The pack of myrrh and jugs of quinine helped cure the malaria. The job requires
        extraordinary skill and patience. The expedition discovered new species of plants
        and animals in the uncharted territory.
        """ * 3

        p2 = Paragraph(filler, style_normal)
        story.append(p2)
        story.append(Spacer(1, 0.1 * inch))

    # Build the PDF
    print(f"Generating {output_path} with 1000 pages...")
    doc.build(story)
    print(f"Successfully generated {output_path}")

if __name__ == "__main__":
    output_path = sys.argv[1] if len(sys.argv) > 1 else "wikipedia-1000.pdf"
    generate_wikipedia_1000(output_path)
