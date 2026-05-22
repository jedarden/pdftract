package com.jedarden.pdftract;

/**
 * Base exception for all pdftract errors.
 */
public class PdftractException extends Exception {
    private final int exitCode;

    public PdftractException(String message, int exitCode) {
        super(message);
        this.exitCode = exitCode;
    }

    public PdftractException(String message, int exitCode, String stderr) {
        super(message + (stderr != null && !stderr.isEmpty() ? ": " + stderr : ""));
        this.exitCode = exitCode;
    }

    public PdftractException(String message, int exitCode, Throwable cause) {
        super(message, cause);
        this.exitCode = exitCode;
    }

    /**
     * Returns the subprocess exit code that caused this exception.
     */
    public int getExitCode() {
        return exitCode;
    }
}
