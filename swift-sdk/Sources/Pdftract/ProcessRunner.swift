//
//  ProcessRunner.swift
//  Pdftract
//
//  Cross-platform Process abstraction for spawning pdftract subprocess.
//  Handles macOS vs Linux differences and provides proper cancellation.
//

import Foundation

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

/// Cross-platform Process runner for spawning pdftract subprocess.
///
/// This abstraction handles differences between macOS and Linux Process implementations,
/// provides proper cancellation support, and ensures resource cleanup.
public actor ProcessRunner {
    /// The underlying process instance.
    private var process: Process?

    /// Standard output pipe.
    private var stdoutPipe: Pipe?

    /// Standard error pipe.
    private var stderrPipe: Pipe?

    /// Standard input pipe.
    private var stdinPipe: Pipe?

    /// Cancellation flag.
    private var isCancelled = false

    /// Create a new ProcessRunner.
    public init() {}

    /// Execute the pdftract binary with the given arguments.
    ///
    /// - Parameters:
    ///   - executable: Path to the pdftract binary.
    ///   - arguments: Command-line arguments to pass.
    ///   - environment: Optional environment variables.
    /// - Returns: The raw output data from stdout.
    /// - Throws: `PdftractError` if the process fails.
    public func execute(
        executable: String,
        arguments: [String],
        environment: [String: String]? = nil
    ) async throws -> Data {
        // Create process
        let process = Process()
        self.process = process

        // Setup pipes
        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        let stdinPipe = Pipe()

        self.stdoutPipe = stdoutPipe
        self.stderrPipe = stderrPipe
        self.stdinPipe = stdinPipe

        // Configure process
        process.executableURL = URL(fileURLWithPath: executable)
        process.arguments = arguments
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe
        process.standardInput = stdinPipe

        // Set environment if provided
        if let env = environment {
            #if os(macOS) || os(Linux)
            var existingEnv = ProcessInfo.processInfo.environment
            for (key, value) in env {
                existingEnv[key] = value
            }
            process.environment = existingEnv
            #endif
        }

        // Collect output
        var stdoutData = Data()
        var stderrData = Data()

        // Setup reading handlers
        let stdoutHandler = stdoutPipe.fileHandleForReading.readabilityHandler
        let stderrHandler = stderrPipe.fileHandleForReading.readabilityHandler

        // Use task cancellation
        return try withTaskCancellationHandler(
            operation: {
                // Launch process
                do {
                    process.launch()
                } catch {
                    throw PdftractError.internalError("Failed to launch process: \(error.localizedDescription)")
                }

                // Read stdout asynchronously
                let stdoutTask = Task<Data, Never> {
                    var data = Data()
                    let handle = stdoutPipe.fileHandleForReading
                    while !self.isCancelled && process.isRunning {
                        let available = handle.availableData
                        if !available.isEmpty {
                            data.append(available)
                        }
                        // Small delay to avoid tight loop
                        try? await Task.sleep(nanoseconds: 10_000_000) // 10ms
                    }
                    // Read any remaining data
                    let remaining = handle.readDataToEndOfFile()
                    data.append(remaining)
                    return data
                }

                // Read stderr asynchronously
                let stderrTask = Task<Data, Never> {
                    var data = Data()
                    let handle = stderrPipe.fileHandleForReading
                    while !self.isCancelled && process.isRunning {
                        let available = handle.availableData
                        if !available.isEmpty {
                            data.append(available)
                        }
                        try? await Task.sleep(nanoseconds: 10_000_000) // 10ms
                    }
                    // Read any remaining data
                    let remaining = handle.readDataToEndOfFile()
                    data.append(remaining)
                    return data
                }

                // Wait for process to complete
                do {
                    try await waitForProcess(process)
                } catch {
                    // Process was cancelled or failed
                    terminateProcess()
                    throw error
                }

                // Get output
                stdoutData = await stdoutTask.value
                stderrData = await stderrTask.value

                // Check exit code
                let exitCode = process.terminationStatus
                if exitCode != 0 {
                    let stderr = String(data: stderrData, encoding: .utf8) ?? "Unable to read stderr"
                    throw PdftractError.internalError(
                        "Process exited with code \(exitCode): \(stderr)"
                    )
                }

                return stdoutData
            },
            onCancel: {
                // Handle cancellation
                self.isCancelled = true
                self.terminateProcess()
            }
        )
    }

    /// Execute the pdftract binary with streaming JSON output.
    ///
    /// This method yields each complete JSON object as it's received,
    /// enabling real-time processing of large outputs.
    ///
    /// - Parameters:
    ///   - executable: Path to the pdftract binary.
    ///   - arguments: Command-line arguments to pass.
    ///   - environment: Optional environment variables.
    /// - Returns: An `AsyncThrowingStream` that yields Data objects.
    /// - Throws: `PdftractError` if the process fails to start.
    public func executeStreaming(
        executable: String,
        arguments: [String],
        environment: [String: String]? = nil
    ) -> AsyncThrowingStream<Data, Error> {
        return AsyncThrowingStream { continuation in
            Task {
                do {
                    // Create process
                    let process = Process()
                    self.process = process

                    // Setup pipes
                    let stdoutPipe = Pipe()
                    let stderrPipe = Pipe()
                    let stdinPipe = Pipe()

                    self.stdoutPipe = stdoutPipe
                    self.stderrPipe = stderrPipe
                    self.stdinPipe = stdinPipe

                    // Configure process
                    process.executableURL = URL(fileURLWithPath: executable)
                    process.arguments = arguments
                    process.standardOutput = stdoutPipe
                    process.standardError = stderrPipe
                    process.standardInput = stdinPipe

                    // Set environment if provided
                    if let env = environment {
                        #if os(macOS) || os(Linux)
                        var existingEnv = ProcessInfo.processInfo.environment
                        for (key, value) in env {
                            existingEnv[key] = value
                        }
                        process.environment = existingEnv
                        #endif
                    }

                    // Launch process
                    do {
                        process.launch()
                    } catch {
                        continuation.finish(throwing: PdftractError.internalError(
                            "Failed to launch process: \(error.localizedDescription)"
                        ))
                        return
                    }

                    // Read stdout line by line
                    let handle = stdoutPipe.fileHandleForReading
                    var buffer = Data()

                    while process.isRunning && !isCancelled {
                        let available = handle.availableData
                        if !available.isEmpty {
                            buffer.append(available)

                            // Try to extract complete JSON objects
                            while let jsonEnd = findJsonEnd(in: buffer) {
                                let jsonData = buffer.prefix(jsonEnd)
                                continuation.yield(Data(jsonData))

                                // Remove processed data
                                buffer.removeFirst(jsonEnd)

                                // Skip any newlines/whitespace
                                while !buffer.isEmpty && [UInt8](buffer)[0] <= 32 {
                                    buffer.removeFirst()
                                }
                            }
                        }

                        // Small delay to avoid tight loop
                        try? await Task.sleep(nanoseconds: 10_000_000) // 10ms
                    }

                    // Read any remaining data
                    let remaining = handle.readDataToEndOfFile()
                    buffer.append(remaining)

                    // Process final JSON object if present
                    if !buffer.isEmpty {
                        continuation.yield(Data(buffer))
                    }

                    // Check exit code
                    let exitCode = process.terminationStatus
                    if exitCode != 0 {
                        let stderrHandle = stderrPipe.fileHandleForReading
                        let stderrData = stderrHandle.readDataToEndOfFile()
                        let stderr = String(data: stderrData, encoding: .utf8) ?? "Unable to read stderr"
                        continuation.finish(throwing: PdftractError.internalError(
                            "Process exited with code \(exitCode): \(stderr)"
                        ))
                    } else {
                        continuation.finish()
                    }

                } catch {
                    continuation.finish(throwing: error)
                }
            }
        }
    }

    /// Wait for a process to complete with cancellation support.
    ///
    /// - Parameter process: The process to wait for.
    /// - Throws: `PdftractError` if cancelled or process fails.
    private func waitForProcess(_ process: Process) async throws {
        // Use a polling approach with cancellation support
        while process.isRunning && !isCancelled {
            try? await Task.sleep(nanoseconds: 50_000_000) // 50ms
        }

        if isCancelled {
            throw PdftractError.internalError("Process cancelled")
        }

        if !process.isRunning && process.terminationStatus != 0 {
            throw PdftractError.internalError(
                "Process failed with exit code \(process.terminationStatus)"
            )
        }
    }

    /// Terminate the running process forcefully.
    private func terminateProcess() {
        guard let process = process, process.isRunning else { return }

        #if os(macOS) || os(Linux)
        process.terminate()
        #endif

        // Close pipes
        stdoutPipe?.fileHandleForReading.closeFile()
        stderrPipe?.fileHandleForReading.closeFile()
        stdinPipe?.fileHandleForWriting.closeFile()

        // Wait a bit for cleanup
        Task {
            try? await Task.sleep(nanoseconds: 100_000_000) // 100ms
        }
    }

    /// Cancel the running process.
    public func cancel() {
        isCancelled = true
        terminateProcess()
    }

    /// Find the end of a complete JSON object in the buffer.
    ///
    /// - Parameter buffer: The data buffer to search.
    /// - Returns: The index of the JSON end, or nil if incomplete.
    private func findJsonEnd(in buffer: Data) -> Int? {
        guard !buffer.isEmpty else { return nil }

        let bytes = [UInt8](buffer)
        var braceCount = 0
        var inString = false
        var escapeNext = false

        for (index, byte) in bytes.enumerated() {
            let char = Character(UnicodeScalar(byte))

            if escapeNext {
                escapeNext = false
                continue
            }

            if char == "\\" && inString {
                escapeNext = true
                continue
            }

            if char == "\"" {
                inString.toggle()
                continue
            }

            if !inString {
                if char == "{" {
                    braceCount += 1
                } else if char == "}" {
                    braceCount -= 1
                    if braceCount == 0 {
                        return index + 1
                    }
                }
            }
        }

        return nil
    }

    /// Clean up resources.
    deinit {
        terminateProcess()
    }
}

/// Extension to provide running property check
extension Process {
    /// Check if the process is currently running.
    ///
    /// This works across macOS and Linux by checking if terminationStatus is available.
    var isRunning: Bool {
        #if os(macOS) || os(Linux)
        return isRunning
        #else
        return false
        #endif
    }

    /// Get the termination status (exit code).
    var terminationStatus: Int32 {
        #if os(macOS) || os(Linux)
        return terminationStatus
        #else
        return -1
        #endif
    }
}
