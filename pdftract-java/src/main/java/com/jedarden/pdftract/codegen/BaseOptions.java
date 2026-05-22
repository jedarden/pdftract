package com.jedarden.pdftract.codegen;

import java.util.ArrayList;
import java.util.List;

/**
 * Base options for all pdftract operations.
 */
public class BaseOptions {
    private Integer timeout;
    private String password;

    /**
     * Set the timeout in seconds.
     */
    public <T extends BaseOptions> T timeout(Integer timeout) {
        this.timeout = timeout;
        @SuppressWarnings("unchecked")
        T self = (T) this;
        return self;
    }

    /**
     * Set the password for encrypted PDFs.
     */
    public <T extends BaseOptions> T password(String password) {
        this.password = password;
        @SuppressWarnings("unchecked")
        T self = (T) this;
        return self;
    }

    // JavaBean-style setters for compatibility
    public void setTimeout(Integer timeout) {
        this.timeout = timeout;
    }

    public void setPassword(String password) {
        this.password = password;
    }

    public Integer timeout() {
        return timeout;
    }

    public String password() {
        return password;
    }

    /**
     * Convert options to CLI arguments.
     */
    public List<String> toArgs() {
        List<String> args = new ArrayList<>();
        if (timeout != null) {
            args.add("--timeout");
            args.add(timeout.toString());
        }
        if (password != null) {
            args.add("--password");
            args.add(password);
        }
        return args;
    }
}
