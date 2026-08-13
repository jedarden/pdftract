using System;

namespace Pdftract.Exceptions
{
    /// <summary>
    /// Exception thrown when an internal error occurs.
    /// </summary>
    public class InternalErrorException : PdftractException
    {
        private const string ErrorCodeValue = "INTERNAL_ERROR";

        /// <summary>
        /// Initializes a new instance of the <see cref="InternalErrorException"/> class.
        /// </summary>
        /// <param name="message">The error message that describes the error.</param>
        /// <param name="errorDetails">Detailed information about the error.</param>
        /// <param name="exitCode">The exit code associated with this error, if applicable.</param>
        public InternalErrorException(
            string message,
            string errorDetails = null,
            int? exitCode = null)
            : base(message, ErrorCodeValue, errorDetails, exitCode)
        {
        }

        /// <summary>
        /// Initializes a new instance of the <see cref="InternalErrorException"/> class
        /// with a reference to the inner exception that is the cause of this exception.
        /// </summary>
        /// <param name="message">The error message that describes the error.</param>
        /// <param name="errorDetails">Detailed information about the error.</param>
        /// <param name="exitCode">The exit code associated with this error, if applicable.</param>
        /// <param name="innerException">The exception that is the cause of the current exception.</param>
        public InternalErrorException(
            string message,
            string errorDetails,
            int? exitCode,
            Exception innerException)
            : base(message, ErrorCodeValue, errorDetails, exitCode, innerException)
        {
        }
    }
}
