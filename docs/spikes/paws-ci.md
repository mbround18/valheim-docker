# Spike: replacing CI/CD with paws

Branch `spike/paws-ci`. This is a spike: it replaces this repo's GitHub Actions CI/CD with
[paws](https://github.com/mbround18/paws), following its
[`llms.txt`](https://github.com/mbround18/paws/blob/main/llms.txt), to find out what
carries over, what doesn't, and what would have to change before doing it for real.

## What changed

| Before                                                                                  | After                                                                                                                                                                                                             |
| --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Build & Check** `build_rust`: setup-rust-toolchain + fmt / clippy / test              | `ci.yml` **rust**: `paws ci --toolchain rust`                                                                                                                                                                     |
| **Build & Check** `build_docker`: buildx + docker-meta + build-push, `canary` push      | `ci.yml` **docker**: one job, one `paws docker --image mbround18/odin,mbround18/valheim --target odin,valheim` call, so valheim reuses odin's Rust compile                                                        |
| **Build & Check** `lint` + `test_scripts`                                               | `ci.yml` **checks**: the same prettier and shell-test commands, as plain steps                                                                                                                                    |
| **Build & Check** permission check + **Container E2E**                                  | `ci.yml` **image**: `docker buildx build --load`, then the same two scripts                                                                                                                                       |
| (none)                                                                                  | `ci.yml` **release-rehearsal**: `paws semver` + `paws release --no-upload` on every PR                                                                                                                            |
| **Release**: `make release`, zip, Intuit `auto` (tag, GitHub Release, changelog)        | `release.yml` **release**: `paws semver --push`, then `paws changelog --commit`, then `paws release`                                                                                                              |
| **Docker Release** (on `release: published`): buildx + docker-meta to Docker Hub + GHCR | `images.yml`, on the release tag push: one job, one `paws docker --image mbround18/odin,mbround18/valheim --target odin,valheim --version <tag> --registries ghcr.io --no-prefix --with-latest --tag-rollup` call |
| **Enforce PR labels**                                                                   | unchanged. paws has no label check, and `enforce-label` is `main`'s only required check                                                                                                                           |

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

Then both images moved into one job, and paws got two fixes that job needed:

| Docker job                                        | Time                                  |
| ------------------------------------------------- | ------------------------------------- |
| Two jobs, one per image (`1811621`)               | 14 runner-minutes (6 odin, 8 valheim) |
| One job, two `paws docker` steps (`4b0a0fd`)      | 5.7 min (4.6 odin, 0.9 valheim)       |
| One job, two steps on `prerelease.47` (`cad19f8`) | 2.2 min (1.3 odin, 0.8 valheim)       |
| One job, one two-target call on `prerelease.48`   | see gap 3                             |

valheim costs under a minute because it reuses odin's Rust compile rather than repeating it. The drop from 5.7 to 2.2 is the cache fix in `prerelease.47`: odin's step had been spending about 2.5 minutes archiving the engine to a save the Actions cache then refused.

## Gaps, most important first

1. **The changelog listed every commit, not every PR. Fixed in paws `v0.0.1-prerelease.45`.**
   - Before that release, `paws changelog` titled each commit with its PR's title, so one merged PR became as many identical lines as it had commits. `v3.7.2..main` rendered #1512 about 15 times.
   - It also included `[skip ci]` changelog commits.
   - Fixed in mbround18/paws#29 (merged as `fe42c52`, released in `v0.0.1-prerelease.45`): one line per PR, keyed by PR number and rendered `- <title> (#<number>)`, with `[skip ci]`/`[ci skip]` commits left out.
   - With the released `prerelease.45` binary, the same `v3.7.2..main` run gives five lines, #1507, #1508, #1509, #1511 and #1512, where it gave 26.
2. **`paws docker` doesn't load images into the runner's Docker.** The build happens inside Dagger, so the permission check and the container e2e still need their own `docker buildx build --load`.
   - **Build once for both images.** The Dockerfile compiles odin and huginn once, in `odin-builder`, and the `valheim` image copies them from the `odin` stage. With one job per image, each runner compiled the whole Rust workspace from scratch, so a PR compiled it three times: odin, valheim, and the image checks. Both paws builds now run in one job on the same Dagger engine, where valheim reuses odin's compile. `images.yml` publishes both the same way.
   - **One duplicate build remains.** The image-checks job still has its own `buildx` build, because paws can't load the image into the runner's Docker. A `--load` option on `paws docker`, feeding the checks from the same Dagger build, would remove it. That would be an upstream change like `--no-prefix`.
3. **Build cache.**
   - **Registry cache.** The old workflows wrote `mbround18/<image>:buildcache`, and paws uses Dagger's cache (the GitHub Actions backend that `paws-up` enables) instead. The `--cache-from` in the image job will go stale once nothing writes that cache any more.
   - **Two paws calls in one job broke the build. Fixed in paws `v0.0.1-prerelease.47`.**
     - Every `paws docker` call restores the Actions cache when it starts: it stops the engine and extracts the cached archive onto its volume.
     - With odin and valheim built in one job, the valheim call extracted the archive over the engine the odin call had just used. The build then failed with `failed to rename .../snapshots/62: file exists`.
     - A first workaround ran the second step with `ACTIONS_RUNTIME_TOKEN` unset, so paws skipped the cache for it entirely. That is no longer needed.
     - Saving had a smaller problem. The cache key is fixed per Dagger version and Actions cache entries can't be overwritten, so every save after the first spent about 2.5 minutes archiving the engine volume and was then refused (409).
     - Fixed in mbround18/paws#31 (merged as `9cef442`, released in `v0.0.1-prerelease.47`): the cache is restored at most once per job, and the save is skipped when the entry already exists. The spike is pinned to that release and the workaround is gone.
   - **One call now builds both images. Added in paws `v0.0.1-prerelease.48`.**
     - `--image` and `--target` take lists, paired in order, so `--image mbround18/odin,mbround18/valheim --target odin,valheim` builds both against one engine, with one cache cycle for the pair. mbround18/paws#32.
     - `make paws-docker` runs the same pair locally; `TARGET=odin` still builds one.
     - Measured locally on this Dockerfile, cold: the second target builds in 42 s against the first's 106 s, and runs no `cargo build` of its own.
     - Not done: the targets still build one after the other. `docker buildx bake` with both targets took 116 s against 148 s for two sequential builds, because it overlaps valheim's apt install and SteamCMD download with the Rust compile. Closing that last gap needs concurrent Dagger sessions in paws.
4. **Clippy doesn't lint tests.** `paws ci` runs `cargo clippy -- -D warnings`, without `--all-targets`, so test code isn't linted. Current CI lints it. Fix upstream in paws.
5. **No Docker build args from the CLI.** paws only reads build args from a `compose.yml` service. `GITHUB_SHA`, `GITHUB_REF` and `GITHUB_REPOSITORY` are therefore not passed, and `/home/steam/.version` in the image says `not-set`. Nothing in the code reads that file today, so it only affects anyone inspecting the image by hand. `ODIN_IMAGE_VERSION` was never declared as an `ARG`, so dropping it changes nothing.
6. **Image tag format. Resolved in paws `v0.0.1-prerelease.46`.** paws used to tag versions `:v3.9.0` where `docker-meta` published `:3.9.0`. mbround18/paws#30 added `--version-prefix` and its shorthand `--no-prefix`, which set one prefix for the whole cascade. `images.yml` passes `--no-prefix`, so releases keep publishing `:3.9.0`, `:3.9`, `:3` and `:latest`, the same as before.
7. **Push behavior changes.**
   - odin was pushed on every PR build. Now both images push on a PR only with `canary`, which avoids failing on fork PRs, where secrets aren't available anyway.
   - Both images now also get `:<sha>` tags on every push to `main`.
   - Releases now run only from `main`. The old Release workflow ran on every branch push.
   - **Images publish from the release tag, not from `main`.** `paws docker` only adds `:latest` and the rollup tags when `GITHUB_REF` is a tag, so publishing in the same run as `paws semver --push`, a push to `main`, would silently skip them. `images.yml` runs on the `v*` tag push instead. That tag has to be created with a personal token (`GH_TOKEN`), because GitHub doesn't start workflows for tags pushed with the built-in `GITHUB_TOKEN`.
   - A release image no longer gets a `sha-<sha>` tag. `--tag-sha` only adds one when `--version` is itself a sha, which a release version isn't.
8. **Pin paws. Done.** `paws-up@main` with `version: latest` resolves to the newest _prerelease_, so every workflow now uses `mbround18/paws/actions/paws-up@v0.0.1-prerelease.48` with `version: v0.0.1-prerelease.48`. That pins both the action and the binary it installs. Moving to a newer paws is a deliberate edit to those lines. Pinning the action to a commit SHA instead of the tag would also protect against the tag being moved.

## Not proven by the spike

The publishing workflows can't run from a PR. `release.yml` runs on `main` (`semver --push`, `changelog --commit`, the binary upload), and `images.yml` runs on the tag that pushes. On a PR they're only rehearsed without publishing, through **release-rehearsal**. The first real run would happen on the first merge. They use the secrets the old workflows already use: `GH_TOKEN`, `DOCKER_TOKEN` and `GHCR_TOKEN`.

## Recommendation

paws can take over both halves:

- **Build side:** `paws ci` matched the current Rust job exactly, and `paws docker` built the images.
- **Release side:** both blockers are fixed upstream. The changelog is one line per PR (gap 1, `prerelease.45`), and releases keep the existing unprefixed image tags (gap 6, `prerelease.46`). Building both images in one job needed the cache fix in `prerelease.47`, and they now build in a single call thanks to `prerelease.48` (gap 3). The spike is pinned to `prerelease.48`.

Gaps 2–5 are rough edges, not blockers. Watch the first release closely: it's the first time the publishing half runs for real.
