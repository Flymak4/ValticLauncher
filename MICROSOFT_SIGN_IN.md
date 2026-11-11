# Microsoft sign-in configuration

Horizon's account system needs a Microsoft Entra/Azure **public client application ID**. Do not use someone else's private credentials, and do not place client secrets in the app. The client ID is a public identifier, not a password.

Follow the upstream guide in `docs/microsoft-account.md` to create a registered application configured for device-code sign-in. Set its public **Application (client) ID** as `HORIZON_MSA_CLIENT_ID` when building or running Valtic Launcher.

For the supplied Windows build script, you can set an environment variable called `VALTIC_MSA_CLIENT_ID` before you start it. For the included GitHub Actions workflow, configure the repository variable `VALTIC_MSA_CLIENT_ID` in **Settings → Secrets and variables → Actions → Variables**. The workflow maps it to the name expected by the original account code.

Without this value, the interface can build and open, but you cannot sign in with Microsoft. Owning Minecraft Java Edition is still required to play.
