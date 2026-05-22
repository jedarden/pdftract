package com.jedarden.pdftract;

/**
 * The PDF file is corrupt or invalid.
 */
public class CorruptPdfException extends PdftractException {
    public CorruptPdfException(String message, int exitCode) {
        super(message, exitCode);
    }

    public CorruptPdfException(String message, int exitCode, String stderr) {
        super(message, exitCode, stderr);
    }

    public CorruptPdfException(String message, int exitCode, Throwable cause) {
        super(message, exitCode, cause);
    }
}
