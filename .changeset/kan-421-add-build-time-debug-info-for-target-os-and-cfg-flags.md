---
bump: patch
---

Add raw key event trace logging to EventHandler (KAN-421)

- Setting `RUST_LOG=trace` logs every raw key event (code, kind, modifiers) before the Windows key filter runs — on Windows this captures both Press and Release events, which is the exact signal needed to diagnose key-doubling issues
