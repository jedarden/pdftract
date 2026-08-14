using MessagePack;
using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Represents document metadata from PDF extraction.
/// </summary>
[GenerateSerializer]
[MessagePackObject]
public record Metadata
{
    [Key(0)]
    [JsonPropertyName("title")]
    public string? Title { get; init; }

    [Key(1)]
    [JsonPropertyName("author")]
    public string? Author { get; init; }

    [Key(2)]
    [JsonPropertyName("subject")]
    public string? Subject { get; init; }

    [Key(3)]
    [JsonPropertyName("keywords")]
    public string? Keywords { get; init; }

    [Key(4)]
    [JsonPropertyName("creator")]
    public string? Creator { get; init; }

    [Key(5)]
    [JsonPropertyName("producer")]
    public string? Producer { get; init; }

    [Key(6)]
    [JsonPropertyName("creation_date")]
    public string? CreationDate { get; init; }

    [Key(7)]
    [JsonPropertyName("modification_date")]
    public string? ModificationDate { get; init; }

    [Key(8)]
    [JsonPropertyName("page_count")]
    public uint PageCount { get; init; }

    [Key(9)]
    [JsonPropertyName("pdf_version")]
    public string? PdfVersion { get; init; }

    [Key(10)]
    [JsonPropertyName("is_tagged")]
    public bool IsTagged { get; init; }

    [Key(11)]
    [JsonPropertyName("is_encrypted")]
    public bool IsEncrypted { get; init; }

    [Key(12)]
    [JsonPropertyName("is_signed")]
    public bool IsSigned { get; init; }

    [Key(13)]
    [JsonPropertyName("conformance")]
    public string? Conformance { get; init; }
}
