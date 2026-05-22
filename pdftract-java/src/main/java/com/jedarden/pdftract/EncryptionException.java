package com.jedarden.pdftract;

/**
 * The PDF is encrypted and password is missing or wrong.
 */
public class EncryptionException extends PdftractException {
    public EncryptionException(String message, int exitCode) {
        super(message, exitCode);
    }

    public EncryptionException(String message, int exitCode, String stderr) {
        super(message, exitCode, stderr);
    }

    public EncryptionException(String message, int exitCode, Throwable cause) {
        super(message, exitCode, cause);
    }
}
