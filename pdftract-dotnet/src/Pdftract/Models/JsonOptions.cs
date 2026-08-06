using System.Text.Json;
using System.Text.Json.Serialization;

namespace Pdftract.Models;

/// <summary>
/// Provides configured JsonSerializerOptions for pdftract JSON serialization.
/// Uses snake_case naming policy to match the Rust binary's JSON output format.
/// </summary>
public static class JsonOptions
{
    /// <summary>
    /// Configured JsonSerializerOptions instance with snake_case naming policy.
    /// Includes support for Source discriminated union types and polymorphic deserialization.
    /// </summary>
    public static JsonSerializerOptions Instance { get; } = CreateOptions();

    private static JsonSerializerOptions CreateOptions()
    {
        var options = new JsonSerializerOptions
        {
            PropertyNamingPolicy = SnakeCaseNamingPolicy.Instance,
            WriteIndented = false,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
            // Allow case-insensitive property matching for robustness
            PropertyNameCaseInsensitive = true
        };

        // Add the source-generated JsonContext to the type info resolver chain
        // This includes all the types decorated in PdftractJsonContext
        options.TypeInfoResolverChain.Add(PdftractJsonContext.Default);

        return options;
    }
}
