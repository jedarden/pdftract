<?php

declare(strict_types=1);

namespace Jedarden\Pdftract\Models;

/**
 * Readonly classification model
 *
 * Simple readonly representation of document classification results
 */
class Classification
{
    /**
     * @param string $type Classification type (e.g., "invoice", "contract", "report")
     * @param float $confidence Confidence score between 0.0 and 1.0
     */
    public function __construct(
        public readonly string $type,
        public readonly float $confidence
    ) {}
}
