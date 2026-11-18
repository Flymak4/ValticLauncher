package dev.valtic.cosmetics.data;

import net.minecraft.util.Identifier;

/**
 * Describes a single cosmetic item available in the wardrobe.
 */
public record CosmeticEntry(
        String id,
        String displayName,
        CosmeticType type,
        Identifier texture,
        String description
) {
    /** Convenience: texture identifier from a simple name under valtic_cosmetics namespace. */
    public static CosmeticEntry of(String id, String name, CosmeticType type, String texturePath, String desc) {
        return new CosmeticEntry(id, name, type,
                Identifier.of("valtic_cosmetics", texturePath), desc);
    }
}
