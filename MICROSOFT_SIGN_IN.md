# Microsoft sign-in setup for Valtic Launcher

Valtic uses Microsoft's device-code sign-in and requires a Microsoft account that owns Minecraft: Java Edition. It never asks players for their Microsoft password.

## One-time developer setup

1. In [Microsoft Entra](https://entra.microsoft.com/), register a **public client** application named **Valtic Launcher** that accepts personal Microsoft accounts.
2. Under Authentication, enable **Allow public client flows** (device-code sign-in). No client secret is needed. Copy the **Application (client) ID** (a GUID).
3. Apply for Minecraft authentication app-ID review using the [official review link](https://aka.ms/mce-reviewappid). New registrations can be refused by Minecraft Services until approved.
4. In GitHub, open **Flymak4/ValticLauncher → Settings → Secrets and variables → Actions → Variables**. Add `VALTIC_MSA_CLIENT_ID` with that GUID.
5. Re-run **Actions → Build Valtic Launcher (Windows)**. The build checks the configured ID rather than publishing a launcher that cannot sign in.

The client ID is public information, not a password. Do **not** put a client secret in the desktop launcher. A valid GUID passes build validation, but actual Microsoft sign-in and Minecraft API access still require proper registration and any required approval.

For local builds, set `VALTIC_MSA_CLIENT_ID` before running Cargo. Internal compatibility with the previous `HORIZON_MSA_CLIENT_ID` environment variable remains, but Valtic's supported build variable is `VALTIC_MSA_CLIENT_ID`.
