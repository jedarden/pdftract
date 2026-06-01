<?php

declare(strict_types=1);

namespace Jedarden\Pdftract\Models;

/**
 * Readonly metadata model
 *
 * Simple readonly representation of PDF document metadata
 */
class Metadata
{
    /**
     * @param string $title Document title
     * @param string $author Document author
     * @param string|null $subject Optional document subject
     * @param array<string>|null $keywords Optional array of keywords
     */
    public function __construct(
        public readonly string $title,
        public readonly string $author,
        public readonly ?string $subject,
        public readonly ?array $keywords
    ) {}
}
