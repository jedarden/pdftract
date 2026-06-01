<?php

declare(strict_types=1);

namespace Jedarden\Pdftract\Models;

/**
 * Readonly page model
 *
 * Simple readonly representation of a PDF page
 */
class Page
{
    /**
     * @param int $number Page number (1-based)
     * @param string $text Extracted text content from the page
     * @param array<string, mixed>|null $structure Optional structure/tree data for the page
     */
    public function __construct(
        public readonly int $number,
        public readonly string $text,
        public readonly ?array $structure
    ) {}
}
