# pdftract-5rl5o: cbindgen header generation for pdftract.h

## Work Completed

### Files Created

1. **crates/pdftract-libpdftract/cbindgen.toml**
   - Configures cbindgen for C header generation
   - Language: C
   - Include guard: `PDFTRACT_H`
   - `pragma_once` enabled for modern compilers
   - `cpp_compat = true` for C++ compatibility
   - Export prefix: `pdftract_`

2. **crates/pdftract-libpdftract/build.rs**
   - Runs cbindgen at build time
   - Reads cbindgen.toml config
   - Generates `include/pdftract.h` from Rust extern "C" surface

3. **crates/pdftract-libpdftract/include/pdftract.h**
   - Auto-generated header file
   - Contains include guard + pragma once
   - Currently minimal (no extern "C" functions defined yet in lib.rs)

## Acceptance Criteria

### PASS
- [x] `cargo build -p pdftract-libpdftract` regenerates `crates/pdftract-libpdftract/include/pdftract.h`
- [x] Generated .h compiles cleanly with `gcc -xc -c -o /dev/null include/pdftract.h`
- [x] Generated .h compiles cleanly with `g++ -xc++ -c -o /dev/null include/pdftract.h` (cpp_compat verified)
- [x] Header contains include guard + pragma once

### NOTE
- CI gate for header diff check should be added in `jedarden/declarative-config` (Argo Workflows CI)
- No extern "C" functions exist yet in lib.rs (scaffold only), so header is minimal
- When extern "C" functions are added in sibling beads, they will automatically appear in the header

## Header Structure
```c
/* Copyright 2026 Jed Cabanino. MIT OR Apache-2.0 */

#ifndef PDFTRACT_H
#define PDFTRACT_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#endif  /* PDFTRACT_H */
```
