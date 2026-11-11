# Valtic Launcher — red/charcoal desktop fork

This is an **integrated source fork** of [Horizon](https://github.com/horizon-rust/horizon), not a pre-built Windows application. The original licence and third-party notices remain in the project. It retains the genuine Rust/GPUI launcher and backend, while introducing Valtic branding, dark flat styling, red accents and a left-rail interface inspired by Modrinth (not its copyrighted iconography).

## Build

On **Windows**, extract all files, install [Rust](https://rustup.rs) and Visual Studio Build Tools (**Desktop development with C++** workload), then double-click `BUILD_WINDOWS.bat`. Its compiled portable and setup programs will appear in `BUILT_APP` if compilation succeeds.

Alternatively, create a repository containing this source and run **Actions → Build Valtic Launcher (Windows) → Run workflow**. The resulting artifact will contain the Windows application. This repository includes only the dedicated opt-in workflow; unrelated upstream scheduled CI has been excluded from this source distribution.

Rust command (if already configured):

```powershell
cd launcher
cargo build --release --locked --bin valtic-launcher
```

## Auth

`HORIZON_MSA_CLIENT_ID` must be set to your Microsoft public client app registration ID before building (or at runtime), otherwise Microsoft login will not be available. This is not a Minecraft bypass; you still need an account that owns Minecraft Java Edition. See `MICROSOFT_SIGN_IN.md` and the upstream `docs/microsoft-account.md`.

## Updates and safety

Valtic intentionally disables **launcher self-updates** from the upstream Horizon release feed so that the new app isn't replaced with an unrelated original executable. Horizon **game** releases and the Modrinth browser remain upstream. A distinct launcher data path, data-location pointer and Windows installation registry identity are used; the Java Minecraft data folder is not forcibly replaced. Official Horizon is credited as the foundation, and the MIT licence, third-party notices and game names are retained where appropriate.

This edition is **not compiled/tested in this environment**. Compile it before using; report build errors with logs so they can be fixed. The app's design is an early functional source rewrite, not a complete replacement of every page.
