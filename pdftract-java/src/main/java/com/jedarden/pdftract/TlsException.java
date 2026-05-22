package com.jedarden.pdftract;

/**
 * TLS certificate validation failed.
 */
public class TlsException extends PdftractException {
    public TlsException(String message, int exitCode) {
        super(message, exitCode);
    }

    public TlsException(String message, int exitCode, String stderr) {
        super(message, exitCode, stderr);
    }

    public TlsException(String message, int exitCode, Throwable cause) {
        super(message, exitCode, cause);
    }
}
