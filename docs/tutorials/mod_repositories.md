# Mod Repositories: Thunderstore and Hexium

Odin can install mods from two repositories:

| Repository             | Prefix | Website                                                         |
| ---------------------- | ------ | --------------------------------------------------------------- |
| Thunderstore (default) | `ts:`  | [thunderstore.io/c/valheim](https://thunderstore.io/c/valheim/) |
| Hexium                 | `hex:` | [hexium.gg](https://hexium.gg)                                  |

Both use the same dependency string format, `Author-ModName-Version`, so a list copied
from r2modman or from the Hexium client works with either. You choose where those strings
are looked up in two ways:

1. **`MODS_REPOSITORY`** sets the default for every entry in `MODS`.
2. **A prefix** (`ts:` or `hex:`) on a single entry overrides the default for that mod.

Direct download URLs (`https://...zip` or `.dll`) are downloaded as-is and never need a prefix.

## Pick a default with `MODS_REPOSITORY`

If most of your mods come from Hexium, make it the default:

```yaml
services:
  valheim:
    image: mbround18/valheim:3
    user: "1000:1000"
    environment:
      - TYPE=BepInEx
      - MODS_REPOSITORY=hexium
      - |
        MODS=Azumatt-AzuCraftyBoxes-1.8.18
        Smoothbrain-Building-*
        ValheimModding-Jotunn-2.30.0
```

| Value                  | Meaning                                                  |
| ---------------------- | -------------------------------------------------------- |
| _unset_                | Thunderstore. This is how Odin has always behaved.       |
| `thunderstore` or `ts` | Thunderstore                                             |
| `hexium` or `hex`      | Hexium                                                   |
| anything else          | A warning is logged and Odin falls back to Thunderstore. |

Values are case-insensitive.

## Mix repositories with a prefix

Put `hex:` or `ts:` in front of an entry to fetch that one mod from a specific repository,
whatever `MODS_REPOSITORY` says:

```yaml
environment:
  - TYPE=BepInEx
  - |
    MODS=hex:Azumatt-AzuCraftyBoxes-1.8.18
    hex:Smoothbrain-Building-*
    ts:RandyKnapp-EpicLoot-0.10.3
    ValheimModding-Jotunn-2.30.0
```

With `MODS_REPOSITORY` unset, that list resolves like this:

| Entry                               | Repository   | Why                               |
| ----------------------------------- | ------------ | --------------------------------- |
| `hex:Azumatt-AzuCraftyBoxes-1.8.18` | Hexium       | `hex:` prefix                     |
| `hex:Smoothbrain-Building-*`        | Hexium       | `hex:` prefix, newest version     |
| `ts:RandyKnapp-EpicLoot-0.10.3`     | Thunderstore | `ts:` prefix                      |
| `ValheimModding-Jotunn-2.30.0`      | Thunderstore | no prefix, so the default applies |

The prefixes are deliberately short so they are quick to type in front of a long list. The
full names, `hexium:` and `thunderstore:`, work too.

## Wildcard versions

The [wildcard patterns](./getting_started_with_mods.md#wildcard-version-patterns) work the
same way on both repositories. Each entry is resolved against its own repository, so
`hex:ValheimModding-Jotunn-*` picks the newest version published on Hexium, which may not
match the newest one on Thunderstore.

## Things to know

- **BepInEx still comes from Thunderstore.** `TYPE=BepInEx` installs BepInExPack_Valheim
  from `BEPINEX_RELEASES_URL` no matter what `MODS_REPOSITORY` is set to.
- **Dependencies are not installed for you**, from either repository. Odin installs exactly
  what `MODS` lists, so include every dependency string your mod manager shows.
- **Copy strings from the repository you point at.** A mod can have a newer version, or
  only exist, on one of the two. A string copied from Thunderstore may not resolve on
  Hexium, and the other way round.
- **Switching `MODS_REPOSITORY` takes effect on the next start.** Unprefixed mods are
  looked up again in the new repository. Mods removed from `MODS` are uninstalled as
  usual, whichever repository they came from.

## Authentication

Both repositories serve mod listings and downloads publicly, so no token is needed to
install mods.

| Variable             | Sent to                                   | Notes                                                                      |
| -------------------- | ----------------------------------------- | -------------------------------------------------------------------------- |
| `HEXIUM_TOKEN`       | `hexium.gg` and its subdomains only       | Optional Hexium API token (`hexium_...`), sent as `Authorization: Bearer`. |
| `THUNDERSTORE_TOKEN` | `thunderstore.io` and its subdomains only | Optional. See [Getting a Thunderstore API Token](./thunderstore_token.md). |

A repository's token is never sent to the other repository, and never to a
`*_BASE_URL` override such as a mirror.

## Downloads and rate limits

Hexium traffic goes through the same download pool as Thunderstore: the same concurrency
budget, the same retries, and the same handling of `429 Too Many Requests` and
`Retry-After`. The settings in
[Troubleshooting: Rate Limited Downloads](./getting_started_with_mods.md#troubleshooting-rate-limited-downloads-http-429)
apply to both.

## Troubleshooting

| Message                                                | What it means                                                                                                                                                                  |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Author-Mod-1.2.3 was not found on Hexium`             | Hexium has no such package or version. Check `https://hexium.gg/c/valheim/p/Author/Mod/`, fix the version, or prefix the entry with `ts:` to get it from Thunderstore instead. |
| `could not list Hexium versions for Author-Mod`        | A wildcard was used for a package Hexium does not have.                                                                                                                        |
| `No matching version found for wildcard ... on Hexium` | The package exists, but no published version matches the pattern (for example `3.*` when only `2.x` exists).                                                                   |
| `Unknown MODS_REPOSITORY="..."`                        | The value is not `thunderstore`, `ts`, `hexium` or `hex`. Odin carried on with Thunderstore.                                                                                   |

Set `MODS_CONTINUE_ON_FAILURE=true` to start the server with whatever installed
successfully while you sort out a missing mod.

## How it works

Hexium describes its API as Thunderstore-compatible, and the package metadata does match.
Downloads work differently, though, which is why pointing `THUNDERSTORE_BASE_URL` at
Hexium is not enough:

|                          | Thunderstore                                                          | Hexium                                                                                             |
| ------------------------ | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Exact version            | `/package/download/{author}/{name}/{version}/`, redirected to its CDN | `/api/experimental/package/{author}/{name}/{version}/` returns a `download_url` on `cdn.hexium.gg` |
| Version list (wildcards) | `/api/experimental/package/{author}/{name}/`, with fallbacks          | `/api/experimental/frontend/c/valheim/p/{author}/{name}/`                                          |
| Base URL override        | `THUNDERSTORE_BASE_URL`                                               | `HEXIUM_BASE_URL`                                                                                  |

Hexium CDN files are named after the version alone (`/upload/48/1.8.18.zip`), so Odin names
cached downloads after the full package (`Azumatt-AzuCraftyBoxes-1.8.18.zip`) to keep two
mods at the same version apart.

## Environment variables

| Variable          | Default             | Description                                                                                    |
| ----------------- | ------------------- | ---------------------------------------------------------------------------------------------- |
| `MODS_REPOSITORY` | `thunderstore`      | Default repository for unprefixed `MODS` entries: `thunderstore`/`ts` or `hexium`/`hex`.       |
| `HEXIUM_TOKEN`    | _unset_             | Optional Hexium API token, sent as `Authorization: Bearer` to `hexium.gg` and its subdomains.  |
| `HEXIUM_BASE_URL` | `https://hexium.gg` | Base URL for Hexium API lookups. Override for a mirror, or to point at a mock server in tests. |

## For contributors

Repository support lives in `src/odin/utils/mod_repository.rs` (names, prefixes, base
URLs, credentials) and `src/odin/mods/valheim_mod.rs` (version listing and download URL
resolution). If you add a repository, give it a short alias of two or three letters, like
`ts` and `hex`. There is a unit test that enforces this.

```sh
# Unit tests plus the end-to-end suite, which runs the real odin binary against mock servers
cargo test -p odin

# Only the end-to-end suite
cargo test -p odin --test mod_repositories_e2e

# Opt-in live tests against hexium.gg
HEXIUM_LIVE_TEST=1 cargo test -p odin hexium_live -- --ignored
```
