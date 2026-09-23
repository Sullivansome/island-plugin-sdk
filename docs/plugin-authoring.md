# Authoring Island plugins

Third parties extend Island **without forking** the host. See [plugin-system.md](./plugin-system.md) for architecture.

The plugin API, SDK, shared design crate, and first-party plugins are MIT licensed. The Island application and management CLI are proprietary. Install the official app/CLI to test against Island; plugin authors do not need the private host source. Earlier MIT grants remain in effect; see [the MIT license](../LICENSE).

## Quick start (Rust IPC + full custom view)

1. Depend on `island-plugin-sdk` / `island-plugin-api` (path or git).
2. Ship a package:

```text
com.you.myplugin/
  island-plugin.toml
  bin/myplugin          # or bare command name resolved from target/
```

3. Connect and draw frames:

```rust
let mut client = island_plugin_sdk::PluginClient::connect("com.you.myplugin")?;
client.present(50, None, false, None)?;
client.set_collapsed_meta(Some("★".into()), Some("go".into()), None)?;
// loop: build BGRA8 SurfaceFrame → client.submit_frame(frame)
```

4. Install locally:

```bash
cargo build -p your-plugin
island-cli plugin install ./path/to/package
# use Market or the CLI; a running host is updated immediately
```

Reference implementation: `plugins/spotify` (Spotify now-playing + LRCLIB lyrics, patterned after [Lyrisland](https://github.com/EurFelux/Lyrisland)).

5. List on the curated index: PR to `registry/` (or `island-plugins-index`) with hashes.

## Manifest

See `plugins/spotify/island-plugin.toml`. Required permissions for custom views:

```toml
[permissions]
grant = ["present", "surface.framebuffer"]
```

## Protocol

- Set `api_version = 2` at the top of the manifest. Older plugins must rebuild against the current SDK.
- Endpoint file: runtime dir `endpoint.json` (`tcp:127.0.0.1:port`); one owner holds an OS lock.
- Host injects `ISLAND_ENDPOINT`, `ISLAND_PLUGIN_ID`, and a per-launch `ISLAND_PLUGIN_TOKEN`. The SDK includes the token in HelloOk. Merely discovering the endpoint does not authorize a connection.
- Empty grants mean no capabilities. Sticky presentation additionally requires `present.sticky`; URL launch requires `shell:open-url`. Configure contains the actual manifest grants.
- Host sends surface size/scale, interpolated expansion, focus, pointer move/down/up, scroll, and key down/up. Coordinates are logical content points. Keys use lowercase names (`space`, `left`, `right`, `escape`). Escape/right-click collapse locally.
- `SetCursor` accepts `default`, `pointer`, `text`, `crosshair`, `not-allowed`.
- Control messages are limited to 64 KiB and 240/s; binary and small JSON frames share a 60/s budget. Prefer binary frames. Frame IDs must change whenever pixels change.
- Drain SDK input regularly; more than 64 queued host events closes the connection rather than growing memory or silently losing ordered button events.
- Frames: binary BGRA8 via SDK `submit_frame`

## CLI

```bash
island-cli plugin list
island-cli plugin install ./plugins/spotify
island-cli plugin search spotify
island-cli plugin info com.sullivansome.spotify
island-cli plugin disable com.sullivansome.spotify
island-cli plugin enable com.sullivansome.spotify
island-cli plugin remove com.sullivansome.spotify
```

## Current constraints

Plugin IDs must be dotted ASCII identifiers (letters, digits, `_`, `-`), with no empty segments or reserved `builtin.` prefix. Entry paths must be relative to the package, without parent traversal. Local installation rejects symlinks, stages replacement before swapping it in, and preserves existing `data/`. Enablement lives under the user plugin directory in `.state/<id>/enabled.json`, including for bundled plugins. Legacy `.disabled` markers migrate without changing the bundle. A process-lifetime package lock serializes launch, replacement, and removal. The shared manager stops/restarts a running package during an approved upgrade; CLI enable/disable takes effect immediately while the host runs.

`ISLAND_PLUGINS_DIR` and `ISLAND_RUNTIME_DIR` override package and runtime directories for isolated development/tests. Local directories are trusted local install paths. Production indexes cannot install `file:` entries. Remote IPC packages must match this OS/architecture, contain exactly one manifest/package, match registry identity/version, and pass SHA-256 verification. Tar extraction rejects links, special files and traversal and enforces size/count limits. The CLI prints requested capabilities and requires `--accept-permissions` before installation. Remote indexes cannot reference local `file:` packages. Wasm execution remains a separate planned feature.

```bash
island-cli plugin install com.example.plugin --from-index --accept-permissions
```

Host operations enforce grants, but native IPC executables still run with the user’s OS permissions. Manifest grants do not sandbox direct filesystem, network, or subprocess access. Install reviewed native packages. Host implementation and platform validation are maintained separately from this SDK.

## Release packaging and catalog

A plugin can also publish its own website. Follow [Plugin Website v1](plugin-website.md) for discovery, metadata validation and native Market rendering; the [Spotify website](https://github.com/Sullivansome/spotify-lyrics/blob/main/website/README.md) provides a working Astro/Cloudflare Pages example.

See [Plugin management and Island Market](plugin-market.md) for deterministic `plugin pack` archives, reviewed permissions, versioned management IPC, update approval, and release catalog generation. The registry snapshot deliberately contains no development download URLs.
