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
    /// Type discriminator for JSON serialization. Ignored during (de)serialization:
    /// the "type" key is owned by the class-level JsonPolymorphic discriminator, so
    /// serializing this property too would emit two "type" keys in the same object.
    /// </summary>
    [JsonIgnore]
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
        /// Constructor used by JSON deserialization and by the FromPath factory method.
        /// System.Text.Json can only populate this type through an annotated constructor:
        /// the properties deliberately expose no public setters.
        /// </summary>
        /// <param name="path">The file system path to the PDF.</param>
        [JsonConstructor]
        internal FilePath(string path)
        {
            Path = path;
            Type = "FilePath";
        }

        /// <summary>
        /// Creates a FilePath source instance.
        /// </summary>
        /// <param name="path">The file system path to the PDF.</param>
        /// <returns>A FilePath source instance.</returns>
        public static FilePath FromPath(string path)
        {
            return new FilePath(path);
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
        /// Constructor used by JSON deserialization and by the FromBase64 factory method.
        /// </summary>
        /// <param name="data">The base64-encoded PDF data.</param>
        [JsonConstructor]
        internal Base64(string data)
        {
            Data = data;
            Type = "Base64";
        }

        /// <summary>
        /// Creates a Base64 source instance.
        /// </summary>
        /// <param name="data">The base64-encoded PDF data.</param>
        /// <returns>A Base64 source instance.</returns>
        public static Base64 FromBase64(string data)
        {
            return new Base64(data);
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
        /// Constructor used by JSON deserialization and by the FromUrl factory method.
        /// The parameter name matches the UrlValue property, which System.Text.Json
        /// requires for constructor binding; the wire key stays "url" via JsonPropertyName.
        /// </summary>
        /// <param name="urlValue">The URL pointing to the PDF.</param>
        [JsonConstructor]
        internal Url(string urlValue)
        {
            UrlValue = urlValue;
            Type = "Url";
        }

        /// <summary>
        /// Creates a Url source instance.
        /// </summary>
        /// <param name="url">The URL pointing to the PDF.</param>
        /// <returns>A Url source instance.</returns>
        public static Url FromUrl(string url)
        {
            return new Url(url);
        }
    }
}
