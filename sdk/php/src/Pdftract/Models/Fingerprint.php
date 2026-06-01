<?php

declare(strict_types=1);

namespace Jedarden\Pdftract\Models;

/**
 * Readonly fingerprint model
 *
 * Simple readonly representation of a PDF document fingerprint
 */
class Fingerprint
{
    /**
     * @param string $id Unique fingerprint identifier
     * @param int $pageCount Total number of pages in the document
     * @param string $contentHash Hash of the document content
     * @param string $structureHash Hash of the document structure
     */
    public function __construct(
        public readonly string $id,
        public readonly int $pageCount,
        public readonly string $contentHash,
        public readonly string $structureHash
    ) {}
}
