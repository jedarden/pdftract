<?php

declare(strict_types=1);

namespace Jedarden\Pdftract\Models;

/**
 * Readonly document model
 *
 * Simple readonly representation of a PDF document with basic properties
 */
class Document
{
    /**
     * @param string $path File path to the PDF document
     * @param int $pageCount Total number of pages in the document
     * @param array<int, Page> $pages Array of Page objects
     */
    public function __construct(
        public readonly string $path,
        public readonly int $pageCount,
        public readonly array $pages
    ) {}
}
