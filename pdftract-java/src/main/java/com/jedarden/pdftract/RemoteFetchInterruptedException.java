package com.jedarden.pdftract;

/**
 * Network interrupted during remote fetch.
 */
public class RemoteFetchInterruptedException extends PdftractException {
    public RemoteFetchInterruptedException(String message, int exitCode) {
        super(message, exitCode);
    }

    public RemoteFetchInterruptedException(String message, int exitCode, String stderr) {
        super(message, exitCode, stderr);
    }

    public RemoteFetchInterruptedException(String message, int exitCode, Throwable cause) {
        super(message, exitCode, cause);
    }
}
