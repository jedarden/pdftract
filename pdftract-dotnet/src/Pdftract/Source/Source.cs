namespace Pdftract;

/// <summary>
/// Represents a PDF source (file path, URL, or raw bytes).
/// </summary>
public abstract class Source
{
    /// <summary>
    /// Returns command-line arguments for the source.
    /// </summary>
    internal abstract List<string> ToArgs();

    /// <summary>
    /// Performs cleanup (e.g., deletes temporary files).
    /// </summary>
    internal virtual void Dispose() { }

    /// <summary>
    /// Creates a Source from a local file path.
    /// </summary>
    public static Source FromPath(string path) => new PathSource(path);

    /// <summary>
    /// Creates a Source from a URL string.
    /// </summary>
    public static Source FromUrl(string url) => new UrlSource(url);

    /// <summary>
    /// Creates a Source from a URI.
    /// </summary>
    public static Source FromUri(Uri uri) => new UrlSource(uri.ToString());

    /// <summary>
    /// Creates a Source from a byte array.
    /// </summary>
    public static Source FromBytes(byte[] data) => new BytesSource(data);

    /// <summary>
    /// Creates a Source from a file by reading it into memory.
    /// </summary>
    public static Source FromFileBytes(string path)
    {
        var data = File.ReadAllBytes(path);
        return new BytesSource(data);
    }
}

/// <summary>
/// A local filesystem path source.
/// </summary>
public sealed class PathSource : Source
{
    private readonly string _path;

    public PathSource(string path)
    {
        _path = Path.GetFullPath(path);
    }

    internal override List<string> ToArgs()
    {
        return new() { _path };
    }
}

/// <summary>
/// A remote URL source.
/// </summary>
public sealed class UrlSource : Source
{
    private readonly string _url;

    public UrlSource(string url)
    {
        if (!url.StartsWith("http://", StringComparison.OrdinalIgnoreCase) &&
            !url.StartsWith("https://", StringComparison.OrdinalIgnoreCase))
        {
            throw new ArgumentException("URL must start with http:// or https://", nameof(url));
        }
        _url = url;
    }

    internal override List<string> ToArgs()
    {
        return new() { "--url", _url };
    }
}

/// <summary>
/// An in-memory byte array source.
/// Creates a temporary file that is cleaned up after use.
/// </summary>
public sealed class BytesSource : Source
{
    private readonly byte[] _data;
    private string? _tmpPath;

    public BytesSource(byte[] data)
    {
        _data = data ?? throw new ArgumentNullException(nameof(data));
    }

    internal override List<string> ToArgs()
    {
        if (_tmpPath != null)
        {
            return new() { _tmpPath };
        }

        var tmpFile = Path.GetTempFileName();
        File.WriteAllBytes(tmpFile, _data);
        _tmpPath = tmpFile;
        return new() { _tmpPath };
    }

    internal override void Dispose()
    {
        try
        {
            if (_tmpPath != null && File.Exists(_tmpPath))
            {
                File.Delete(_tmpPath);
            }
        }
        catch
        {
            // Ignore cleanup errors
        }
        _tmpPath = null;
    }
}
