using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Abstract base class representing a PDF source for MCP operations.
/// Uses discriminated union pattern with Type property for polymorphic JSON serialization.
/// </summary>
[JsonPolymorphic(TypeDiscriminatorPropertyName = "type")]
[JsonDerivedType(typeof(Source.FilePath), "FilePath")]
[JsonDerivedType(typeof(Source.Base64), "Base64")]
[JsonDerivedType(typeof(Source.Url), "Url")]
public abstract class Source
{
    /// <summary>
    /// Type discriminator for JSON serialization.
    /// </summary>
    [JsonPropertyName("type")]
    public string Type { get; private set; } = string.Empty;

    /// <summary>
    /// Protected parameterless constructor for JSON deserialization.
    /// </summary>
    protected Source()
    {
    }

    /// <summary>
    /// Represents a PDF source from a local file path.
    /// </summary>
    public sealed class FilePath : Source
    {
        /// <summary>
        /// The file system path to the PDF.
        /// </summary>
        [JsonPropertyName("path")]
        public string Path { get; private set; } = string.Empty;

        /// <summary>
        /// Private constructor - use FromPath factory method.
        /// </summary>
        private FilePath()
        {
            Type = "FilePath";
        }

        /// <summary>
        /// Creates a FilePath source instance.
        /// </summary>
        /// <param name="path">The file system path to the PDF.</param>
        /// <returns>A FilePath source instance.</returns>
        public static FilePath FromPath(string path)
        {
            return new FilePath { Path = path };
        }
    }

    /// <summary>
    /// Represents a PDF source from base64-encoded data.
    /// </summary>
    public sealed class Base64 : Source
    {
        /// <summary>
        /// The base64-encoded PDF data.
        /// </summary>
        [JsonPropertyName("data")]
        public string Data { get; private set; } = string.Empty;

        /// <summary>
        /// Private constructor - use FromBase64 factory method.
        /// </summary>
        private Base64()
        {
            Type = "Base64";
        }

        /// <summary>
        /// Creates a Base64 source instance.
        /// </summary>
        /// <param name="data">The base64-encoded PDF data.</param>
        /// <returns>A Base64 source instance.</returns>
        public static Base64 FromBase64(string data)
        {
            return new Base64 { Data = data };
        }
    }

    /// <summary>
    /// Represents a PDF source from a URL.
    /// </summary>
    public sealed class Url : Source
    {
        /// <summary>
        /// The URL pointing to the PDF.
        /// </summary>
        [JsonPropertyName("url")]
        public string UrlValue { get; private set; } = string.Empty;

        /// <summary>
        /// Private constructor - use FromUrl factory method.
        /// </summary>
        private Url()
        {
            Type = "Url";
        }

        /// <summary>
        /// Creates a Url source instance.
        /// </summary>
        /// <param name="url">The URL pointing to the PDF.</param>
        /// <returns>A Url source instance.</returns>
        public static Url FromUrl(string url)
        {
            return new Url { UrlValue = url };
        }
    }
}
