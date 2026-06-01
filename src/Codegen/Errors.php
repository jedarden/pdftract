<?php

namespace Jedarden\Pdftract\Exceptions;

/**
 * Base exception class for all pdftract exceptions.
 */
class PdftractException extends \Exception
{
}

/**
 * Thrown when a PDF source file cannot be found or accessed.
 */
class SourceNotFoundException extends PdftractException
{
}

/**
 * Thrown when a PDF feature is not supported by the parser.
 */
class UnsupportedFeatureException extends PdftractException
{
}

/**
 * Thrown when a PDF file is corrupted or malformed.
 */
class CorruptPdfException extends PdftractException
{
}

/**
 * Thrown when a receipt doesn't match the expected hash or fingerprint.
 */
class ReceiptMismatchException extends PdftractException
{
}

/**
 * Thrown when PDF encryption cannot be handled.
 */
class EncryptionException extends PdftractException
{
}

/**
 * Thrown when OCR processing fails.
 */
class OcrException extends PdftractException
{
}

/**
 * Thrown when content extraction fails.
 */
class ExtractionException extends PdftractException
{
}

/**
 * Thrown when the pdftract server encounters an error.
 */
class ServerException extends PdftractException
{
}
