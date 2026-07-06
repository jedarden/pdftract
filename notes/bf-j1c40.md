# bf-j1c40: Assess GitHub/Forgejo commit divergence

## Baseline State Documented

**Date:** 2026-07-06
**Tip comparison:**
- Forgejo (origin/main): `40bd7e1d382d1fa024ccf429d5fd90493d0dffba`
- GitHub (github/main): `88b4f0da276c7257ade02d3cecfaeb09f7881acc`

**Divergence summary:**
- Commits on Forgejo/main missing from GitHub/main: **330**
- Commits on GitHub/main missing from Forgejo/main: **0**

**Conclusion:** GitHub/main is a strict subset of Forgejo/main. GitHub is 330 commits behind.

**Common ancestor:**
- SHA: `88b4f0da276c7257ade02d3cecfaeb09f7881acc`
- Date: 2026-06-01 09:39:29 -0400
- Message: `fix(pdftract-2rc4): fix CI schema gate script and add verification note`

**Divergence window:** GitHub/main stopped receiving commits on **2026-06-01**. Forgejo/main has continued with 330 commits over approximately 35 days (June 1 → July 6).

**Sample of missing commits on GitHub (most recent 20):**
```
40bd7e1d docs(bf-5ee1l): document existing SSRF_BLOCKED tests
b6cdc6b5 test(bf-2rwx6): verify SSRF_BLOCKED assertion applied to all SSRF URL tests
6541c4bc docs(bf-g6aqi): document assert_exit_code method location and implementation state
93ba9a49 docs(bf-6d973-child-1): verify SSRF_BLOCKED assertion helper implementation
5231563b test(bf-1739m): add verification note for PdfExtractor instance creation
b78dfcb5 docs(bf-452rg): implement SSRF_BLOCKED substring check logic
54d77ba1 test(bf-3ayf6): fix glyph name format assertion in CMAP test
863a5e02 test(bf-5f42t): add CLI output capture verification note
a014a697 docs(bf-3n62c): document CLI execution attempt on degraded fixture
dac90550 docs(bf-3n62c): document CLI execution on degraded fixture
3973b5cc test(bf-5d2id): add unmapped glyph absence assertion to CMAP test
6070c018 docs(bf-2gfd1): document pdftract CLI text extraction flags
cccbbc24 feat(bf-pxdn0): add SSRF_BLOCKED helper function signature
02325ea6 test(bf-1zxrz): verify degraded fixture exists and is readable
5f356195 feat(bf-okdnk): add CMAP output parsing and inspection
15768b96 docs(bf-35gpt): create GLYPH_UNMAPPED message format specification
30822aff docs(bf-5n4dp): document GLYPH_UNMAPPED diagnostic message patterns
5c21bdc6 docs(bf-5g04q): document diagnostic file format and structure analysis
475e738b docs(bf-5g04q): document diagnostic file format and structure analysis
010a1fc6 docs(bf-3e0vl): document diagnostic output files in build directories
```

**Oldest missing commits (sample from divergence point):**
```
e8992816 docs(pdftract-25k4x): verify figure and caption detection implementation
4ef78174 feat(pdftract-5lvpu): add Swift SDK publish Argo workflow
dd2cb0b8 feat(pdftract-5lvpu): implement Swift SDK subprocess templates
246befd8 feat(pdftract-2m3gl): implement PHP SDK with Packagist publishing
b0b73c3c docs(pdftract-45vo7): document Ruby SDK completion status
```

## Next Steps

This baseline is recorded before attempting sync operations on child bead bf-10182. The sync will need to push all 330 commits from Forgejo to GitHub.
