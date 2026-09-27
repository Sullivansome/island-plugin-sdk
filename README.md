# Island Plugin SDK

MIT-licensed IPC protocol, client SDK, and shared design tokens for Island plugins. The proprietary Island application and management CLI are not included and are not required to build or test this workspace.

| Crate | Purpose |
| --- | --- |
| `island-plugin-api` | Versioned messages, manifest validation, framing, permissions and plugin website metadata |
| `island-plugin-sdk` | Rust IPC client for a host-provided endpoint and per-launch token |
| `island-design` | Shared geometry, colors, motion and component tokens |

[Island app and purchase terms](https://island.brighteng.org/pricing/) · [More from bright](https://me.brighteng.org/)

## Build and test

Requires stable Rust with edition 2024 support (Rust 1.85 or newer).

```sh
cargo build --workspace --locked
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Add the API and SDK as Git dependencies from this repository. Pin both to the same full commit SHA in your Cargo manifest; the official plugins provide working examples. There are no private path or Git dependencies.

## Plugins and protocol

- [Agent Pulse](https://github.com/Sullivansome/agent-pulse): local Claude Code, Codex and Grok lifecycle monitoring.
- [Spotify Lyrics](https://github.com/Sullivansome/spotify-lyrics): Spotify Desktop playback and synchronized lyrics.
- [Authoring guide](docs/plugin-authoring.md), [protocol and architecture](docs/plugin-system.md), [website metadata standard](docs/plugin-website.md), and [design specification](design.md).

A compatible host must provide the documented authenticated IPC connection to run a plugin's overlay. Native plugin processes have the OS user's authority; manifest grants do not sandbox arbitrary native code.

## Curated catalog

`registry/index/v1.json` is the public reviewed catalog. It currently lists source integrations without downloadable binary releases. Website metadata cannot grant capabilities or authorize executable downloads. Published artifacts must be reviewed, versioned and SHA-256 pinned before adding release entries. Submit catalog changes here.

## License and provenance

MIT; see [LICENSE](LICENSE). This is a component-only extraction from the Island workspace on 2026-09-23, including its then-current plugin protocol and design changes. It contains no proprietary application code or private repository history. Third-party licenses and trademarks remain with their owners.
