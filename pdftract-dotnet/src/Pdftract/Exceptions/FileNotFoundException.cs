using System;

namespace Pdftract.Exceptions
{
    /// <summary>
    /// Exception thrown when a specified file cannot be found.
    /// </summary>
    public class FileNotFoundException : PdftractException
    {
        private const string ErrorCodeValue = "FILE_NOT_FOUND";

        /// <summary>
        /// Initializes a new instance of the <see cref="FileNotFoundException"/> class.
        /// </summary>
        /// <param name="message">The error message that describes the error.</param>
        /// <param name="errorDetails">Detailed information about the error.</param>
        /// <param name="exitCode">The exit code associated with this error, if applicable.</param>
        public FileNotFoundException(
            string message,
            string errorDetails = null,
            int? exitCode = null)
            : base(message, ErrorCodeValue, errorDetails, exitCode)
        {
        }

        /// <summary>
        /// Initializes a new instance of the <see cref="FileNotFoundException"/> class
        /// with a reference to the inner exception that is the cause of this exception.
        /// </summary>
        /// <param name="message">The error message that describes the error.</param>
        /// <param name="errorDetails">Detailed information about the error.</param>
        /// <param name="exitCode">The exit code associated with this error, if applicable.</param>
        /// <param name="innerException">The exception that is the cause of the current exception.</param>
        public FileNotFoundException(
            string message,
            string errorDetails,
            int? exitCode,
            Exception innerException)
            : base(message, ErrorCodeValue, errorDetails, exitCode, innerException)
        {
        }
    }
}
