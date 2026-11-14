# Microsoft sign-in for Valtic Launcher

Valtic uses Microsoft's real **device-code sign-in** (Microsoft browser, Xbox Live, Minecraft
Services). Players never enter a Microsoft password into Valtic. The verified Minecraft session
is passed to **Valtic's own game build**, not Horizon's downloadable launcher or game binary.

## Configure a Valtic application

1. In [Microsoft Entra](https://entra.microsoft.com/), register a public-client application
   **Valtic Launcher** that permits personal Microsoft accounts.
2. Enable **Allow public client flows**. Copy its **Application (client) ID** (a GUID). Do not
   put a client secret in a desktop app.
3. Submit the ID for Minecraft Services review using [Minecraft's app review](https://aka.ms/mce-reviewappid).
   Successful Microsoft sign-in alone does not guarantee Minecraft Services access.
4. Valtic's existing public-client registration `5cc9c47c-7e6e-4012-9ef3-b73b06d73e7a`
   is included as the default in the Windows build workflow. In
   **Flymak4/ValticLauncher → Settings → Secrets and variables → Actions → Variables**,
   you can override `VALTIC_MSA_CLIENT_ID` if Valtic's registration changes.
5. Until approved, leave `VALTIC_MSA_APPROVED` set to `false`. After Microsoft/Minecraft
   approve Valtic, set this variable to `true`, then rebuild/publish the launcher.

## Optional temporary authentication while waiting

You may set `VALTIC_TEMP_MSA_CLIENT_ID` to another public-client application ID **only if its
owner has explicitly permitted Valtic to use it and this use is allowed by Microsoft/Minecraft**.
Do not copy Horizon's client ID from source code or impersonate another launcher's app registration.

When an authorized temporary ID exists and `VALTIC_MSA_APPROVED` is not `true`, Valtic
automatically selects the temporary ID for the same Microsoft device-code flow. All the launcher
UI, saved Minecraft instances, installed mods and game binaries stay Valtic's. Without an
authorized temporary ID, Valtic's own ID is used (and Minecraft Services can deny access until
its approval).

| GitHub Actions variable | While waiting | After Valtic approval |
| --- | --- | --- |
| `VALTIC_MSA_CLIENT_ID` | Valtic's registration | Same registration |
| `VALTIC_TEMP_MSA_CLIENT_ID` | Authorized temporary ID, if available | May remain for fallback |
| `VALTIC_MSA_APPROVED` | `false` | `true` |

The new build automatically prefers Valtic's client ID when that flag becomes true. Players
whose saved Microsoft refresh tokens belong to a different app are asked to sign in again; the
launcher does not attempt to redeem a temporary app's token with Valtic's app ID. Other saved
instances, installed mods, account names, and local game data are preserved.

**Approval is not detectable automatically from inside the launcher.** The publisher must
confirm it and set `VALTIC_MSA_APPROVED=true` before publishing the transition update.
This is *not* a way to bypass Microsoft authentication, Minecraft ownership, or application
approval.

Local builds can set the same variables before running Cargo. The legacy
`HORIZON_MSA_CLIENT_ID` is read only for compatibility with explicitly authorized
configurations; Valtic builds no longer embed a third-party ID by default.

## Why the November 14 sign-in build failed

An earlier update accidentally deleted the configured default Valtic client ID from
`.github/workflows/build-valtic-windows.yml`. Because no Actions override was set, the
launcher was compiled without any Microsoft app registration and showed
`Microsoft sign-in is not configured`. The registered Valtic ID has been restored; the
workflow now refuses to publish if neither the Valtic nor an authorized temporary ID is valid.
Microsoft/Minecraft still control application acceptance and account authentication.
