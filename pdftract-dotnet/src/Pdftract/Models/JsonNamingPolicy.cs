using System.Text.Json;

namespace Pdftract.Models;

/// <summary>
/// Converts PascalCase property names to snake_case for JSON serialization.
/// This ensures compatibility with the Rust pdftract binary's JSON output format.
/// </summary>
public sealed class SnakeCaseNamingPolicy : JsonNamingPolicy
{
    /// <summary>
    /// Singleton instance for reuse.
    /// </summary>
    public static readonly SnakeCaseNamingPolicy Instance = new();

    /// <summary>
    /// Converts a PascalCase property name to snake_case.
    /// </summary>
    /// <param name="name">The PascalCase property name.</param>
    /// <returns>The converted snake_case name.</returns>
    public override string ConvertName(string name)
    {
        if (string.IsNullOrEmpty(name))
        {
            return name;
        }

        // Handle abbreviations and acronyms
        var result = new System.Text.StringBuilder();
        for (int i = 0; i < name.Length; i++)
        {
            char currentChar = name[i];

            // Insert underscore before uppercase letters that follow lowercase letters
            if (char.IsUpper(currentChar) && i > 0 && char.IsLower(name[i - 1]))
            {
                result.Append('_');
            }

            // Insert underscore between consecutive uppercase letters when followed by lowercase
            if (i > 0 && char.IsUpper(currentChar) && i < name.Length - 1
                && char.IsLower(name[i + 1]) && char.IsUpper(name[i - 1]))
            {
                result.Append('_');
            }

            result.Append(char.ToLowerInvariant(currentChar));
        }

        return result.ToString();
    }
}
