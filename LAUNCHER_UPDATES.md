# Valtic Launcher – in-app updates

The Windows launcher checks for newer releases on startup and every 30 minutes. When
a release is available it verifies the archive's GitHub SHA-256 digest, downloads
it in the background, and shows **New Valtic Launcher update available** with
**Update & Restart**. Existing game files, worlds, accounts and settings are not
removed by this process.

## One-time setup for distribution

**Your source repository is private.** Releases in a private repository are
also private, so players cannot use them for automatic updates. Do not place a
GitHub token in the launcher to solve this.

1. Create a second **public** GitHub repository under the same account, e.g.
   `Flymak4/ValticLauncher-Releases`, with a main branch. This repository
   will host only downloadable release ZIPs, not your private source.
2. In the private **ValticLauncher** repository, open Settings > Secrets and
   variables > Actions > Variables. Add `VALTIC_UPDATE_REPO` with the exact
   public repository name, e.g. `Flymak4/ValticLauncher-Releases`.
3. Add a repository Actions secret called `VALTIC_RELEASE_TOKEN` containing a
   **fine-grained** GitHub token with **Contents: Read and write** on the
   public releases repository. Do not expose the token in files or the app.
4. Run the Windows build workflow (or push a change to `launcher/**`). Its
   successful build automatically publishes a release named
   `launcher@0.2.<run-number>` with the Windows updater ZIP and normal ZIP.
5. Distribute `valtic-launcher-setup.exe` from the ZIP **once**. Players
   installing that version can update from inside Valtic Launcher afterward.

The workflow checks the release repository's visibility and reports a warning
when it is not public. Each successful workflow execution gets a new numbered
version; failed builds do not publish a release. The launcher still obtains
its mod/game builds separately from the upstream project.

**Security:** The updater only installs release archives fetched over HTTPS
from the configured GitHub release repository, and checks the exact tag, asset
name, URL, size and SHA-256 digest against GitHub's API before replacement.
This protects against corrupted downloads but does not provide independent
cryptographic release signatures. Treat the public release repository and its
fine-grained publishing token as trusted publishing infrastructure.
