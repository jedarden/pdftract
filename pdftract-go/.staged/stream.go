package pdftract

import (
	"context"
	"encoding/json"
)

// PageResult represents either a Page or an error from streaming extraction.
type PageResult struct {
	Page *Page
	Err  error
}

// MatchResult represents either a Match or an error from streaming search.
type MatchResult struct {
	Match *Match
	Err   error
}

// extractStream streams page results from the pdftract binary.
func (c *Client) extractStream(ctx context.Context, source Source, opts *ExtractOptions) (<-chan PageResult, error) {
	args := []string{"extract", "--ndjson"}
	args = append(args, source.source()...)

	if opts != nil {
		args = append(args, opts.toArgs()...)
	}

	rawChan, errChan := c.invokeStream(ctx, args)
	resultChan := make(chan PageResult, 16)

	go func() {
		defer close(resultChan)

		for {
			select {
			case raw, ok := <-rawChan:
				if !ok {
					return
				}
				var page Page
				if err := json.Unmarshal(raw, &page); err != nil {
					resultChan <- PageResult{Err: err}
					continue
				}
				resultChan <- PageResult{Page: &page}
			case err := <-errChan:
				if err != nil {
					resultChan <- PageResult{Err: err}
				}
				return
			case <-ctx.Done():
				resultChan <- PageResult{Err: ctx.Err()}
				return
			}
		}
	}()

	return resultChan, nil
}

// search streams match results from the pdftract binary.
func (c *Client) search(ctx context.Context, source Source, pattern string, opts *SearchOptions) (<-chan MatchResult, error) {
	args := []string{"grep", pattern}
	args = append(args, source.source()...)

	if opts != nil {
		args = append(args, opts.toArgs()...)
	}

	rawChan, errChan := c.invokeStream(ctx, args)
	resultChan := make(chan MatchResult, 16)

	go func() {
		defer close(resultChan)

		for {
			select {
			case raw, ok := <-rawChan:
				if !ok {
					return
				}
				var match Match
				if err := json.Unmarshal(raw, &match); err != nil {
					resultChan <- MatchResult{Err: err}
					continue
				}
				resultChan <- MatchResult{Match: &match}
			case err := <-errChan:
				if err != nil {
					resultChan <- MatchResult{Err: err}
				}
				return
			case <-ctx.Done():
				resultChan <- MatchResult{Err: ctx.Err()}
				return
			}
		}
	}()

	return resultChan, nil
}
