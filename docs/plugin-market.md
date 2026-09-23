# Plugin management and Island Market

Market (GPUI and GTK) and `island-cli` use one Rust management implementation in `island-plugin-host`. The versioned command/result schema lives in `island-core::management`; it is independent of the plugin surface protocol and contains no toolkit types.

## Ownership

- The catalog owns published releases, platform/protocol compatibility, artifact hashes and reviewed permissions.
- Installed packages own their version and origin. Enablement and accepted-package receipts live in per-user `.state/<id>/`.
- The runtime reports stopped, starting, connected/running or failed independently of enablement. A child that fails to connect within ten seconds is stopped and reported as failed.
- Market displays Discover and My plugins as a searchable storefront with category filters and dedicated plugin pages, preview galleries, permissions, review/approval, updates, retry and removal. Downloads and package changes run off the UI thread. The saved catalog and installed packages remain available when refresh fails.

The plugin manager stays inside the existing host crate. GUI clients do not parse registry JSON, copy executables or own process lifecycle policy.

## Local plugins and recovery

My plugins includes **Install from folder…**. Choose a package containing `island-plugin.toml` and its executable, review the package and permissions, then approve or cancel. Replacements preserve plugin data and enablement. Local installation returns to My plugins with its search and category cleared so the new package is visible.

Structurally valid packages with an unsupported plugin API remain in My plugins and CLI listings as **Needs update**, with the installed/host protocol versions explained. They can be disabled, removed with data retained, or replaced with a compatible local or catalog package. Enable and Retry cannot launch incompatible code. A compatible catalog release can repair a package even when its semantic version is unchanged.

Discovery selects a compatible bundled/workspace package ahead of an incompatible user copy, without logging warnings about the unused copy. Explicit local installation can replace that stale user copy. The manifest and executable must both come from the compatible build: changing `api_version` alone does not migrate the IPC protocol.

## CLI and local control

```sh
island-cli plugin list
island-cli plugin search lyrics
island-cli plugin search lyrics --offline
island-cli plugin info com.sullivansome.spotify
island-cli plugin install com.example.plugin@1.2.0 --accept-permissions
island-cli plugin install ./my-package --disabled
island-cli plugin update com.example.plugin --accept-permissions
island-cli plugin update --accept-permissions
island-cli plugin enable com.example.plugin
island-cli plugin disable com.example.plugin
island-cli plugin retry com.example.plugin
island-cli plugin remove com.example.plugin             # retain data
island-cli plugin remove com.example.plugin --delete-data
island --market
```

While Island runs, CLI commands use an authenticated loopback management listener. Its random credential is published atomically in the private runtime directory's `management.json`, separately from plugin launch credentials. Requests require the management protocol version and token, have bounded sizes/timeouts, and cannot be sent as plugin surface messages. Native plugins still have the OS user's authority; separate protocol credentials are not an OS sandbox.

When Island is closed, the CLI acquires the same runtime lock before using the manager offline. It never silently falls back to offline mutation while the host owns that lock. Enablement then takes effect on the next launch. App, CLI and GTK share bundled/portable stock discovery. Newer user-installed versions can supersede a bundled version; equal versions retain stock precedence. Bundled packages can be disabled, but are not deleted from app resources.

## Reviewed installation

1. Resolve the requested version, or the newest compatible stable semantic version. Prereleases require an explicit version. OS, architecture and plugin protocol must match.
2. Download a bounded archive, verify SHA-256, reject unsafe entries and validate package identity/version and exact reviewed permissions. Remote packages must include their executable.
3. Stage the complete package and return a single-use review token. The review includes the previous version, origin, digest, permissions and newly added capabilities.
4. Apply that exact staged package after approval. Do not download again. Reviews expire after 15 minutes; at most eight are retained. Reject approval if the installed version changed since review.
5. Stop the old process, replace under the package lock with existing staging/rollback protection, preserve user data, record consent, and restart only if enabled. A failed installation attempts to restart the old enabled package. Installation success and startup failure are reported separately; there is no automatic rollback on a later runtime crash.

Local CLI paths are explicitly trusted local installs and still print a review. Remote installs require `--accept-permissions`. Market always shows a review before a download is installed. An update preserves enablement; a new installation offers Install & enable or Install disabled.

Removal defaults to retaining `data/` under `.state/<id>/data`; reinstall restores it. `--delete-data` removes retained data too. Removal does not erase bundled executables.

## Catalog and publishing

`registry/index/v1.json` is the embedded bootstrap snapshot. It deliberately has no fake downloadable artifacts. A bundled integration remains usable without a remote release. The default live catalog is `registry/index/v1.json` in the public `Sullivansome/island-plugin-sdk` repository; `ISLAND_INDEX_URL` and explicit CLI `--index-url` can override it. HTTPS is required except for loopback authoring/test servers. Each source has a separate last-known-good cache under `.catalog/`.

Production indexes inline their releases. Legacy `meta_path` files are also readable. Every release includes `version`, plugin `api_version`, exact reviewed `permissions`, and artifacts with `os`, `arch`, `kind`, `url`, `sha256`. Full custom views still use the existing IPC surface protocol; Wasm remains unimplemented.

Create a real artifact after building the plugin and CLI:

```sh
scripts/package-plugin.sh plugins/spotify target/release/island-plugin-spotify \
  https://github.com/Sullivansome/island/releases/download/v0.1.0 dist
# Or, for another author/package:
island-cli plugin pack ./package --binary ./built/plugin \
  --release-url https://github.com/owner/plugin/releases/download/v1.2.0 --output dist
```

Packaging includes the manifest, executable, assets/, market/, icon.png, README and license files. It emits a deterministic tarball and a catalog fragment with a real digest and permissions taken from the validated manifest. It does not publish anything. The macOS release job packages Spotify; the publish job merges prior releases and reviewed fragments into `plugin-index.json`, retaining pinned versions. Published artifact hashes are immutable: change the plugin version before shipping different bytes for the same OS/architecture. App and plugin versions are independent.

The release pipeline must run and publish its assets before a fresh client can download those releases. No source-checkout paths or `sha256: dev` entries are used in production discovery.

## Verification

Manager process tests cover authenticated runtime discovery, immediate enable/disable of bundled packages, newer user upgrades, pinned staged bytes, single-use/cancelled consent, failed preparation, startup failure, data retention/reinstall, and exclusive offline ownership. CLI/local HTTP tests cover remote consent, digest rejection and preserved installation. Packaging tests verify archive/metadata consistency and exclusion of development files. Native platform/installer smoke tests remain separate from these contracts.

Linux CI also runs the real GTK Market through AT-SPI, covering detail navigation, both preview modes, search retention, disabling a fixture plugin, and cancelling removal:

```sh
dbus-run-session -- xvfb-run -a /usr/bin/python3 tests/market_smoke.py target/debug/island
```

On macOS, check Market while it overlaps the island host, including the header buttons. The native bridge keeps transparent panel margins click-through even after app activation changes the panel's window level.

## Detail-page previews

Plugins can publish an independent website using [Plugin Website v1](plugin-website.md). Curators validate/import its descriptor with `island-cli plugin import-website`; both shells render the reviewed description, features, named previews and website/setup/support links natively. Website data cannot change package artifacts or permissions. Spotify's standalone website is the reference implementation.

Published HTTPS screenshots are displayed inside both GPUI and GTK. A read-only shared loader limits responses to 6 MiB, image dimensions to 4096 × 4096, and decoder allocation to 64 MiB. Loading, missing media, and download/decoding failures have independent UI states and a retry action. Previews never execute plugin code or require installation.

Spotify ships compact and expanded images rendered from original sample content. Regenerate them without Spotify, IPC, or network access:

```sh
cargo run -p island-plugin-spotify -- --export-market-previews plugins/spotify/market
```

These captured previews are embedded as the offline fallback when the catalog has no published screenshots. Their captions distinguish sample content from live playback. Author-supplied screenshots take precedence.
