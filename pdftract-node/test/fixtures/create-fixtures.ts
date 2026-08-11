#!/usr/bin/env node
/**
 * Create valid PDF fixtures for testing
 *
 * This script generates minimal but valid PDF files for testing the SDK.
 */

import { writeFileSync } from 'fs';
import { join } from 'path';

/**
 * Create a minimal valid PDF with one page containing "Hello World"
 */
function createMinimalHelloPdf(): Buffer {
  // A minimal valid PDF with one page
  const pdf = `%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Count 1
/Kids [3 0 R]
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
/Resources <<
/Font <<
/F1 <<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
>>
>>
>>
endobj
4 0 obj
<<
/Length 44
>>
stream
BT
/F1 12 Tf
100 700 Td
(Hello World) Tj
ET
endstream
endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000296 00000 n
trailer
<<
/Size 5
/Root 1 0 R
>>
startxref
393
%%EOF
`;

  return Buffer.from(pdf, 'utf-8');
}

/**
 * Create a minimal PDF with metadata
 */
function createPdfWithMetadata(): Buffer {
  const pdf = `%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Count 1
/Kids [3 0 R]
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
/Resources <<
/Font <<
/F1 <<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
>>
>>
>>
endobj
4 0 obj
<<
/Length 52
>>
stream
BT
/F1 12 Tf
100 700 Td
(Test Document) Tj
100 680 Td
(Page 1 Content) Tj
ET
endstream
endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000296 00000 n
trailer
<<
/Size 5
/Root 1 0 R
/Title (Test PDF)
/Author (Test Author)
/Subject (Test Subject)
>>
startxref
393
%%EOF
`;

  return Buffer.from(pdf, 'utf-8');
}

/**
 * Create a multi-page PDF
 */
function createMultiPagePdf(): Buffer {
  const pdf = `%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Count 3
/Kids [3 0 R 5 0 R 7 0 R]
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
/Resources <<
/Font <<
/F1 <<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
>>
>>
>>
endobj
4 0 obj
<<
/Length 48
>>
stream
BT
/F1 12 Tf
100 700 Td
(Page 1) Tj
ET
endstream
endobj
5 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 6 0 R
/Resources <<
/Font <<
/F1 <<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
>>
>>
>>
endobj
6 0 obj
<<
/Length 48
>>
stream
BT
/F1 12 Tf
100 700 Td
(Page 2) Tj
ET
endstream
endobj
7 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 8 0 R
/Resources <<
/Font <<
/F1 <<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
>>
>>
>>
endobj
8 0 obj
<<
/Length 48
>>
stream
BT
/F1 12 Tf
100 700 Td
(Page 3) Tj
ET
endstream
endobj
xref
0 9
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000304 00000 n
0000000425 00000 n
0000000546 00000 n
0000000667 00000 n
0000000788 00000 n
trailer
<<
/Size 9
/Root 1 0 R
>>
startxref
901
%%EOF
`;

  return Buffer.from(pdf, 'utf-8');
}

// Create output directory
const fixturesDir = join(process.cwd(), 'test', 'fixtures', 'pdfs');

// Write PDF files
writeFileSync(join(fixturesDir, 'minimal-hello.pdf'), createMinimalHelloPdf());
writeFileSync(join(fixturesDir, 'minimal-metadata.pdf'), createPdfWithMetadata());
writeFileSync(join(fixturesDir, 'minimal-multipage.pdf'), createMultiPagePdf());

console.log('Created PDF fixtures:');
console.log('  - minimal-hello.pdf');
console.log('  - minimal-metadata.pdf');
console.log('  - minimal-multipage.pdf');
