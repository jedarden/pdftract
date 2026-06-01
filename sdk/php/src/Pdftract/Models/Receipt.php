<?php

declare(strict_types=1);

namespace Jedarden\Pdftract\Models;

/**
 * Readonly receipt model
 *
 * Simple readonly representation of a document receipt for verification
 */
class Receipt
{
    /**
     * @param string $id Unique receipt identifier
     * @param int $pageCount Total number of pages in the document
     * @param string $contentHash Hash of the document content
     */
    public function __construct(
        public readonly string $id,
        public readonly int $pageCount,
        public readonly string $contentHash
    ) {}
}
