# Kanban Landing Page

The `web/` directory contains the static landing page for the kanban project.

## Build

```bash
nix build .#web
```

Output is placed in `result/`:

```
result/
├── index.html       # Version-stamped from Cargo.toml
├── styles.css
└── demo/
    └── demo.gif     # Copied from demo/demo.gif at build time
```

The build substitutes `@VERSION@` in `index.html` with the workspace version from `Cargo.toml`.

## Development

Edit `index.html` and `styles.css` directly. The demo GIF is sourced from `../demo/demo.gif` — run `nix develop .#demo --command bash demo/record.sh` to regenerate it.

## File Structure

```
web/
├── README.md       # This file
├── default.nix     # Nix derivation
├── index.html      # Landing page (uses @VERSION@ placeholder)
└── styles.css      # Styles
```
