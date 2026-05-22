package com.jedarden.pdftract.codegen;

import java.util.ArrayList;
import java.util.List;

/**
 * Options for search operations.
 */
public class SearchOptions extends BaseOptions {
    private Boolean caseInsensitive;
    private Boolean regex;
    private Boolean wholeWord;
    private Integer maxResults;

    public SearchOptions caseInsensitive(Boolean insensitive) {
        this.caseInsensitive = insensitive;
        return this;
    }

    public SearchOptions regex(Boolean regex) {
        this.regex = regex;
        return this;
    }

    public SearchOptions wholeWord(Boolean wholeWord) {
        this.wholeWord = wholeWord;
        return this;
    }

    public SearchOptions maxResults(Integer maxResults) {
        this.maxResults = maxResults;
        return this;
    }

    // JavaBean-style setters for compatibility
    public void setCaseInsensitive(Boolean insensitive) {
        this.caseInsensitive = insensitive;
    }

    public void setRegex(Boolean regex) {
        this.regex = regex;
    }

    public void setWholeWord(Boolean wholeWord) {
        this.wholeWord = wholeWord;
    }

    public void setMaxResults(Integer maxResults) {
        this.maxResults = maxResults;
    }

    public Boolean caseInsensitive() {
        return caseInsensitive;
    }

    public Boolean regex() {
        return regex;
    }

    public Boolean wholeWord() {
        return wholeWord;
    }

    public Integer maxResults() {
        return maxResults;
    }

    @Override
    public List<String> toArgs() {
        List<String> args = super.toArgs();
        if (caseInsensitive != null && caseInsensitive) {
            args.add("--case-insensitive");
        }
        if (regex != null && regex) {
            args.add("--regex");
        }
        if (wholeWord != null && wholeWord) {
            args.add("--whole-word");
        }
        if (maxResults != null) {
            args.add("--max-results");
            args.add(maxResults.toString());
        }
        return args;
    }
}
