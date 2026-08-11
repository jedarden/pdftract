<?php

namespace Jedarden\Pdftract;

/**
 * Base exception for all pdftract errors.
 */
class PdftractException extends \Exception
{
    public int $exitCode;

    public function __construct(string $message, int $exitCode, ?\Throwable $previous = null)
    {
        $this->exitCode = $exitCode;
        parent::__construct($message, $exitCode, $previous);
    }
}

/**
 * Corrupt PDF error (exit code 2).
 */
class CorruptPdfError extends PdftractException
{
    public function __construct(string $message, int $exitCode = 2, ?\Throwable $previous = null)
    {
        parent::__construct($message, $exitCode, $previous);
    }
}

/**
 * Encryption error (exit code 3).
 */
class EncryptionError extends PdftractException
{
    public function __construct(string $message, int $exitCode = 3, ?\Throwable $previous = null)
    {
        parent::__construct($message, $exitCode, $previous);
    }
}

/**
 * Source unreachable error (exit code 4).
 */
class SourceUnreachableError extends PdftractException
{
    public function __construct(string $message, int $exitCode = 4, ?\Throwable $previous = null)
    {
        parent::__construct($message, $exitCode, $previous);
    }
}

/**
 * Remote fetch interrupted error (exit code 5).
 */
class RemoteFetchInterruptedError extends PdftractException
{
    public function __construct(string $message, int $exitCode = 5, ?\Throwable $previous = null)
    {
        parent::__construct($message, $exitCode, $previous);
    }
}

/**
 * TLS error (exit code 6).
 */
class TlsError extends PdftractException
{
    public function __construct(string $message, int $exitCode = 6, ?\Throwable $previous = null)
    {
        parent::__construct($message, $exitCode, $previous);
    }
}

/**
 * Receipt verification error (exit code 10).
 */
class ReceiptVerifyError extends PdftractException
{
    public function __construct(string $message, int $exitCode = 10, ?\Throwable $previous = null)
    {
        parent::__construct($message, $exitCode, $previous);
    }
}
