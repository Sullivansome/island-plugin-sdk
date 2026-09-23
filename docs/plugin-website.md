# Plugin Website v1

A plugin may run its own website with its own visual identity. Island Market consumes a reviewed JSON snapshot and draws native views using Island's shared design system. It does not embed author HTML, execute scripts, or adopt arbitrary website CSS.

## Publish the descriptor

Serve `/.well-known/island-plugin.json` from the website's origin with HTTP 200 and `Content-Type: application/json`. Discovery must not redirect. The document is limited to 64 KiB. Link it from the page head:

```html
<link rel="island-plugin" type="application/json" href="/.well-known/island-plugin.json">
```

The [JSON Schema](plugin-website-v1.schema.json) describes the format. The live schema is hosted at `https://island-companion.pages.dev/schemas/plugin-website-v1.schema.json`. [Spotify's descriptor](https://github.com/Sullivansome/spotify-lyrics/blob/main/website/plugin.json) is the reference implementation and the content source for its website.

| Field | Contract | Market rendering |
| --- | --- | --- |
| `schema_version` | Exactly `1`; independent of plugin API and release version | Version validation |
| `id` | Exact existing plugin ID | Must match the curated listing |
| `name`, `summary` | Up to 80 / 240 characters | Author website identity; curated listing name and summary stay authoritative |
| `description` | Up to 4,000 characters, plain text | Detail-page description |
| `links.homepage` | Required absolute HTTPS URL | Open website |
| `links.documentation`, `support`, `source` | Optional absolute HTTPS URLs | Setup guide, Get help; curated repository remains the source link |
| `features` | Up to 8; title ≤80, description ≤600 | Native feature rows |
| `previews` | Up to 6; URL, label ≤40, alt ≤180, caption ≤300 | Named native gallery with captions |

Links cannot contain credentials. Preview URLs must share the homepage origin. PNG/JPEG previews use the shared bounded decoder: 6 MiB encoded, 4096×4096 pixels, 64 MiB decoder allocation. Supply accurate captions, distinguish sample content from live playback, and explain layouts that are not current defaults. Text renders literally; HTML and Markdown are not interpreted. Non-whitespace text is required; control characters other than newline/tab are rejected. Unknown fields and unsupported schema versions are rejected.

## Review and import

The website cannot self-register or change release contracts. A maintainer first adds a reviewed identity to the curated catalog, then imports its website:

```sh
cargo run -p island-cli -- plugin import-website \
  https://island-spotify.pages.dev/.well-known/island-plugin.json \
  --index registry/index/v1.json --output /tmp/reviewed-index.json
```

For authoring without network access, supply a local descriptor path instead. Production discovery requires HTTPS, a 15-second timeout, and homepage-origin equality. Loopback HTTP is a local authoring fixture and may advertise the intended production homepage. The command validates the entire output and writes it atomically. Review the diff before replacing/publishing the index. Import changes only `homepage` and the `website` snapshot; names, publishers, categories, supported platforms, release versions, artifact hashes, URLs, and reviewed permissions stay untouched. No plugin is installed or started.

Normal catalog refresh distributes the snapshot. Both GPUI and GTK render the same model through `PluginRecord`; the standard does not create a new plugin ABI. Readers of older catalogs work without these optional fields. Earlier clients ignore them. Offline metadata uses the last-known-good catalog; remote images retain independent loading/error/retry states. Without website previews, Market retains legacy screenshots and bundled preview fallbacks.

Website publishing and catalog publishing are separate actions. Editing a website does not silently change reviewed Market copy. Re-run the import and review the index to update it. Package/release generation preserves the descriptor, and the catalog merger carries it forward. Website source deployments do not publish a native plugin release.

## Reference website workflow

```sh
./scripts/website-dev.sh --site spotify
./scripts/website-deploy.sh --site spotify --dry-run
./scripts/website-deploy.sh --site spotify
```

Spotify's independent Pages project is `island-spotify`; Island remains `island-companion`. Each has its own Astro build, canonical URL, styling, and deployment. The old Island `/spotify/` URL permanently redirects to the plugin website.
