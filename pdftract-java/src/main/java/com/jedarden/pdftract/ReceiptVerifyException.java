package com.jedarden.pdftract;

/**
 * Receipt verification failed.
 */
public class ReceiptVerifyException extends PdftractException {
    public ReceiptVerifyException(String message, int exitCode) {
        super(message, exitCode);
    }

    public ReceiptVerifyException(String message, int exitCode, String stderr) {
        super(message, exitCode, stderr);
    }

    public ReceiptVerifyException(String message, int exitCode, Throwable cause) {
        super(message, exitCode, cause);
    }
}
