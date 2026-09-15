# Goldens

Recorded reference outputs for the platform smoke corpus — one
`<fixture>.json` per `corpus/<fixture>.pdf`, produced by
`tests/smoke/record_goldens.sh` from a reference binary, plus the
`RECORDING.md` provenance note for the latest recording.

Nothing here is hand-edited. Goldens change only by re-running
`record_goldens.sh` and committing the result; see `tests/smoke/README.md`
for the lifecycle and the current bootstrap status.

`diff_goldens.sh` fails with exit code 3 while this directory holds no
`*.json` files — that is the "baseline not yet recorded" state, not a smoke
pass.
