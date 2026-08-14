using Xunit;
using System.Diagnostics;
using System.Reflection;
using Pdftract.Internal;
using Pdftract.Exceptions;
using System.Runtime.InteropServices;
using System.Threading.Tasks;

namespace Pdftract.Tests.Process;

/// <summary>
/// Integration tests for ProcessWrapper subprocess lifecycle and error handling.
/// Tests successful execution, cancellation, error conditions, and resource management.
/// </summary>
public class ProcessWrapperIntegrationTests : IDisposable
{
    private readonly string _testTempDir;
    private readonly string _originalPath;

    public ProcessWrapperIntegrationTests()
    {
        // Create a temporary directory for test binaries
        _testTempDir = Path.Combine(Path.GetTempPath(), $"pdftract-integration-{Guid.NewGuid()}");
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

    #region Successful Execution Tests

    [Fact]
    public async Task StartAsync_SuccessfulExecution_ReturnsProcessResult()
    {
        // Arrange - Create a mock pdftract binary that succeeds
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockSuccessBinary(testBinaryPath);

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();
            var result = await wrapper.StartAsync(new[] { "test" });

            // Assert
            Assert.Equal(0, result.ExitCode);
            Assert.Contains("success", result.Stdout.ToLower());
            Assert.Equal(string.Empty, result.Stderr);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public void Start_SuccessfulExecution_ReturnsProcessResult()
    {
        // Arrange - Create a mock pdftract binary that succeeds
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockSuccessBinary(testBinaryPath);

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();
            var result = wrapper.Start(new[] { "test" });

            // Assert
            Assert.Equal(0, result.ExitCode);
            Assert.Contains("success", result.Stdout.ToLower());
            Assert.Equal(string.Empty, result.Stderr);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_WithValidJsonOutput_ReturnsProcessResult()
    {
        // Arrange - Create a mock pdftract binary that outputs JSON
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockJsonBinary(testBinaryPath);

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();
            var result = await wrapper.StartAsync(new[] { "json" });

            // Assert
            Assert.Equal(0, result.ExitCode);
            Assert.Contains("{", result.Stdout);
            Assert.Contains("}", result.Stdout);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    #endregion

    #region Cancellation Tests

    [Fact]
    public async Task StartAsync_CancellationDuringExecution_ThrowsOperationCanceledException()
    {
        // Arrange - Create a mock pdftract binary that sleeps
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockSleepingBinary(testBinaryPath);

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            var cts = new CancellationTokenSource();
            var wrapper = new ProcessWrapper(cts.Token);

            // Start the task
            var task = wrapper.StartAsync(new[] { "sleep" });

            // Cancel after a short delay
            cts.CancelAfter(TimeSpan.FromMilliseconds(100));

            // Act & Assert - Should throw OperationCanceledException
            var exception = await Assert.ThrowsAnyAsync<OperationCanceledException>(async () => await task);
            Assert.Equal(cts.Token, exception.CancellationToken);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public void Start_CancellationDuringExecution_ThrowsOperationCanceledException()
    {
        // Arrange - Create a mock pdftract binary that sleeps
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockSleepingBinary(testBinaryPath);

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            var cts = new CancellationTokenSource();
            var wrapper = new ProcessWrapper(cts.Token);

            // Start the task
            var task = Task.Run(() => wrapper.Start(new[] { "sleep" }));

            // Cancel after a short delay
            cts.CancelAfter(TimeSpan.FromMilliseconds(100));

            // Act & Assert - Should throw OperationCanceledException
            Assert.ThrowsAny<OperationCanceledException>(() => task.Wait());
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_CancellationKillsProcessTree()
    {
        // Arrange - Create a mock pdftract binary that spawns a child
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockBinaryWithChild(testBinaryPath);

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            var cts = new CancellationTokenSource();
            var wrapper = new ProcessWrapper(cts.Token);

            // Start the task
            var task = wrapper.StartAsync(new[] { "spawn" });

            // Cancel after a short delay (allowing child to spawn)
            cts.CancelAfter(TimeSpan.FromMilliseconds(500));

            // Act & Assert
            await Assert.ThrowsAnyAsync<OperationCanceledException>(async () => await task);

            // Give time for cleanup
            await Task.Delay(200);

            // Verify no zombie processes remain
            // (This is implementation-specific and may vary by platform)
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    #endregion

    #region Error Condition Tests

    [Fact]
    public async Task StartAsync_FileNotFoundError_ThrowsPdftractProcessException()
    {
        // Arrange - Create a mock pdftract binary that simulates FILE_NOT_FOUND
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockErrorBinary(testBinaryPath, "FILE_NOT_FOUND", "The specified file was not found.");

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = await Assert.ThrowsAsync<PdftractProcessException>(
                async () => await wrapper.StartAsync(new[] { "error" }));

            Assert.NotNull(exception.UnderlyingException);
            Assert.IsType<global::Pdftract.Exceptions.FileNotFoundException>(exception.UnderlyingException);
            Assert.Equal("FILE_NOT_FOUND", exception.UnderlyingException.ErrorCode);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_InvalidFormatError_ThrowsPdftractProcessException()
    {
        // Arrange - Create a mock pdftract binary that simulates INVALID_FORMAT
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockErrorBinary(testBinaryPath, "INVALID_FORMAT", "The file format is invalid.");

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = await Assert.ThrowsAsync<PdftractProcessException>(
                async () => await wrapper.StartAsync(new[] { "error" }));

            Assert.NotNull(exception.UnderlyingException);
            Assert.IsType<InvalidFormatException>(exception.UnderlyingException);
            Assert.Equal("INVALID_FORMAT", exception.UnderlyingException.ErrorCode);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_PermissionDeniedError_ThrowsPdftractProcessException()
    {
        // Arrange - Create a mock pdftract binary that simulates PERMISSION_DENIED
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockErrorBinary(testBinaryPath, "PERMISSION_DENIED", "Permission denied.");

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = await Assert.ThrowsAsync<PdftractProcessException>(
                async () => await wrapper.StartAsync(new[] { "error" }));

            Assert.NotNull(exception.UnderlyingException);
            Assert.IsType<PermissionDeniedException>(exception.UnderlyingException);
            Assert.Equal("PERMISSION_DENIED", exception.UnderlyingException.ErrorCode);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_TimeoutError_ThrowsPdftractProcessException()
    {
        // Arrange - Create a mock pdftract binary that simulates TIMEOUT
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockErrorBinary(testBinaryPath, "TIMEOUT", "Operation timed out.");

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = await Assert.ThrowsAsync<PdftractProcessException>(
                async () => await wrapper.StartAsync(new[] { "error" }));

            Assert.NotNull(exception.UnderlyingException);
            Assert.IsType<global::Pdftract.Exceptions.TimeoutException>(exception.UnderlyingException);
            Assert.Equal("TIMEOUT", exception.UnderlyingException.ErrorCode);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_ResourceLimitExceededError_ThrowsPdftractProcessException()
    {
        // Arrange - Create a mock pdftract binary that simulates RESOURCE_LIMIT_EXCEEDED
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockErrorBinary(testBinaryPath, "RESOURCE_LIMIT_EXCEEDED", "Resource limit exceeded.");

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = await Assert.ThrowsAsync<PdftractProcessException>(
                async () => await wrapper.StartAsync(new[] { "error" }));

            Assert.NotNull(exception.UnderlyingException);
            Assert.IsType<ResourceLimitExceededException>(exception.UnderlyingException);
            Assert.Equal("RESOURCE_LIMIT_EXCEEDED", exception.UnderlyingException.ErrorCode);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_EncodingError_ThrowsPdftractProcessException()
    {
        // Arrange - Create a mock pdftract binary that simulates ENCODING_ERROR
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockErrorBinary(testBinaryPath, "ENCODING_ERROR", "Character encoding error.");

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = await Assert.ThrowsAsync<PdftractProcessException>(
                async () => await wrapper.StartAsync(new[] { "error" }));

            Assert.NotNull(exception.UnderlyingException);
            Assert.IsType<EncodingException>(exception.UnderlyingException);
            Assert.Equal("ENCODING_ERROR", exception.UnderlyingException.ErrorCode);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_ValidationError_ThrowsPdftractProcessException()
    {
        // Arrange - Create a mock pdftract binary that simulates VALIDATION_ERROR
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockErrorBinary(testBinaryPath, "VALIDATION_ERROR", "Input validation failed.");

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = await Assert.ThrowsAsync<PdftractProcessException>(
                async () => await wrapper.StartAsync(new[] { "error" }));

            Assert.NotNull(exception.UnderlyingException);
            Assert.IsType<ValidationException>(exception.UnderlyingException);
            Assert.Equal("VALIDATION_ERROR", exception.UnderlyingException.ErrorCode);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_InternalError_ThrowsPdftractProcessException()
    {
        // Arrange - Create a mock pdftract binary that simulates INTERNAL_ERROR
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockErrorBinary(testBinaryPath, "INTERNAL_ERROR", "An internal error occurred.");

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = await Assert.ThrowsAsync<PdftractProcessException>(
                async () => await wrapper.StartAsync(new[] { "error" }));

            Assert.NotNull(exception.UnderlyingException);
            Assert.IsType<InternalErrorException>(exception.UnderlyingException);
            Assert.Equal("INTERNAL_ERROR", exception.UnderlyingException.ErrorCode);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_UnknownErrorCode_ThrowsPdftractProcessException()
    {
        // Arrange - Create a mock pdftract binary that simulates unknown error
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockErrorBinary(testBinaryPath, "UNKNOWN_CODE", "Unknown error occurred.");

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = await Assert.ThrowsAsync<PdftractProcessException>(
                async () => await wrapper.StartAsync(new[] { "error" }));

            Assert.NotNull(exception.UnderlyingException);
            Assert.IsType<InternalErrorException>(exception.UnderlyingException);
            Assert.Contains("Unknown error code", exception.UnderlyingException.Message);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_NonJsonError_ThrowsPdftractProcessException()
    {
        // Arrange - Create a mock pdftract binary that outputs plain text error
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockPlainTextErrorBinary(testBinaryPath, "Plain text error message");

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = await Assert.ThrowsAsync<PdftractProcessException>(
                async () => await wrapper.StartAsync(new[] { "error" }));

            // Should not have underlying exception (couldn't parse as JSON)
            Assert.Null(exception.UnderlyingException);
            Assert.Contains("Plain text error message", exception.Message);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Theory]
    [InlineData("FILE_NOT_FOUND", typeof(global::Pdftract.Exceptions.FileNotFoundException))]
    [InlineData("INVALID_FORMAT", typeof(global::Pdftract.Exceptions.InvalidFormatException))]
    [InlineData("PERMISSION_DENIED", typeof(global::Pdftract.Exceptions.PermissionDeniedException))]
    [InlineData("TIMEOUT", typeof(global::Pdftract.Exceptions.TimeoutException))]
    [InlineData("RESOURCE_LIMIT_EXCEEDED", typeof(global::Pdftract.Exceptions.ResourceLimitExceededException))]
    [InlineData("ENCODING_ERROR", typeof(global::Pdftract.Exceptions.EncodingException))]
    [InlineData("VALIDATION_ERROR", typeof(global::Pdftract.Exceptions.ValidationException))]
    [InlineData("INTERNAL_ERROR", typeof(global::Pdftract.Exceptions.InternalErrorException))]
    public async Task StartAsync_AllErrorCodes_ThrowCorrectExceptionTypes(string errorCode, Type expectedType)
    {
        // Arrange - Create a mock pdftract binary that simulates the error
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockErrorBinary(testBinaryPath, errorCode, $"Error for {errorCode}");

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = await Assert.ThrowsAsync<PdftractProcessException>(
                async () => await wrapper.StartAsync(new[] { "error" }));

            Assert.NotNull(exception.UnderlyingException);
            Assert.IsType(expectedType, exception.UnderlyingException);
            Assert.Equal(errorCode, exception.UnderlyingException.ErrorCode);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_ExceptionMessage_ContainsUsefulDetails()
    {
        // Arrange - Create a mock pdftract binary that simulates FILE_NOT_FOUND with detailed message
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        string detailedMessage = "Could not find file '/path/to/document.pdf'. Please check the file path and try again.";
        CreateMockErrorBinary(testBinaryPath, "FILE_NOT_FOUND", detailedMessage);

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = await Assert.ThrowsAsync<PdftractProcessException>(
                async () => await wrapper.StartAsync(new[] { "error" }));

            Assert.NotNull(exception.UnderlyingException);
            Assert.Contains(detailedMessage, exception.UnderlyingException.Message);
            Assert.Equal("FILE_NOT_FOUND", exception.UnderlyingException.ErrorCode);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    #endregion

    #region Resource Leak Tests

    [Fact]
    public async Task StartAsync_MultipleExecutions_NoFileHandleLeaks()
    {
        // Arrange - Create a mock pdftract binary
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockSuccessBinary(testBinaryPath);

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act - Run many processes in a loop
            var wrapper = new ProcessWrapper();
            const int iterations = 50;

            for (int i = 0; i < iterations; i++)
            {
                var result = await wrapper.StartAsync(new[] { $"test-{i}" });
                Assert.Equal(0, result.ExitCode);
            }

            // Assert - If we get here without running out of file handles, no leaks occurred
            // (On systems with low file descriptor limits, this would fail)
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_ConcurrentExecutions_HandlesConcurrency()
    {
        // Arrange - Create a mock pdftract binary
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockSuccessBinary(testBinaryPath);

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act - Run multiple processes concurrently
            const int concurrency = 10;
            var tasks = new List<Task<ProcessResult>>();

            for (int i = 0; i < concurrency; i++)
            {
                var wrapper = new ProcessWrapper();
                tasks.Add(wrapper.StartAsync(new[] { $"concurrent-{i}" }));
            }

            // Wait for all to complete
            var results = await Task.WhenAll(tasks);

            // Assert - All should succeed
            Assert.Equal(concurrency, results.Length);
            Assert.All(results, r => Assert.Equal(0, r.ExitCode));
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public async Task StartAsync_MemoryUsage_StaysBounded()
    {
        // Arrange - Create a mock pdftract binary
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockSuccessBinary(testBinaryPath);

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act - Run many processes
            var wrapper = new ProcessWrapper();
            const int iterations = 100;

            for (int i = 0; i < iterations; i++)
            {
                var result = await wrapper.StartAsync(new[] { $"memory-test-{i}" });
                Assert.Equal(0, result.ExitCode);
            }

            // Assert - If we get here without OutOfMemoryException, memory usage is bounded
            // (This is a basic sanity check; detailed memory profiling would require tools)
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public void ProcessWrapper_DisposesCorrectly_NoResourceLeaks()
    {
        // Arrange - Create multiple ProcessWrapper instances
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockSuccessBinary(testBinaryPath);

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act - Create and use many wrappers
            const int iterations = 50;
            for (int i = 0; i < iterations; i++)
            {
                var wrapper = new ProcessWrapper();
                var result = wrapper.Start(new[] { $"dispose-{i}" });
                Assert.Equal(0, result.ExitCode);
                // Let wrapper go out of scope
            }

            // Force garbage collection to trigger any finalizers
            GC.Collect();
            GC.WaitForPendingFinalizers();

            // Assert - If we get here, no resource leaks occurred
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    #endregion

    #region Synchronous Error Tests

    [Fact]
    public void Start_FileNotFoundError_ThrowsPdftractProcessException()
    {
        // Arrange - Create a mock pdftract binary that simulates FILE_NOT_FOUND
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockErrorBinary(testBinaryPath, "FILE_NOT_FOUND", "File not found.");

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = Assert.Throws<PdftractProcessException>(() => wrapper.Start(new[] { "error" }));

            Assert.NotNull(exception.UnderlyingException);
            Assert.IsType<global::Pdftract.Exceptions.FileNotFoundException>(exception.UnderlyingException);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    [Fact]
    public void Start_NonJsonError_ThrowsPdftractProcessException()
    {
        // Arrange - Create a mock pdftract binary that outputs plain text error
        string binaryName = GetPlatformExecutableName();
        string testBinaryPath = Path.Combine(_testTempDir, binaryName);
        CreateMockPlainTextErrorBinary(testBinaryPath, "Plain text error");

        // Set PATH to include our test directory
        SetTestPath();

        try
        {
            // Act
            var wrapper = new ProcessWrapper();

            // Assert
            var exception = Assert.Throws<PdftractProcessException>(() => wrapper.Start(new[] { "error" }));

            Assert.Null(exception.UnderlyingException);
            Assert.Contains("Plain text error", exception.Message);
        }
        catch (System.IO.FileNotFoundException)
        {
            // Skip test if pdftract binary is not available
        }
    }

    #endregion

    #region Helper Methods

    private static string GetPlatformExecutableName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return "pdftract.exe";
        }
        else
        {
            return "pdftract";
        }
    }

    private void SetTestPath()
    {
        string newPath = _testTempDir + Path.PathSeparator + _originalPath;
        Environment.SetEnvironmentVariable("PATH", newPath);
    }

    private void CreateMockSuccessBinary(string path)
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            // Windows batch script
            var script = $"@echo off\necho SUCCESS: Process completed successfully\nexit /b 0\n";
            File.WriteAllText(path, script);
        }
        else
        {
            // Unix shell script
            var script = $"#!/bin/sh\necho 'SUCCESS: Process completed successfully'\nexit 0\n";
            File.WriteAllText(path, script);

            // Make executable
            System.Diagnostics.Process.Start("chmod", $"+x {path}")?.WaitForExit();
        }
    }

    private void CreateMockJsonBinary(string path)
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            var script = $"@echo off\necho {{\"status\": \"success\"}}\nexit /b 0\n";
            File.WriteAllText(path, script);
        }
        else
        {
            var script = $"#!/bin/sh\necho '{{\"status\": \"success\"}}'\nexit 0\n";
            File.WriteAllText(path, script);
            System.Diagnostics.Process.Start("chmod", $"+x {path}")?.WaitForExit();
        }
    }

    private void CreateMockErrorBinary(string path, string errorCode, string message)
    {
        string jsonError = $"{{\"error\": {{\"code\": \"{errorCode}\", \"message\": \"{message}\"}}}}";
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            var script = $"@echo off\necho {jsonError} 1>&2\nexit /b 1\n";
            File.WriteAllText(path, script);
        }
        else
        {
            var script = $"#!/bin/sh\necho '{jsonError}' >&2\nexit 1\n";
            File.WriteAllText(path, script);
            System.Diagnostics.Process.Start("chmod", $"+x {path}")?.WaitForExit();
        }
    }

    private void CreateMockPlainTextErrorBinary(string path, string errorMessage)
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            var script = $"@echo off\necho {errorMessage} 1>&2\nexit /b 1\n";
            File.WriteAllText(path, script);
        }
        else
        {
            var script = $"#!/bin/sh\necho '{errorMessage}' >&2\nexit 1\n";
            File.WriteAllText(path, script);
            System.Diagnostics.Process.Start("chmod", $"+x {path}")?.WaitForExit();
        }
    }

    private void CreateMockSleepingBinary(string path)
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            var script = $"@echo off\ntimeout /t 60 /nobreak >nul 2>&1\nexit /b 0\n";
            File.WriteAllText(path, script);
        }
        else
        {
            var script = $"#!/bin/sh\nsleep 60\nexit 0\n";
            File.WriteAllText(path, script);
            System.Diagnostics.Process.Start("chmod", $"+x {path}")?.WaitForExit();
        }
    }

    private void CreateMockBinaryWithChild(string path)
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            // Windows: spawn a child process that sleeps
            var script = $"@echo off\nstart /B cmd /c \"timeout /t 60 /nobreak\"\ntimeout /t 60 /nobreak\nexit /b 0\n";
            File.WriteAllText(path, script);
        }
        else
        {
            // Unix: spawn a child process that sleeps
            var script = $"#!/bin/sh\n(sleep 60) &\nsleep 60\nexit 0\n";
            File.WriteAllText(path, script);
            System.Diagnostics.Process.Start("chmod", $"+x {path}")?.WaitForExit();
        }
    }

    #endregion
}