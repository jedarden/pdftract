using System.Diagnostics.CodeAnalysis;
using System.Runtime.CompilerServices;
using Pdftract.Models;

namespace Pdftract;

/// <summary>
/// Synchronous (blocking) wrappers for async Pdftract methods.
/// These methods are discouraged for production use in async contexts
/// as they can lead to thread-pool starvation.
/// </summary>
public sealed partial class Pdftract
{
    /// <summary>
    /// Extracts structured data from a PDF (synchronous).
    /// </summary>
    /// <remarks>
    /// This synchronous wrapper is provided for legacy code paths.
    /// In async contexts, prefer <see cref="ExtractAsync"/> instead.
    /// </remarks>
    [SuppressMessage("Usage", "CA1849:Call async methods when in an async context", Justification = "Intentional sync wrapper")]
    public Document Extract(Source source, ExtractOptions? options = null)
    {
        return ExtractAsync(source, options, CancellationToken.None).GetAwaiter().GetResult();
    }

    /// <summary>
    /// Extracts plain text from a PDF (synchronous).
    /// </summary>
    /// <remarks>
    /// This synchronous wrapper is provided for legacy code paths.
    /// In async contexts, prefer <see cref="ExtractTextAsync"/> instead.
    /// </remarks>
    [SuppressMessage("Usage", "CA1849:Call async methods when in an async context", Justification = "Intentional sync wrapper")]
    public string ExtractText(Source source, ExtractOptions? options = null)
    {
        return ExtractTextAsync(source, options, CancellationToken.None).GetAwaiter().GetResult();
    }

    /// <summary>
    /// Extracts markdown-formatted text from a PDF (synchronous).
    /// </summary>
    /// <remarks>
    /// This synchronous wrapper is provided for legacy code paths.
    /// In async contexts, prefer <see cref="ExtractMarkdownAsync"/> instead.
    /// </remarks>
    [SuppressMessage("Usage", "CA1849:Call async methods when in an async context", Justification = "Intentional sync wrapper")]
    public string ExtractMarkdown(Source source, ExtractOptions? options = null)
    {
        return ExtractMarkdownAsync(source, options, CancellationToken.None).GetAwaiter().GetResult();
    }

    /// <summary>
    /// Extracts pages from a PDF as a stream (synchronous).
    /// </summary>
    /// <remarks>
    /// This synchronous wrapper is provided for legacy code paths.
    /// In async contexts, prefer <see cref="ExtractStreamAsync"/> instead.
    /// </remarks>
    [SuppressMessage("Usage", "CA1849:Call async methods when in an async context", Justification = "Intentional sync wrapper")]
    public IEnumerable<Page> ExtractStream(Source source, ExtractOptions? options = null)
    {
        return ExtractStreamAsync(source, options, CancellationToken.None)
            .ToBlockingEnumerable();
    }

    /// <summary>
    /// Searches for a pattern in a PDF (synchronous).
    /// </summary>
    /// <remarks>
    /// This synchronous wrapper is provided for legacy code paths.
    /// In async contexts, prefer <see cref="SearchAsync"/> instead.
    /// </remarks>
    [SuppressMessage("Usage", "CA1849:Call async methods when in an async context", Justification = "Intentional sync wrapper")]
    public IEnumerable<Match> Search(Source source, string pattern, SearchOptions? options = null)
    {
        return SearchAsync(source, pattern, options, CancellationToken.None)
            .ToBlockingEnumerable();
    }

    /// <summary>
    /// Extracts metadata from a PDF (synchronous).
    /// </summary>
    /// <remarks>
    /// This synchronous wrapper is provided for legacy code paths.
    /// In async contexts, prefer <see cref="GetMetadataAsync"/> instead.
    /// </remarks>
    [SuppressMessage("Usage", "CA1849:Call async methods when in an async context", Justification = "Intentional sync wrapper")]
    public Metadata GetMetadata(Source source, ExtractOptions? options = null)
    {
        return GetMetadataAsync(source, options, CancellationToken.None).GetAwaiter().GetResult();
    }

    /// <summary>
    /// Computes the fingerprint hash of a PDF (synchronous).
    /// </summary>
    /// <remarks>
    /// This synchronous wrapper is provided for legacy code paths.
    /// In async contexts, prefer <see cref="HashAsync"/> instead.
    /// </remarks>
    [SuppressMessage("Usage", "CA1849:Call async methods when in an async context", Justification = "Intentional sync wrapper")]
    public Fingerprint Hash(Source source, HashOptions? options = null)
    {
        return HashAsync(source, options, CancellationToken.None).GetAwaiter().GetResult();
    }

    /// <summary>
    /// Classifies a PDF document (synchronous).
    /// </summary>
    /// <remarks>
    /// This synchronous wrapper is provided for legacy code paths.
    /// In async contexts, prefer <see cref="ClassifyAsync"/> instead.
    /// </remarks>
    [SuppressMessage("Usage", "CA1849:Call async methods when in an async context", Justification = "Intentional sync wrapper")]
    public Classification Classify(Source source)
    {
        return ClassifyAsync(source, CancellationToken.None).GetAwaiter().GetResult();
    }

    /// <summary>
    /// Verifies a cryptographic receipt for a PDF (synchronous).
    /// </summary>
    /// <remarks>
    /// This synchronous wrapper is provided for legacy code paths.
    /// In async contexts, prefer <see cref="VerifyReceiptAsync"/> instead.
    /// </remarks>
    [SuppressMessage("Usage", "CA1849:Call async methods when in an async context", Justification = "Intentional sync wrapper")]
    public bool VerifyReceipt(string path, Receipt receipt)
    {
        return VerifyReceiptAsync(path, receipt, CancellationToken.None).GetAwaiter().GetResult();
    }

    /// <summary>
    /// Returns the pdftract binary version (synchronous).
    /// </summary>
    /// <remarks>
    /// This synchronous wrapper is provided for legacy code paths.
    /// In async contexts, prefer <see cref="GetVersionAsync"/> instead.
    /// </remarks>
    [SuppressMessage("Usage", "CA1849:Call async methods when in an async context", Justification = "Intentional sync wrapper")]
    public string GetVersion()
    {
        return GetVersionAsync(CancellationToken.None).GetAwaiter().GetResult();
    }
}

file static class AsyncEnumerableExtensions
{
    public static IEnumerable<T> ToBlockingEnumerable<T>(this IAsyncEnumerable<T> asyncEnumerable)
    {
        if (asyncEnumerable is null)
        {
            throw new ArgumentNullException(nameof(asyncEnumerable));
        }

        return new BlockingAsyncEnumerable<T>(asyncEnumerable);
    }

    private sealed class BlockingAsyncEnumerable<T>(IAsyncEnumerable<T> source) : IEnumerable<T>
    {
        public IEnumerator<T> GetEnumerator()
        {
            return new BlockingAsyncEnumerator<T>(source.GetAsyncEnumerator(CancellationToken.None));
        }

        System.Collections.IEnumerator System.Collections.IEnumerable.GetEnumerator()
        {
            return GetEnumerator();
        }
    }

    private sealed class BlockingAsyncEnumerator<T>(IAsyncEnumerator<T> source) : IEnumerator<T>
    {
        private T? _current;
        private bool _disposed;

        public T Current => _current!;

        object System.Collections.IEnumerator.Current => Current!;

        public bool MoveNext()
        {
            if (_disposed)
            {
                return false;
            }

            using var _ = new ManualResetEvent(false);
            bool moveNextSucceeded = false;
            Exception? exception = null;

            Task.Run(async () =>
            {
                try
                {
                    moveNextSucceeded = await source.MoveNextAsync();
                }
                catch (Exception ex)
                {
                    exception = ex;
                }
                finally
                {
                    _.Set();
                }
            }).Wait();

            if (exception is not null)
            {
                throw exception;
            }

            if (moveNextSucceeded)
            {
                _current = source.Current;
            }

            return moveNextSucceeded;
        }

        public void Reset()
        {
            throw new NotSupportedException("Reset is not supported on async enumerators");
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                source.DisposeAsync().AsTask().Wait();
                _disposed = true;
            }
        }
    }
}
