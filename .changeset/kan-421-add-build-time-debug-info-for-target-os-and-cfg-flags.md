---
bump: patch
---

Add platform debug logging to EventHandler (KAN-421)

- Setting `RUST_LOG=debug` or `KANBAN_DEBUG_LOG` now logs the build target (OS, arch, family, endian, pointer width) once at startup when the event loop initialises
- Setting `RUST_LOG=trace` logs every raw key event (code, kind, modifiers) before the Windows key filter runs — on Windows this captures both Press and Release events, which is the exact signal needed to diagnose key-doubling issues
