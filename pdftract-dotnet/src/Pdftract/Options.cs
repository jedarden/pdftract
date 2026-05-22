namespace Pdftract;

/// <summary>
/// Options controlling PDF extraction behavior.
/// </summary>
public sealed class ExtractOptions
{
    /// <summary>
    /// Password for encrypted PDFs.
    /// </summary>
    public string? Password { get; init; }

    /// <summary>
    /// ISO 639-3 language code for OCR.
    /// </summary>
    public string? OcrLanguage { get; init; }

    /// <summary>
    /// Confidence threshold for OCR (0-1).
    /// </summary>
    public double? OcrThreshold { get; init; }

    /// <summary>
    /// Preserve original reading order and layout.
    /// </summary>
    public bool? PreserveLayout { get; init; }

    /// <summary>
    /// Extract embedded images.
    /// </summary>
    public bool? ExtractImages { get; init; }

    /// <summary>
    /// Format for extracted images (png, jpg, webp).
    /// </summary>
    public string? ImageFormat { get; init; }

    /// <summary>
    /// Minimum dimension for image extraction.
    /// </summary>
    public int? MinImageSize { get; init; }

    /// <summary>
    /// Maximum seconds to wait for the operation.
    /// </summary>
    public int? Timeout { get; init; }

    internal List<string> ToArgs()
    {
        var args = new List<string>();

        if (Password is not null)
        {
            args.Add("--password");
            args.Add(Password);
        }

        if (OcrLanguage is not null)
        {
            args.Add("--ocr-language");
            args.Add(OcrLanguage);
        }

        if (OcrThreshold.HasValue)
        {
            args.Add("--ocr-threshold");
            args.Add(OcrThreshold.Value.ToStringInvariant());
        }

        if (PreserveLayout == true)
        {
            args.Add("--preserve-layout");
        }

        if (ExtractImages == true)
        {
            args.Add("--extract-images");
        }

        if (ImageFormat is not null)
        {
            args.Add("--image-format");
            args.Add(ImageFormat);
        }

        if (MinImageSize.HasValue)
        {
            args.Add("--min-image-size");
            args.Add(MinImageSize.Value.ToString());
        }

        if (Timeout.HasValue)
        {
            args.Add("--timeout");
            args.Add(Timeout.Value.ToString());
        }

        return args;
    }
}

/// <summary>
/// Options controlling search behavior.
/// </summary>
public sealed class SearchOptions
{
    /// <summary>
    /// Ignore case when matching.
    /// </summary>
    public bool? CaseInsensitive { get; init; }

    /// <summary>
    /// Treat pattern as regular expression.
    /// </summary>
    public bool? Regex { get; init; }

    /// <summary>
    /// Match only whole words.
    /// </summary>
    public bool? WholeWord { get; init; }

    /// <summary>
    /// Maximum matches to return.
    /// </summary>
    public int? MaxResults { get; init; }

    internal List<string> ToArgs()
    {
        var args = new List<string>();

        if (CaseInsensitive == true)
        {
            args.Add("--case-insensitive");
        }

        if (Regex == true)
        {
            args.Add("--regex");
        }

        if (WholeWord == true)
        {
            args.Add("--whole-word");
        }

        if (MaxResults.HasValue)
        {
            args.Add("--max-results");
            args.Add(MaxResults.Value.ToString());
        }

        return args;
    }
}

/// <summary>
/// Options controlling hash computation behavior.
/// </summary>
public sealed class HashOptions
{
    /// <summary>
    /// Password for encrypted PDFs.
    /// </summary>
    public string? Password { get; init; }

    internal List<string> ToArgs()
    {
        var args = new List<string>();

        if (Password is not null)
        {
            args.Add("--password");
            args.Add(Password);
        }

        return args;
    }
}

file static class DoubleExtensions
{
    public static string ToStringInvariant(this double value) => 
        value.ToString(System.Globalization.CultureInfo.InvariantCulture);
}
