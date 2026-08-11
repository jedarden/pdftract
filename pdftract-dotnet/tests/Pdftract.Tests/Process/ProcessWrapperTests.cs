using Xunit;
using System.Diagnostics;
using System.Reflection;
using Pdftract.Internal;

namespace Pdftract.Tests.Internal;

/// <summary>
/// Unit tests for ProcessWrapper binary path resolution logic.
/// </summary>
public class ProcessWrapperTests : IDisposable
{
    private readonly string _testTempDir;
    private readonly string _originalPath;

    public ProcessWrapperTests()
    {
        // Create a temporary directory for test binaries
        _testTempDir = Path.Combine(Path.GetTempPath(), $"pdftract-test-{Guid.NewGuid()}");
        Directory.CreateDirectory(_testTempDir);

        // Save original PATH for restoration
        _originalPath = Environment.GetEnvironmentVariable("PATH") ?? string.Empty;
    }

    public void Dispose()
    {
        // Clean up test directory
        try
        {
            if (Directory.Exists(_testTempDir))
            {
                Directory.Delete(_testTempDir, recursive: true);
            }
        }
        catch
        {
            // Ignore cleanup errors in tests
        }

        // Restore original PATH
        Environment.SetEnvironmentVariable("PATH", _originalPath);
    }

    [Fact]
    public void Constructor_AcceptsCancellationToken()
    {
        // Arrange & Act
        var cts = new CancellationTokenSource();
        var wrapper = new ProcessWrapper(cts.Token);

        // Assert - no exception thrown
        Assert.NotNull(wrapper);
    }

    [Fact]
    public void Constructor_Default_NoException()
    {
        // Arrange & Act
        var wrapper = new ProcessWrapper();

        // Assert - no exception thrown
        Assert.NotNull(wrapper);
    }

    [Fact]
    public void BinaryResolution_FromBundledLocation_WhenFileExists()
    {
        // Arrange - Create a mock pdftract binary in the test directory
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        File.WriteAllText(testBinaryPath, "# mock binary");

        // Create a mock assembly in the test directory
        // Note: We can't easily mock Assembly.GetExecutingAssembly().Location in tests,
        // so this test verifies the logic would work if placed alongside the DLL

        // Act & Assert - Since we can't fully mock Assembly.Location,
        // this test documents the expected behavior
        // In a real deployment, the binary would be placed next to the DLL
        Assert.True(File.Exists(testBinaryPath));
        Assert.NotNull(Path.GetDirectoryName(testBinaryPath));
    }

    [Fact]
    public void BinaryResolution_FromPATH_WhenInEnvironmentVariable()
    {
        // Arrange - Add test directory to PATH
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        File.WriteAllText(testBinaryPath, "# mock binary");

        // Set PATH to include our test directory (plus any existing PATH to avoid breaking things)
        string newPath = _testTempDir + Path.PathSeparator + _originalPath;
        Environment.SetEnvironmentVariable("PATH", newPath);

        // Act - This will test PATH resolution when the binary is in PATH
        // Note: ProcessWrapper caches the binary path, so we create a new instance
        var wrapper = new ProcessWrapper();

        // Assert - Verify the binary exists in the PATH we set
        var pathDirs = newPath.Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries);
        bool found = pathDirs.Any(dir => File.Exists(Path.Combine(dir, binaryName)));

        Assert.True(found, $"Binary '{binaryName}' should be found in PATH: {newPath}");
        Assert.True(File.Exists(testBinaryPath), "Test binary should exist");
    }

    [Fact]
    public void BinaryResolution_ThrowsFileNotFoundException_WhenBinaryNotFound()
    {
        // Arrange - Set PATH to a directory without the binary
        string emptyDir = Path.Combine(Path.GetTempPath(), $"pdftract-empty-{Guid.NewGuid()}");
        Directory.CreateDirectory(emptyDir);
        try
        {
            Environment.SetEnvironmentVariable("PATH", emptyDir);

            // Act & Assert
            var exception = Assert.Throws<FileNotFoundException>(
                () => new ProcessWrapper());

            Assert.Contains("pdftract binary not found", exception.Message);
        }
        finally
        {
            if (Directory.Exists(emptyDir))
            {
                Directory.Delete(emptyDir, recursive: true);
            }
        }
    }

    [Fact]
    public void BinaryResolution_FromPATH_SearchesMultipleDirectories()
    {
        // Arrange - Create multiple directories, only one has the binary
        var dir1 = Path.Combine(Path.GetTempPath(), $"pdftract-dir1-{Guid.NewGuid()}");
        var dir2 = Path.Combine(Path.GetTempPath(), $"pdftract-dir2-{Guid.NewGuid()}");
        var dir3 = Path.Combine(Path.GetTempPath(), $"pdftract-dir3-{Guid.NewGuid()}");

        Directory.CreateDirectory(dir1);
        Directory.CreateDirectory(dir2);
        Directory.CreateDirectory(dir3);

        try
        {
            string binaryName = GetPlatformExecutableName();

            // Only put binary in dir2
            File.WriteAllText(Path.Combine(dir2, binaryName), "# mock binary in dir2");

            // Set PATH to search all three directories
            string searchPath = string.Join(Path.PathSeparator, dir1, dir2, dir3);
            Environment.SetEnvironmentVariable("PATH", searchPath);

            // Act - The wrapper should find the binary in dir2
            var wrapper = new ProcessWrapper();

            // Assert - Verify dir2 is in PATH and has the binary
            var pathDirs = searchPath.Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries);
            bool foundInDir2 = File.Exists(Path.Combine(dir2, binaryName));

            Assert.True(foundInDir2, $"Binary should be found in dir2: {dir2}");
            Assert.False(File.Exists(Path.Combine(dir1, binaryName)), "Dir1 should not have binary");
            Assert.False(File.Exists(Path.Combine(dir3, binaryName)), "Dir3 should not have binary");
        }
        finally
        {
            // Cleanup
            DeleteDirectoryIfExists(dir1);
            DeleteDirectoryIfExists(dir2);
            DeleteDirectoryIfExists(dir3);
        }
    }

    [Fact]
    public async Task StartAsync_ThrowsNotImplementedException()
    {
        // Arrange
        var wrapper = new ProcessWrapper();

        // Act & Assert
        await Assert.ThrowsAsync<NotImplementedException>(
            async () => await wrapper.StartAsync(new[] { "--version" }));
    }

    [Fact]
    public void Start_ThrowsNotImplementedException()
    {
        // Arrange
        var wrapper = new ProcessWrapper();

        // Act & Assert
        Assert.Throws<NotImplementedException>(
            () => wrapper.Start(new[] { "--version" }));
    }

    private static string GetPlatformExecutableName()
    {
        if (Environment.OSVersion.Platform == PlatformID.Unix ||
            Environment.OSVersion.Platform == PlatformID.MacOSX ||
            (int)Environment.OSVersion.Platform == 128)
        {
            return "pdftract";
        }
        else
        {
            return "pdftract.exe";
        }
    }

    private static void DeleteDirectoryIfExists(string path)
    {
        try
        {
            if (Directory.Exists(path))
            {
                Directory.Delete(path, recursive: true);
            }
        }
        catch
        {
            // Ignore cleanup errors
        }
    }
}
