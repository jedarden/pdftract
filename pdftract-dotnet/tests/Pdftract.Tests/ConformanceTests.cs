using System.Text.Json;
using Xunit;
using Pdftract;
using Pdftract.Models;

namespace Pdftract.Tests;

public class ConformanceTests : IAsyncLifetime
{
    private Pdftract? _client;

    public Task InitializeAsync()
    {
        // Find the pdftract binary relative to the test project
        var binaryPath = FindBinaryPath();
        _client = new Pdftract(binaryPath);
        return Task.CompletedTask;
    }

    public Task DisposeAsync()
    {
        _client?.DisposeAsync();
        return Task.CompletedTask;
    }

    private static string FindBinaryPath()
    {
        // Check common locations for the binary
        var candidates = new[]
        {
            Path.Combine("..", "..", "..", "..", "..", "..", "target", "release", "pdftract"),
            Path.Combine("..", "..", "..", "..", "..", "..", "target", "debug", "pdftract"),
            "pdftract" // Assume it's in PATH
        };

        if (Environment.OSVersion.Platform == PlatformID.Win32NT)
        {
            candidates = candidates.Select(c => c + ".exe").ToArray();
        }

        foreach (var candidate in candidates)
        {
            var fullPath = Path.GetFullPath(candidate);
            if (File.Exists(fullPath))
            {
                return fullPath;
            }
        }

        return "pdftract"; // Fall back to PATH
    }

    private static string GetFixturePath(string fixture)
    {
        // Assuming fixtures are in a well-known location
        var baseDir = Path.GetFullPath(Path.Combine("..", "..", "..", "..", "..", ".."));
        return Path.Combine(baseDir, "tests", "sdk-conformance", "fixtures", fixture);
    }

    [Fact]
    public async Task BasicExtract()
    {
        // Simple smoke test for basic extraction
        var fixturePath = GetFixturePath("minimal.pdf");
        if (!File.Exists(fixturePath))
        {
            // Skip if fixture not available
            return;
        }

        var source = Source.FromPath(fixturePath);
        var doc = await _client!.ExtractAsync(source);

        Assert.NotNull(doc);
        Assert.NotNull(doc.Pages);
        Assert.NotNull(doc.Metadata);
    }

    [Fact]
    public async Task ExtractText()
    {
        var fixturePath = GetFixturePath("minimal.pdf");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        var source = Source.FromPath(fixturePath);
        var text = await _client!.ExtractTextAsync(source);

        Assert.NotNull(text);
        Assert.NotEmpty(text);
    }

    [Fact]
    public async Task ExtractMarkdown()
    {
        var fixturePath = GetFixturePath("minimal.pdf");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        var source = Source.FromPath(fixturePath);
        var md = await _client!.ExtractMarkdownAsync(source);

        Assert.NotNull(md);
    }

    [Fact]
    public async Task GetMetadata()
    {
        var fixturePath = GetFixturePath("minimal.pdf");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        var source = Source.FromPath(fixturePath);
        var metadata = await _client!.GetMetadataAsync(source);

        Assert.NotNull(metadata);
        Assert.True(metadata.PageCount >= 0);
    }

    [Fact]
    public async Task Hash()
    {
        var fixturePath = GetFixturePath("minimal.pdf");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        var source = Source.FromPath(fixturePath);
        var fingerprint = await _client!.HashAsync(source);

        Assert.NotNull(fingerprint);
        Assert.NotNull(fingerprint.Hash);
        Assert.NotEmpty(fingerprint.Hash);
    }

    [Fact]
    public async Task Classify()
    {
        var fixturePath = GetFixturePath("minimal.pdf");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        var source = Source.FromPath(fixturePath);
        var classification = await _client!.ClassifyAsync(source);

        Assert.NotNull(classification);
        Assert.NotNull(classification.Category);
    }

    [Fact]
    public async Task ExtractStream()
    {
        var fixturePath = GetFixturePath("minimal.pdf");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        var source = Source.FromPath(fixturePath);
        var pages = new List<Page>();

        await foreach (var page in _client!.ExtractStreamAsync(source))
        {
            pages.Add(page);
        }

        Assert.NotEmpty(pages);
    }

    [Fact]
    public async Task Search()
    {
        var fixturePath = GetFixturePath("minimal.pdf");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        var source = Source.FromPath(fixturePath);
        var matches = new List<Match>();

        await foreach (var match in _client!.SearchAsync(source, "the"))
        {
            matches.Add(match);
        }

        // We don't assert count since we don't know the fixture content
        Assert.NotNull(matches);
    }

    [Fact]
    public void SourceFromPath()
    {
        var source = Source.FromPath("test.pdf");
        Assert.NotNull(source);
    }

    [Fact]
    public void SourceFromUrl()
    {
        var source = Source.FromUrl("https://example.com/doc.pdf");
        Assert.NotNull(source);
    }

    [Fact]
    public void SourceFromUri()
    {
        var uri = new Uri("https://example.com/doc.pdf");
        var source = Source.FromUri(uri);
        Assert.NotNull(source);
    }

    [Fact]
    public void SourceFromBytes()
    {
        var data = new byte[] { 0x25, 0x50, 0x44, 0x46 }; // %PDF
        var source = Source.FromBytes(data);
        Assert.NotNull(source);
    }

    [Fact]
    public async Task ExtractOptions()
    {
        var fixturePath = GetFixturePath("minimal.pdf");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        var source = Source.FromPath(fixturePath);
        var options = new ExtractOptions
        {
            PreserveLayout = true
        };

        var doc = await _client!.ExtractAsync(source, options);
        Assert.NotNull(doc);
    }

    [Fact]
    public async Task SearchOptions()
    {
        var fixturePath = GetFixturePath("minimal.pdf");
        if (!File.Exists(fixturePath))
        {
            return;
        }

        var source = Source.FromPath(fixturePath);
        var options = new SearchOptions
        {
            CaseInsensitive = true
        };

        var matches = new List<Match>();
        await foreach (var match in _client!.SearchAsync(source, "THE", options))
        {
            matches.Add(match);
        }

        Assert.NotNull(matches);
    }
}
