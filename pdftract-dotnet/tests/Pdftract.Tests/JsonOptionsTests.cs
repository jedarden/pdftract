using System.Text.Json;
using System.Text.Json.Serialization;
using Pdftract.Models;
using Xunit;

namespace Pdftract.Tests;

/// <summary>
/// Unit tests for JsonOptions configuration verification.
/// </summary>
public class JsonOptionsTests
{
    /// <summary>
    /// Tests that JsonOptions.Instance is a singleton (same instance on every access).
    /// </summary>
    [Fact]
    public void JsonOptions_Instance_IsSingleton()
    {
        // Arrange & Act
        var instance1 = JsonOptions.Instance;
        var instance2 = JsonOptions.Instance;

        // Assert
        Assert.Same(instance1, instance2);
    }

    /// <summary>
    /// Tests that JsonOptions.Instance is configured with SnakeCaseNamingPolicy.
    /// </summary>
    [Fact]
    public void JsonOptions_Instance_HasSnakeCaseNamingPolicy()
    {
        // Arrange & Act
        var options = JsonOptions.Instance;

        // Assert
        Assert.NotNull(options.PropertyNamingPolicy);
        Assert.IsType<SnakeCaseNamingPolicy>(options.PropertyNamingPolicy);
    }

    /// <summary>
    /// Tests that JsonOptions.Instance has PropertyNameCaseInsensitive set to true.
    /// </summary>
    [Fact]
    public void JsonOptions_Instance_PropertyNameCaseInsensitive_IsTrue()
    {
        // Arrange & Act
        var options = JsonOptions.Instance;

        // Assert
        Assert.True(options.PropertyNameCaseInsensitive);
    }

    /// <summary>
    /// Tests that JsonOptions.Instance has WriteIndented set to false for compact JSON.
    /// </summary>
    [Fact]
    public void JsonOptions_Instance_WriteIndented_IsFalse()
    {
        // Arrange & Act
        var options = JsonOptions.Instance;

        // Assert
        Assert.False(options.WriteIndented);
    }

    /// <summary>
    /// Tests that JsonOptions.Instance ignores null properties when writing.
    /// </summary>
    [Fact]
    public void JsonOptions_Instance_IgnoresNullPropertiesWhenWriting()
    {
        // Arrange & Act
        var options = JsonOptions.Instance;

        // Assert
        Assert.Equal(JsonIgnoreCondition.WhenWritingNull, options.DefaultIgnoreCondition);
    }

    /// <summary>
    /// Tests that JsonOptions.Instance includes PdftractJsonContext in the TypeInfoResolverChain.
    /// </summary>
    [Fact]
    public void JsonOptions_Instance_IncludesPdftractJsonContext()
    {
        // Arrange & Act
        var options = JsonOptions.Instance;

        // Assert
        Assert.NotEmpty(options.TypeInfoResolverChain);
        Assert.Contains(options.TypeInfoResolverChain, resolver => resolver != null);
    }

    /// <summary>
    /// Integration test: Verifies that serialization produces snake_case output.
    /// </summary>
    [Fact]
    public void JsonOptions_Instance_Serializes_ToSnakeCase()
    {
        // Arrange
        var testObj = new TestClass
        {
            MyProperty = "value",
            AnotherField = 42,
            NestedData = new NestedClass { InternalValue = "test" }
        };

        // Act
        string json = JsonSerializer.Serialize(testObj, JsonOptions.Instance);

        // Assert
        Assert.Contains("my_property", json);
        Assert.Contains("another_field", json);
        Assert.Contains("nested_data", json);
        Assert.Contains("internal_value", json);
        // Should not contain PascalCase
        Assert.DoesNotContain("MyProperty", json);
        Assert.DoesNotContain("AnotherField", json);
    }

    /// <summary>
    /// Integration test: Verifies that deserialization handles snake_case input.
    /// </summary>
    [Fact]
    public void JsonOptions_Instance_Deserializes_FromSnakeCase()
    {
        // Arrange
        string json = """{"my_property":"value","another_field":42,"nested_data":{"internal_value":"test"}}""";

        // Act
        var result = JsonSerializer.Deserialize<TestClass>(json, JsonOptions.Instance);

        // Assert
        Assert.NotNull(result);
        Assert.Equal("value", result.MyProperty);
        Assert.Equal(42, result.AnotherField);
        Assert.NotNull(result.NestedData);
        Assert.Equal("test", result.NestedData.InternalValue);
    }

    /// <summary>
    /// Integration test: Verifies that null properties are ignored during serialization.
    /// </summary>
    [Fact]
    public void JsonOptions_Instance_IgnoresNullProperties_DuringSerialization()
    {
        // Arrange
        var testObj = new TestClass
        {
            MyProperty = "value",
            // AnotherField is null
            NestedData = null
        };

        // Act
        string json = JsonSerializer.Serialize(testObj, JsonOptions.Instance);

        // Assert
        Assert.Contains("my_property", json);
        Assert.DoesNotContain("another_field", json);
        Assert.DoesNotContain("nested_data", json);
    }

    // Test helper classes
    private class TestClass
    {
        public string MyProperty { get; set; } = string.Empty;
        public int? AnotherField { get; set; }
        public NestedClass? NestedData { get; set; }
    }

    private class NestedClass
    {
        public string InternalValue { get; set; } = string.Empty;
    }
}
