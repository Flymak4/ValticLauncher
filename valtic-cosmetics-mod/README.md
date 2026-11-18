# Valtic Cosmetics — Fabric 26.3 test mod

Built from the uploaded Valtic Cosmetics prototype, ported from obsolete Yarn
rendering APIs to official Minecraft 26.3 client rendering.

- In-game cosmetics menu on the title screen and pause menu.
- Wings, capes, hats and horns: four designs per category (16 total).
- Select **Equip** or **Unequip**; choice persists locally in `valtic_cosmetics.json`.
- Textures and 3D geometry are bundled in the mod; no separate resource pack.
- Only your own character renders cosmetics in this prototype.
- All variants are unlocked for testing. No purchases or webstore checks.
- Requires Fabric Loader and Fabric API for Minecraft Java 26.3.

The 16 PNG files in the uploaded ZIP were corrupt/non-decodable. To avoid
shipping a broken mod, this source includes 16 valid replacement palette variants
based on the older Valtic 3D cosmetic textures. The models share geometry per
category and differ by texture.

Visual appearance needs to be checked in the game before production release.
