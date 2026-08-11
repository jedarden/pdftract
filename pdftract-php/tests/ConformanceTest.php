<?php

namespace Jedarden\Pdftract\Tests;

use Jedarden\Pdftract\Client;
use Jedarden\Pdftract\PdftractException;
use PHPUnit\Framework\TestCase;

class ConformanceTest extends TestCase
{
    private Client $client;
    private string $fixturePath;

    protected function setUp(): void
    {
        $this->client = new Client();
        $this->fixturePath = __DIR__ . '/../fixtures/vector/academic-paper-2col.pdf';
    }

    public function testExtractReturnsDocument()
    {
        $document = $this->client->extract($this->fixturePath);

        $this->assertEquals('1.0', $document->schemaVersion);
        $this->assertGreaterThan(0, $document->getPageCount());
        $this->assertIsArray($document->pages);
        $this->assertInstanceOf(\Jedarden\Pdftract\Models\Metadata::class, $document->metadata);
    }

    public function testExtractTextReturnsString()
    {
        $text = $this->client->extractText($this->fixturePath);

        $this->assertIsString($text);
        $this->assertNotEmpty($text);
    }

    public function testExtractMarkdownReturnsString()
    {
        $markdown = $this->client->extractMarkdown($this->fixturePath);

        $this->assertIsString($markdown);
        $this->assertNotEmpty($markdown);
    }

    public function testExtractStreamYieldsPages()
    {
        $pages = iterator_to_array($this->client->extractStream($this->fixturePath));

        $this->assertGreaterThan(0, count($pages));
        $this->assertInstanceOf(\Jedarden\Pdftract\Models\Page::class, $pages[0]);
    }

    public function testGetMetadataReturnsMetadata()
    {
        $metadata = $this->client->getMetadata($this->fixturePath);

        $this->assertInstanceOf(\Jedarden\Pdftract\Models\Metadata::class, $metadata);
        $this->assertGreaterThan(0, $metadata->pageCount);
    }

    public function testHashReturnsFingerprint()
    {
        $fingerprint = $this->client->hash($this->fixturePath);

        $this->assertInstanceOf(\Jedarden\Pdftract\Models\Fingerprint::class, $fingerprint);
        $this->assertNotEmpty($fingerprint->hash);
        $this->assertGreaterThan(0, $fingerprint->pageCount);
    }

    public function testClassifyReturnsClassification()
    {
        $classification = $this->client->classify($this->fixturePath);

        $this->assertInstanceOf(\Jedarden\Pdftract\Models\Classification::class, $classification);
        $this->assertNotEmpty($classification->category);
    }

    public function testSearchYieldsMatches()
    {
        $matches = iterator_to_array($this->client->search($this->fixturePath, 'abstract'));

        $this->assertIsArray($matches);
        // May be empty if no matches found
    }

    public function testNonexistentFileThrowsError()
    {
        $this->expectException(PdftractException::class);
        $this->client->extract('/nonexistent/file.pdf');
    }

    public function testCorruptPdfThrowsCorruptPdfError()
    {
        $this->expectException(\Jedarden\Pdftract\CorruptPdfError::class);
        $this->client->extract(__DIR__ . '/../fixtures/corrupt.pdf');
    }
}
