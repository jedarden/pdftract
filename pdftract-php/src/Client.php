<?php

namespace Jedarden\Pdftract;

// Import main client implementation
require_once __DIR__ . '/Codegen/Methods.php';
require_once __DIR__ . '/Codegen/Errors.php';

// Import models
require_once __DIR__ . '/Models/Types.php';

// Re-export for convenience
class_alias(
    \Jedarden\Pdftract\Client::class,
    \Jedarden\Pdftract\PdftractClient::class
);
