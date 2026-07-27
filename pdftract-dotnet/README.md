# ⚠️ This SDK has moved

The **pdftract-dotnet** .NET SDK has been extracted from the monorepo and is now maintained as a standalone canonical repository.

## 📍 New Location

**All development now happens at:**
- **GitHub:** https://github.com/jedarden/pdftract-dotnet
- **Forgejo:** https://git.ardenone.com/jedarden/pdftract-dotnet

## 🔄 Migration Context

This extraction follows the same pattern as previous pdftract SDK extractions:
- **pdftract-php** → standalone repo (2025)
- **pdftract-swift** → standalone repo (2025)
- **pdftract-dotnet** → standalone repo (2026)

Language SDKs reach a level of maturity where they benefit from:
- Independent release cycles
- Language-specific CI/CD pipelines
- Their own issue tracking and contribution workflows
- Ecosystem alignment (NuGet packages, .NET-specific tooling)

## 📦 What This Means

- ✅ **The standalone repo is now the canonical source**
- ✅ **Future NuGet packages will be published from the standalone repo**
- ✅ **All new features and bug fixes should be directed there**
- ⚠️ **This monorepo copy is deprecated and will not receive updates**

## 🔧 Features Yet to Be Ported

As of 2026-07-27, the following features from this monorepo version have not yet been ported to the standalone canonical repo:

- `ReceiptInfo.cs` model (receipt verification metadata)
- `MatchContext` (Before/After context in search results)
- `HashOptions` record (hash-specific configuration)
- `ToBlockingEnumerable()` helper (sync enumerable conversion)
- Split model architecture (11 separate files vs 6 consolidated)
- Dedicated `Pdftract.Sync.cs` sync layer
- `Source/` subdirectory with enhanced resource management
- Advanced packaging configuration (symbols, AOT analyzer)

**See:** [Architecture Sync Plan](https://github.com/jedarden/pdftract-dotnet/blob/main/docs/plan/plan.md) for the migration roadmap.

## 📚 Historical Reference

This directory is preserved for historical reference only. Do not open issues or pull requests here — they will not be addressed.

For the current, actively maintained implementation, visit:
**https://github.com/jedarden/pdftract-dotnet**

---

*This directory was deprecated on 2026-07-27 as part of the pdftract-dotnet canonical consolidation (see ADR-002 in the standalone repo's plan.md)*
