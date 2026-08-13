using System;

namespace Pdftract.Exceptions
{
    /// <summary>
    /// Base exception class for all Pdftract SDK exceptions.
    /// Provides consistent error information across all custom exception types.
    /// </summary>
    public abstract class PdftractException : Exception
    {
        /// <summary>
        /// Gets the error code that identifies the type of error that occurred.
        /// </summary>
        public string ErrorCode { get; }

        /// <summary>
        /// Gets detailed information about the error.
        /// </summary>
        public string ErrorDetails { get; }

        /// <summary>
        /// Gets the exit code associated with this error, if applicable.
        /// </summary>
        public int? ExitCode { get; }

        /// <summary>
        /// Initializes a new instance of the <see cref="PdftractException"/> class.
        /// </summary>
        /// <param name="message">The error message that describes the error.</param>
        /// <param name="errorCode">The error code identifying the type of error.</param>
        /// <param name="errorDetails">Detailed information about the error.</param>
        /// <param name="exitCode">The exit code associated with this error, if applicable.</param>
        protected PdftractException(
            string message,
            string errorCode,
            string errorDetails,
            int? exitCode)
            : base(message)
        {
            ErrorCode = errorCode;
            ErrorDetails = errorDetails;
            ExitCode = exitCode;
        }

        /// <summary>
        /// Initializes a new instance of the <see cref="PdftractException"/> class
        /// with a reference to the inner exception that is the cause of this exception.
        /// </summary>
        /// <param name="message">The error message that describes the error.</param>
        /// <param name="errorCode">The error code identifying the type of error.</param>
        /// <param name="errorDetails">Detailed information about the error.</param>
        /// <param name="exitCode">The exit code associated with this error, if applicable.</param>
        /// <param name="innerException">The exception that is the cause of the current exception.</param>
        protected PdftractException(
            string message,
            string errorCode,
            string errorDetails,
            int? exitCode,
            Exception innerException)
            : base(message, innerException)
        {
            ErrorCode = errorCode;
            ErrorDetails = errorDetails;
            ExitCode = exitCode;
        }
    }
}
