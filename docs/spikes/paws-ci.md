# Spike: replacing CI/CD with paws

Branch `spike/paws-ci`. This is a spike: it replaces this repo's GitHub Actions CI/CD with
[paws](https://github.com/mbround18/paws), following its
[`llms.txt`](https://github.com/mbround18/paws/blob/main/llms.txt), to find out what
carries over, what doesn't, and what would have to change before doing it for real.

## What changed

| Before                                                                                  | After                                                                                                                                           |
| --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| **Build & Check** `build_rust`: setup-rust-toolchain + fmt / clippy / test              | `ci.yml` **rust**: `paws ci --toolchain rust`                                                                                                   |
| **Build & Check** `build_docker`: buildx + docker-meta + build-push, `canary` push      | `ci.yml` **docker**: `paws docker --target <image> --labels ... --tag-sha`                                                                      |
| **Build & Check** `lint` + `test_scripts`                                               | `ci.yml` **checks**: the same prettier and shell-test commands, as plain steps                                                                  |
| **Build & Check** permission check + **Container E2E**                                  | `ci.yml` **image**: `docker buildx build --load`, then the same two scripts                                                                     |
| (none)                                                                                  | `ci.yml` **release-rehearsal**: `paws semver` + `paws release --no-upload` on every PR                                                          |
| **Release**: `make release`, zip, Intuit `auto` (tag, GitHub Release, changelog)        | `release.yml` **release**: `paws semver --push`, then `paws changelog --commit`, then `paws release`                                            |
| **Docker Release** (on `release: published`): buildx + docker-meta to Docker Hub + GHCR | `release.yml` **images**: `paws docker --version <v> --registries ghcr.io --with-latest --tag-rollup --tag-sha`, run straight after the release |
| **Enforce PR labels**                                                                   | unchanged. paws has no label check, and `enforce-label` is `main`'s only required check                                                         |

Every paws step has a local equivalent: `make paws-ci`, `make paws-docker TARGET=odin`,
`make paws-release-dry-run`.

One code change came out of it: **`huginn --version`** now prints the version and exits
(`src/huginn/main.rs`, test in `src/huginn/tests/version_flag.rs`). `paws release`
smoke-tests every binary with `<binary> --version`, and huginn used to ignore its arguments
and start the server, so the release would have hung.

## What ran locally

All on this branch, with the paws CLI and dagger v0.21.8:

| Command                                                                                                                    | Result                                                                                                                                                                                   |
| -------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `paws ci --toolchain rust`                                                                                                 | ✅ 257 s from cold. fmt, clippy `-D warnings`, build and every test: 275 odin lib, 276 odin bin, 3 mod-repository e2e, plus huginn                                                       |
| `paws docker --image mbround18/odin --target odin`                                                                         | ✅ 296 s, build only. It reported one tag to one registry for a real push                                                                                                                |
| `paws semver --labels minor` / `patch` / `major`                                                                           | ✅ `v3.9.0` / `v3.8.1` / `v4.0.0`, from `v3.8.0`                                                                                                                                         |
| `paws changelog --version main --previous-ref v3.7.2`                                                                      | ⚠️ works, but see gap 1                                                                                                                                                                  |
| `paws release --local-build --no-upload --target x86_64-unknown-linux-gnu --package odin,huginn --binary-name odin,huginn` | ✅ 181 s. Both binaries built and passed the smoke test (`odin 2.2.0`, `huginn 0.1.1` after the fix above), then were packaged into `odin+huginn-<version>-x86_64-unknown-linux-gnu.zip` |
| `paws workflow generate`                                                                                                   | detects Rust and Docker. Emits `paws-up`, then `paws ci --toolchain rust`, then `paws docker`, which is the starting point `ci.yml` builds on                                            |

## What ran in GitHub Actions

On the spike PR (`1811621`), every job in the new `ci.yml` passed on `ubuntu-latest`:

| Job                                                           | Time   |
| ------------------------------------------------------------- | ------ |
| Rust (`paws ci`)                                              | 6 min  |
| Docker (odin), build only                                     | 6 min  |
| Docker (valheim), build only                                  | 8 min  |
| Image checks (build, then permission check)                   | 3 min  |
| Release rehearsal (`paws semver`, `paws release --no-upload`) | 4 min  |
| Lint & shell tests                                            | <1 min |

These were cold runs. The GitHub Actions cache backend `paws-up` enables should shorten repeat runs.

## Gaps, most important first

1. **The changelog listed every commit, not every PR. Fixed in paws, not released yet.**
   - The released `paws changelog` titles each commit with its PR's title, so one merged PR becomes as many identical lines as it has commits. `v3.7.2..main` rendered #1512 about 15 times.
   - It also included `[skip ci]` changelog commits.
   - Fixed upstream in paws commit `a3b446b` (mbround18/paws#29): one line per PR, keyed by PR number and rendered `- <title> (#<number>)`, with `[skip ci]`/`[ci skip]` commits left out. Built from that branch, the same `v3.7.2..main` run gives five lines, #1507, #1508, #1509, #1511 and #1512, where it gave 26.
   - `release.yml` gets the fix once a paws release containing it is out.
2. **`paws docker` doesn't load images into the runner's Docker.** The build happens inside Dagger, so the permission check and the container e2e still need their own `docker buildx build --load`. That's a second image build per run.
3. **Registry build cache.** The old workflows wrote `mbround18/<image>:buildcache`, and paws uses Dagger's cache (the GitHub Actions backend that `paws-up` enables) instead. The `--cache-from` in the image job will go stale once nothing writes that cache any more.
4. **Clippy doesn't lint tests.** `paws ci` runs `cargo clippy -- -D warnings`, without `--all-targets`, so test code isn't linted. Current CI lints it. Fix upstream in paws.
5. **No Docker build args from the CLI.** paws only reads build args from a `compose.yml` service. `GITHUB_SHA`, `GITHUB_REF` and `GITHUB_REPOSITORY` are therefore not passed, and `/home/steam/.version` in the image says `not-set`. Nothing in the code reads that file today, so it only affects anyone inspecting the image by hand. `ODIN_IMAGE_VERSION` was never declared as an `ARG`, so dropping it changes nothing.
6. **The exact-version image tag changes.** paws tags versions as `:v3.9.0`; `docker-meta` published `:3.9.0`. `--tag-rollup` keeps `:3` and `:3.9`, and `:latest` is still published. Anyone pinned to an exact `:3.x.y` tag would need `:v3.x.y` from the first paws release.
7. **Push behavior changes.**
   - odin was pushed on every PR build. Now both images push on a PR only with `canary`, which avoids failing on fork PRs, where secrets aren't available anyway.
   - Both images now also get `:<sha>` tags on every push to `main`.
   - Releases now run only from `main`. The old Release workflow ran on every branch push.
8. **Pin paws.** `paws-up@main` with `version: latest` resolves to the newest _prerelease_. For stable CI, pin `paws-up` to a commit and set `version:` to a known release.

## Not proven by the spike

The publishing half of `release.yml` only runs on `main`: `semver --push`, `changelog --commit`, the release upload and the image pushes. On a PR it's only rehearsed without publishing, through **release-rehearsal**. The first real run would happen on the first merge. Its secrets are the ones the old workflows already use: `GH_TOKEN`, `DOCKER_TOKEN` and `GHCR_TOKEN`.

## Recommendation

paws can take over the build side today. `paws ci` matched the current Rust job exactly, and `paws docker` built the images. The release side needs two things first:

- **The changelog fix** (gap 1). Otherwise every release note repeats each PR once per commit.
- **A decision on the image tag format** (gap 6).

A reasonable path:

1. Adopt `ci.yml` now, with paws pinned.
2. Keep the current release workflows until the changelog gap is fixed upstream.
3. Then switch to `release.yml`.
