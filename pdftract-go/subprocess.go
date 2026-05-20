package pdftract

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os/exec"
	"sync"
)

// subprocessResult holds the result of a subprocess execution.
type subprocessResult struct {
	output []byte
	err    error
}

// invoke executes the pdftract binary with the given arguments and context.
// It returns the combined stdout/stderr output and any error that occurred.
func (c *Client) invoke(ctx context.Context, args []string, source Source) ([]byte, error) {
	cmd := exec.CommandContext(ctx, c.binaryPath, args...)

	// Set up cancellation to kill the process
	if ctx.Done() != nil {
		var once sync.Once
		cmd.Cancel = func() error {
			once.Do(func() {
				if cmd.Process != nil {
					cmd.Process.Kill()
				}
			})
			return nil
		}
	}

	// Ensure cleanup of BytesSource temp files
	if bs, ok := source.(*BytesSource); ok {
		defer bs.cleanup()
	}

	output, err := cmd.CombinedOutput()
	if err != nil {
		if ctx.Err() != nil {
			return nil, fmt.Errorf("pdftract cancelled: %w", ctx.Err())
		}

		// Map exit codes to specific error types
		if exitErr, ok := err.(*exec.ExitError); ok {
			exitCode := exitErr.ExitCode()
			stderr := string(output)
			return nil, mapExitCodeToError(exitCode, stderr)
		}

		return nil, fmt.Errorf("pdftract execution failed: %w", err)
	}

	return output, nil
}

// invokeJSON executes the pdftract binary and parses the output as JSON.
func (c *Client) invokeJSON(ctx context.Context, args []string, result interface{}, source Source) error {
	output, err := c.invoke(ctx, args, source)
	if err != nil {
		return err
	}

	if err := json.Unmarshal(output, result); err != nil {
		return &PdftractError{
			Kind:     ErrKindUnknown,
			Message:  fmt.Sprintf("failed to parse JSON output: %v", err),
			ExitCode: -1,
		}
	}

	return nil
}

// invokeString executes the pdftract binary and returns the output as a string.
func (c *Client) invokeString(ctx context.Context, args []string, source Source) (string, error) {
	output, err := c.invoke(ctx, args, source)
	if err != nil {
		return "", err
	}
	return string(output), nil
}

// invokeStream executes the pdftract binary and streams JSONL output to a channel.
func (c *Client) invokeStream(ctx context.Context, args []string, source Source) (<-chan json.RawMessage, <-chan error) {
	resultChan := make(chan json.RawMessage, 16)
	errChan := make(chan error, 1)

	go func() {
		defer close(resultChan)
		defer close(errChan)

		// Ensure cleanup of BytesSource temp files
		if bs, ok := source.(*BytesSource); ok {
			defer bs.cleanup()
		}

		cmd := exec.CommandContext(ctx, c.binaryPath, args...)

		// Set up cancellation to kill the process
		if ctx.Done() != nil {
			var once sync.Once
			cmd.Cancel = func() error {
				once.Do(func() {
					if cmd.Process != nil {
						cmd.Process.Kill()
					}
				})
				return nil
			}
		}

		stdout, err := cmd.StdoutPipe()
		if err != nil {
			errChan <- fmt.Errorf("failed to create stdout pipe: %w", err)
			return
		}

		stderr := &bytes.Buffer{}
		cmd.Stderr = stderr

		if err := cmd.Start(); err != nil {
			errChan <- fmt.Errorf("failed to start process: %w", err)
			return
		}

		decoder := json.NewDecoder(stdout)
		for {
			var raw json.RawMessage
			if err := decoder.Decode(&raw); err != nil {
				if err == io.EOF {
					break
				}
				if ctx.Err() != nil {
					errChan <- fmt.Errorf("pdftract cancelled: %w", ctx.Err())
					return
				}
				errChan <- fmt.Errorf("failed to decode JSON: %w", err)
				return
			}
			select {
			case resultChan <- raw:
			case <-ctx.Done():
				errChan <- fmt.Errorf("pdftract cancelled: %w", ctx.Err())
				cmd.Process.Kill()
				return
			}
		}

		if err := cmd.Wait(); err != nil {
			if ctx.Err() != nil {
				errChan <- fmt.Errorf("pdftract cancelled: %w", ctx.Err())
				return
			}

			if exitErr, ok := err.(*exec.ExitError); ok {
				exitCode := exitErr.ExitCode()
				stderrStr := stderr.String()
				errChan <- mapExitCodeToError(exitCode, stderrStr)
				return
			}

			errChan <- fmt.Errorf("pdftract execution failed: %w", err)
			return
		}
	}()

	return resultChan, errChan
}
