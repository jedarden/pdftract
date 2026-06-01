<?php

declare(strict_types=1);

namespace Jedarden\Pdftract\Models;

/**
 * Readonly match model
 *
 * Simple readonly representation of a content match within a document
 */
class Match
{
    /**
     * @param int $page Page number where the match was found (1-based)
     * @param string $context Text context surrounding the match
     * @param int $startIndex Starting character index of the match
     * @param int $endIndex Ending character index of the match
     */
    public function __construct(
        public readonly int $page,
        public readonly string $context,
        public readonly int $startIndex,
        public readonly int $endIndex
    ) {}
}
