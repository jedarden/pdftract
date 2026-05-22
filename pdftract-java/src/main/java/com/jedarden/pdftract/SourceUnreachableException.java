package com.jedarden.pdftract;

/**
 * The source (file or URL) is unreadable.
 */
public class SourceUnreachableException extends PdftractException {
    public SourceUnreachableException(String message, int exitCode) {
        super(message, exitCode);
    }

    public SourceUnreachableException(String message, int exitCode, String stderr) {
        super(message, exitCode, stderr);
    }

    public SourceUnreachableException(String message, int exitCode, Throwable cause) {
        super(message, exitCode, cause);
    }
}
