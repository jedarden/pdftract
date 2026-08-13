using System;

namespace Pdftract.Exceptions
{
    /// <summary>
    /// Exception thrown when a resource limit is exceeded.
    /// </summary>
    public class ResourceLimitExceededException : PdftractException
    {
        private const string ErrorCodeValue = "RESOURCE_LIMIT_EXCEEDED";

        /// <summary>
        /// Initializes a new instance of the <see cref="ResourceLimitExceededException"/> class.
        /// </summary>
        /// <param name="message">The error message that describes the error.</param>
        /// <param name="errorDetails">Detailed information about the error.</param>
        /// <param name="exitCode">The exit code associated with this error, if applicable.</param>
        public ResourceLimitExceededException(
            string message,
            string errorDetails = null,
            int? exitCode = null)
            : base(message, ErrorCodeValue, errorDetails, exitCode)
        {
        }

        /// <summary>
        /// Initializes a new instance of the <see cref="ResourceLimitExceededException"/> class
        /// with a reference to the inner exception that is the cause of this exception.
        /// </summary>
        /// <param name="message">The error message that describes the error.</param>
        /// <param name="errorDetails">Detailed information about the error.</param>
        /// <param name="exitCode">The exit code associated with this error, if applicable.</param>
        /// <param name="innerException">The exception that is the cause of the current exception.</param>
        public ResourceLimitExceededException(
            string message,
            string errorDetails,
            int? exitCode,
            Exception innerException)
            : base(message, ErrorCodeValue, errorDetails, exitCode, innerException)
        {
        }
    }
}
