# SDK Conformance Documentation

The canonical SDK API contract is [`../notes/sdk-contract.md`](../notes/sdk-contract.md).
It defines the nine methods, source and option types, error mapping, and
versioning rules shared by every SDK.

Implementation guidance for conformance runners and their reports lives in
[`../notes/sdk-conformance-runner.md`](../notes/sdk-conformance-runner.md).
The runner document is deliberately separate from the API contract so that a
report-format change cannot be mistaken for a change to the SDK surface.
