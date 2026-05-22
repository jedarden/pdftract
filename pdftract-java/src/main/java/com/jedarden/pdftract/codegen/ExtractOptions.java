package com.jedarden.pdftract.codegen;

import java.util.ArrayList;
import java.util.List;

/**
 * Options for extract operations.
 */
public class ExtractOptions extends BaseOptions {
    private String ocrLanguage;
    private Double ocrThreshold;
    private Boolean preserveLayout;
    private Boolean extractImages;
    private String imageFormat;
    private Integer minImageSize;

    public ExtractOptions ocrLanguage(String language) {
        this.ocrLanguage = language;
        return this;
    }

    public ExtractOptions ocrThreshold(Double threshold) {
        this.ocrThreshold = threshold;
        return this;
    }

    public ExtractOptions preserveLayout(Boolean preserve) {
        this.preserveLayout = preserve;
        return this;
    }

    public ExtractOptions extractImages(Boolean extract) {
        this.extractImages = extract;
        return this;
    }

    public ExtractOptions imageFormat(String format) {
        this.imageFormat = format;
        return this;
    }

    public ExtractOptions minImageSize(Integer size) {
        this.minImageSize = size;
        return this;
    }

    // JavaBean-style setters for compatibility
    public void setOcrLanguage(String language) {
        this.ocrLanguage = language;
    }

    public void setOcrThreshold(Double threshold) {
        this.ocrThreshold = threshold;
    }

    public void setPreserveLayout(Boolean preserve) {
        this.preserveLayout = preserve;
    }

    public void setExtractImages(Boolean extract) {
        this.extractImages = extract;
    }

    public void setImageFormat(String format) {
        this.imageFormat = format;
    }

    public void setMinImageSize(Integer size) {
        this.minImageSize = size;
    }

    public String ocrLanguage() {
        return ocrLanguage;
    }

    public Double ocrThreshold() {
        return ocrThreshold;
    }

    public Boolean preserveLayout() {
        return preserveLayout;
    }

    public Boolean extractImages() {
        return extractImages;
    }

    public String imageFormat() {
        return imageFormat;
    }

    public Integer minImageSize() {
        return minImageSize;
    }

    @Override
    public List<String> toArgs() {
        List<String> args = super.toArgs();
        if (ocrLanguage != null) {
            args.add("--ocr-language");
            args.add(ocrLanguage);
        }
        if (ocrThreshold != null) {
            args.add("--ocr-threshold");
            args.add(ocrThreshold.toString());
        }
        if (preserveLayout != null && preserveLayout) {
            args.add("--preserve-layout");
        }
        if (extractImages != null && extractImages) {
            args.add("--extract-images");
        }
        if (imageFormat != null) {
            args.add("--image-format");
            args.add(imageFormat);
        }
        if (minImageSize != null) {
            args.add("--min-image-size");
            args.add(minImageSize.toString());
        }
        return args;
    }
}
