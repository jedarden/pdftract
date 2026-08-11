<?php

namespace Jedarden\Pdftract\Models;

/**
 * Document structure.
 */
readonly class Document
{
    public string $schemaVersion;
    public array $pages;
    public Metadata $metadata;

    public function __construct(array $data)
    {
        $this->schemaVersion = $data['schema_version'] ?? '1.0';
        $this->pages = array_map(fn($p) => new Page($p), $data['pages'] ?? []);
        $this->metadata = new Metadata($data['metadata'] ?? []);
    }

    /**
     * Get page count.
     */
    public function getPageCount(): int
    {
        return count($this->pages);
    }
}

/**
 * Page structure.
 */
readonly class Page
{
    public int $page;
    public float $width;
    public float $height;
    public int $rotation;
    public array $spans;
    public array $blocks;

    public function __construct(array $data)
    {
        $this->page = $data['page'] ?? 1;
        $this->width = $data['width'] ?? 0.0;
        $this->height = $data['height'] ?? 0.0;
        $this->rotation = $data['rotation'] ?? 0;
        $this->spans = $data['spans'] ?? [];
        $this->blocks = array_map(fn($b) => new Block($b), $data['blocks'] ?? []);
    }

    /**
     * Get plain text from page.
     */
    public function getText(): string
    {
        return implode('', array_map(fn($s) => $s['text'] ?? '', $this->spans));
    }
}

/**
 * Block structure (paragraph, heading, table, figure, list).
 */
readonly class Block
{
    public string $kind;
    public string $text;
    public array $bbox;
    public ?int $level;

    public function __construct(array $data)
    {
        $this->kind = $data['kind'] ?? 'paragraph';
        $this->text = $data['text'] ?? '';
        $this->bbox = $data['bbox'] ?? [0, 0, 0, 0];
        $this->level = $data['level'] ?? null;
    }
}

/**
 * Match result from search.
 */
readonly class Match
{
    public string $text;
    public int $page;
    public array $bbox;
    public array $context;

    public function __construct(array $data)
    {
        $this->text = $data['text'] ?? '';
        $this->page = $data['page'] ?? 1;
        $this->bbox = $data['bbox'] ?? [0, 0, 0, 0];
        $this->context = [
            'before' => $data['context']['before'] ?? '',
            'after' => $data['context']['after'] ?? '',
        ];
    }
}

/**
 * Document metadata.
 */
readonly class Metadata
{
    public ?string $title;
    public ?string $author;
    public ?string $subject;
    public array $keywords;
    public ?string $creator;
    public ?string $producer;
    public ?string $created;
    public ?string $modified;
    public int $pageCount;

    public function __construct(array $data)
    {
        $this->title = $data['title'] ?? null;
        $this->author = $data['author'] ?? null;
        $this->subject = $data['subject'] ?? null;
        $this->keywords = $data['keywords'] ?? [];
        $this->creator = $data['creator'] ?? null;
        $this->producer = $data['producer'] ?? null;
        $this->created = $data['created'] ?? null;
        $this->modified = $data['modified'] ?? null;
        $this->pageCount = $data['page_count'] ?? 0;
    }
}

/**
 * Document fingerprint for deduplication.
 */
readonly class Fingerprint
{
    public string $hash;
    public int $pageCount;
    public string $fastHash;
    public Metadata $metadata;

    public function __construct(array $data)
    {
        $this->hash = $data['hash'] ?? '';
        $this->pageCount = $data['page_count'] ?? 0;
        $this->fastHash = $data['fast_hash'] ?? '';
        $this->metadata = new Metadata($data['metadata'] ?? []);
    }
}

/**
 * Document classification result.
 */
readonly class Classification
{
    public string $category;
    public float $confidence;
    public array $tags;
    public array $heuristics;

    public function __construct(array $data)
    {
        $this->category = $data['category'] ?? 'unknown';
        $this->confidence = $data['confidence'] ?? 0.0;
        $this->tags = $data['tags'] ?? [];
        $this->heuristics = $data['heuristics'] ?? [];
    }
}

/**
 * Receipt for verification.
 */
readonly class Receipt
{
    public string $hash;
    public int $pageCount;
    public string $timestamp;

    public function __construct(array $data)
    {
        $this->hash = $data['hash'] ?? '';
        $this->pageCount = $data['page_count'] ?? 0;
        $this->timestamp = $data['timestamp'] ?? '';
    }

    /**
     * Create receipt from JSON string.
     */
    public static function fromJson(string $json): self
    {
        $data = json_decode($json, true);
        if (json_last_error() !== JSON_ERROR_NONE) {
            throw new \InvalidArgumentException("Invalid receipt JSON: " . json_last_error_msg());
        }
        return new self($data);
    }

    /**
     * Convert to JSON string.
     */
    public function toJson(): string
    {
        return json_encode([
            'hash' => $this->hash,
            'page_count' => $this->pageCount,
            'timestamp' => $this->timestamp,
        ]);
    }
}
