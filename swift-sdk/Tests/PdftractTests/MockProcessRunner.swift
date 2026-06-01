//
//  MockProcessRunner.swift
//  PdftractTests
//
//  Mock ProcessRunner for testing without actual subprocess execution.
//

import Foundation
#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

/// Mock process runner for testing PDF extraction without real subprocesses.
///
/// This mock simulates pdftract binary responses with predefined JSON/text outputs,
/// enabling deterministic unit tests without external dependencies.
public actor MockProcessRunner {
    /// Predefined responses for specific command patterns.
    private var responses: [String: Response] = [:]

    /// Track which commands were executed.
    private var executionLog: [ExecutionRecord] = []

    /// Whether to simulate errors.
    private var shouldSimulateError = false
    private var simulatedError: PdftractError?

    /// Response data structure.
    public struct Response {
        let stdout: Data
        let exitCode: Int32
        let delay: UInt64  // nanoseconds to simulate processing time

        public init(stdout: Data, exitCode: Int32 = 0, delay: UInt64 = 0) {
            self.stdout = stdout
            self.exitCode = exitCode
            self.delay = delay
        }
    }

    /// Execution record for verification.
    public struct ExecutionRecord {
        let executable: String
        let arguments: [String]
        let timestamp: Date

        public init(executable: String, arguments: [String], timestamp: Date = Date()) {
            self.executable = executable
            self.arguments = arguments
            self.timestamp = timestamp
        }

        /// Check if this execution matches a command pattern.
        func matches(_ command: String) -> Bool {
            arguments.contains(command)
        }

        /// Get command arguments as a key.
        var commandKey: String {
            arguments.joined(separator: " ")
        }
    }

    /// Create a new mock process runner.
    public init() {}

    /// Set a predefined response for a command pattern.
    ///
    /// - Parameters:
    ///   - pattern: Command pattern to match (e.g., "extract" or "metadata").
    ///   - response: The response to return.
    public func setResponse(_ pattern: String, _ response: Response) {
        responses[pattern] = response
    }

    /// Set a response from a JSON string.
    ///
    /// - Parameters:
    ///   - pattern: Command pattern to match.
    ///   - jsonString: Valid JSON string to return as stdout.
    public func setJSONResponse(_ pattern: String, _ jsonString: String) {
        guard let data = jsonString.data(using: .utf8) else {
            fatalError("Invalid JSON string encoding")
        }
        responses[pattern] = Response(stdout: data)
    }

    /// Set a text response.
    ///
    /// - Parameters:
    ///   - pattern: Command pattern to match.
    ///   - text: Text to return as stdout.
    public func setTextResponse(_ pattern: String, _ text: String) {
        guard let data = text.data(using: .utf8) else {
            fatalError("Invalid text encoding")
        }
        responses[pattern] = Response(stdout: data)
    }

    /// Set error simulation.
    ///
    /// - Parameters:
    ///   - error: The error to throw when execution is attempted.
    public func setSimulatedError(_ error: PdftractError) {
        self.shouldSimulateError = true
        self.simulatedError = error
    }

    /// Clear all predefined responses and logs.
    public func reset() {
        responses.removeAll()
        executionLog.removeAll()
        shouldSimulateError = false
        simulatedError = nil
    }

    /// Execute with mock data.
    public func execute(
        executable: String,
        arguments: [String],
        environment: [String: String]? = nil
    ) async throws -> Data {
        // Log execution
        let record = ExecutionRecord(executable: executable, arguments: arguments)
        executionLog.append(record)

        // Check for simulated error
        if shouldSimulateError {
            throw simulatedError ?? PdftractError.internalError("Simulated error")
        }

        // Find matching response
        let commandKey = arguments.joined(separator: " ")
        for (pattern, response) in responses {
            if commandKey.contains(pattern) || arguments.contains(pattern) {
                // Simulate processing delay
                if response.delay > 0 {
                    try await Task.sleep(nanoseconds: response.delay)
                }

                // Check exit code
                if response.exitCode != 0 {
                    throw PdftractError.internalError(
                        "Process exited with code \(response.exitCode)"
                    )
                }

                return response.stdout
            }
        }

        // No matching response - return default minimal JSON
        let defaultJSON = """
        {
            "schema_version": "1.0",
            "metadata": {
                "page_count": 1
            },
            "pages": [
                {
                    "page_index": 0,
                    "width": 612,
                    "height": 792,
                    "rotation": 0,
                    "spans": [],
                    "blocks": []
                }
            ],
            "errors": []
        }
        """

        guard let data = defaultJSON.data(using: .utf8) else {
            throw PdftractError.internalError("Failed to encode default JSON")
        }

        return data
    }

    /// Execute streaming with mock data.
    public func executeStreaming(
        executable: String,
        arguments: [String],
        environment: [String: String]? = nil
    ) -> AsyncThrowingStream<Data, Error> {
        return AsyncThrowingStream { continuation in
            Task {
                // Log execution
                let record = ExecutionRecord(executable: executable, arguments: arguments)
                executionLog.append(record)

                // Find matching response
                let commandKey = arguments.joined(separator: " ")
                var foundResponse = false

                for (pattern, response) in responses {
                    if commandKey.contains(pattern) || arguments.contains(pattern) {
                        foundResponse = true

                        // Simulate streaming by chunking the response
                        let chunkSize = 100  // Small chunks for streaming simulation
                        let data = response.stdout

                        for i in stride(from: 0, to: data.count, by: chunkSize) {
                            let end = min(i + chunkSize, data.count)
                            let chunk = data[i..<end]

                            if response.delay > 0 {
                                try? await Task.sleep(nanoseconds: response.delay / 5)
                            }

                            continuation.yield(Data(chunk))
                        }

                        // Check exit code
                        if response.exitCode != 0 {
                            continuation.finish(throwing: PdftractError.internalError(
                                "Process exited with code \(response.exitCode)"
                            ))
                        } else {
                            continuation.finish()
                        }

                        break
                    }
                }

                if !foundResponse {
                    // Return default minimal document as stream
                    let defaultJSON = """
                    {
                        "schema_version": "1.0",
                        "metadata": {"page_count": 1},
                        "pages": [{
                            "page_index": 0,
                            "width": 612,
                            "height": 792,
                            "rotation": 0,
                            "spans": [],
                            "blocks": []
                        }],
                        "errors": []
                    }
                    """

                    if let data = defaultJSON.data(using: .utf8) {
                        continuation.yield(data)
                    }

                    continuation.finish()
                }
            }
        }
    }

    /// Cancel any ongoing operation (no-op for mock).
    public func cancel() {
        // Mock doesn't have real processes to cancel
    }

    /// Get execution log for verification.
    public func getExecutionLog() -> [ExecutionRecord] {
        executionLog
    }

    /// Verify a specific command was executed.
    ///
    /// - Parameter pattern: Command pattern to look for.
    /// - Returns: True if the pattern was found in execution log.
    public func wasExecuted(_ pattern: String) -> Bool {
        executionLog.contains { record in
            record.arguments.contains(pattern) || record.commandKey.contains(pattern)
        }
    }

    /// Get execution count for a pattern.
    ///
    /// - Parameter pattern: Command pattern to count.
    /// - Returns: Number of times the pattern was executed.
    public func executionCount(_ pattern: String) -> Int {
        executionLog.filter { record in
            record.arguments.contains(pattern) || record.commandKey.contains(pattern)
        }.count
    }
}

/// Default mock responses for common operations.
extension MockProcessRunner {
    /// Set up default responses for standard operations.
    public func setupDefaultResponses() {
        // Extract response
        setJSONResponse("extract", """
        {
            "schema_version": "1.0",
            "metadata": {
                "title": "Test Document",
                "author": "Test Author",
                "page_count": 2,
                "pdf_version": "1.7"
            },
            "pages": [
                {
                    "page_index": 0,
                    "width": 612,
                    "height": 792,
                    "rotation": 0,
                    "spans": [
                        {
                            "text": "Hello World",
                            "font": "Helvetica",
                            "size": 12,
                            "bbox": [100, 700, 200, 712]
                        }
                    ],
                    "blocks": [
                        {
                            "kind": "text",
                            "bbox": [100, 700, 200, 712],
                            "spans": [0]
                        }
                    ]
                },
                {
                    "page_index": 1,
                    "width": 612,
                    "height": 792,
                    "rotation": 0,
                    "spans": [],
                    "blocks": []
                }
            ],
            "errors": []
        }
        """)

        // Text extraction response
        setTextResponse("text", "Hello World\n\nThis is test content.")

        // Markdown extraction response
        setTextResponse("markdown", "# Hello World\n\nThis is test content.")

        // Hash response
        setTextResponse("hash", """
        MD5: d41d8cd98f00b204e9800998ecf8427e
        SHA256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        """)

        // Metadata response
        setJSONResponse("metadata", """
        {
            "metadata": {
                "title": "Test Document",
                "author": "Test Author",
                "subject": "Testing",
                "page_count": 2,
                "pdf_version": "1.7",
                "is_tagged": false,
                "is_encrypted": false
            }
        }
        """)
    }

    /// Load responses from fixture files.
    ///
    /// - Parameter fixturesPath: Path to fixtures directory.
    public func loadFixtures(from fixturesPath: String) {
        let fileManager = FileManager.default

        guard fileManager.fileExists(atPath: fixturesPath) else {
            print("Warning: Fixtures path not found: \(fixturesPath)")
            return
        }

        // Load fixture files if they exist
        let fixtures = [
            ("scientific_paper.json", "extract"),
            ("text_output.txt", "text"),
            ("markdown_output.md", "markdown"),
            ("metadata.json", "metadata")
        ]

        for (filename, pattern) in fixtures {
            let filePath = (fixturesPath as NSString).appendingPathComponent(filename)
            if fileManager.fileExists(atPath: filePath),
               let data = fileManager.contents(atPath: filePath) {
                responses[pattern] = Response(stdout: data)
            }
        }
    }
}
