---
bump: patch
---

Web landing page installation section refactored to collapse five separate install method blocks into a single unified code block. All methods (Cargo, Homebrew, Nix, AUR, from source) are now presented together with inline comments for clarity. Nix installation simplified from multi-step instructions to a single command: `nix run nixpkgs/nixpkgs-unstable#kanban`.
