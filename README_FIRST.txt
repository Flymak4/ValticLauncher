VALTIC LAUNCHER — SOURCE PROJECT
================================

This folder contains an edited copy of the REAL open-source Horizon Rust launcher.
It has a dark charcoal / crimson theme, Valtic branding, a left navigation
sidebar, Valtic desktop icons and a separate app install name.

IMPORTANT: This is not a pre-built app. An .exe has to be compiled on Windows.
No Minecraft account or game files are included.

EASIEST WAY TO BUILD ON WINDOWS
-----------------------------

1. Extract the ENTIRE ZIP to a folder (not inside the ZIP viewer).
2. Install Rust from: https://rustup.rs/
   Choose the default Windows installation (MSVC).
3. Install Visual Studio Build Tools from:
   https://visualstudio.microsoft.com/downloads/
   Select "Desktop development with C++".
4. Double-click BUILD_WINDOWS.bat in this folder.
5. When it reports SUCCESS, open the BUILT_APP folder.
   The portable "Valtic Launcher.exe" is the application.
   "valtic-launcher-setup.exe" is a separate optional installer.

If you don't want to install developer tools, you can also compile on GitHub
using the included .github/workflows/build-valtic-windows.yml workflow.
That still requires uploading these files to a repository and running Actions.

MICROSOFT ACCOUNT SIGN-IN
-------------------------
Microsoft sign-in requires a registered public client application ID. Horizon
reads the HORIZON_MSA_CLIENT_ID environment variable at BUILD time (or at
run time). A build without an ID can open the interface but cannot sign in to
launch Minecraft until an ID is supplied. Set VALTIC_MSA_CLIENT_ID in your
Windows environment before running BUILD_WINDOWS.bat, or set the repository
variable VALTIC_MSA_CLIENT_ID for GitHub Actions.
See MICROSOFT_SIGN_IN.md for details. The account must own Java Edition.

WHAT IS CURRENTLY COMPLETE
--------------------------
- Valtic Launcher name, Windows window title, logo and Windows icon
- Red #E5484D accent, charcoal #0E1014 background and solid dark cards
- Left navigation for Play, Discover, Servers, Versions and Settings
- Reused genuine Horizon account, instance, Java mod, download and play systems
- Separate Valtic launcher data path and Windows setup registration
- Launcher self-update blocked to avoid replacing Valtic with upstream Horizon
- Windows build script and optional manual GitHub Actions build

WHAT IS NOT COMPLETE
--------------------
- Not yet compiled or tested on Windows; this is a first integrated source fork
- Not a pixel-perfect Modrinth replica; existing Horizon screens are retained
- No independent Valtic update server or signed releases
- No new Microsoft application ID supplied
- Horizon's separately developed Minecraft engine retains its own name

ORIGINAL OPEN SOURCE: https://github.com/horizon-rust/horizon
The original MIT LICENSE and THIRD_PARTY_NOTICES.md are included. All existing
rights and notices must be kept on redistribution. This is not an official
Minecraft, Mojang, Microsoft or Modrinth product.
