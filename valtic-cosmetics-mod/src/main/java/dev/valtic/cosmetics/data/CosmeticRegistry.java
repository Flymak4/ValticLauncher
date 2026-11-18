package dev.valtic.cosmetics.data;

import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;

/**
 * Hard-coded catalogue of all cosmetics.
 * In production this would be fetched from the Valtic backend.
 * For testing, every cosmetic is unlocked.
 */
public class CosmeticRegistry {

    // ── Wings ────────────────────────────────────────────────────────────────
    public static final CosmeticEntry WINGS_CHERRY     = CosmeticEntry.of("wings_cherry",    "Cherry Wings",     CosmeticType.WINGS, "cosmetics/wings/cherry",     "Delicate pink wings with cherry-blossom veining.");
    public static final CosmeticEntry WINGS_OBSIDIAN   = CosmeticEntry.of("wings_obsidian",  "Obsidian Wings",   CosmeticType.WINGS, "cosmetics/wings/obsidian",   "Dark, angular wings forged from deep obsidian.");
    public static final CosmeticEntry WINGS_AURORA     = CosmeticEntry.of("wings_aurora",    "Aurora Wings",     CosmeticType.WINGS, "cosmetics/wings/aurora",     "Translucent wings shimmering with aurora colours.");
    public static final CosmeticEntry WINGS_FEATHER    = CosmeticEntry.of("wings_feather",   "Feathered Wings",  CosmeticType.WINGS, "cosmetics/wings/feather",    "Realistic feathered wings, naturally animated.");

    // ── Capes ────────────────────────────────────────────────────────────────
    public static final CosmeticEntry CAPE_VALTIC      = CosmeticEntry.of("cape_valtic",     "Valtic Cape",      CosmeticType.CAPE,  "cosmetics/capes/valtic",     "The official Valtic Launcher cape.");
    public static final CosmeticEntry CAPE_BLOSSOM     = CosmeticEntry.of("cape_blossom",    "Blossom Cape",     CosmeticType.CAPE,  "cosmetics/capes/blossom",    "Cherry blossom pattern on a soft pink ground.");
    public static final CosmeticEntry CAPE_MIDNIGHT    = CosmeticEntry.of("cape_midnight",   "Midnight Cape",    CosmeticType.CAPE,  "cosmetics/capes/midnight",   "Deep black cape with starfield detail.");
    public static final CosmeticEntry CAPE_GOLD        = CosmeticEntry.of("cape_gold",       "Golden Cape",      CosmeticType.CAPE,  "cosmetics/capes/gold",       "Shimmering golden trim on a royal weave.");

    // ── Hats ─────────────────────────────────────────────────────────────────
    public static final CosmeticEntry HAT_CROWN        = CosmeticEntry.of("hat_crown",       "Royal Crown",      CosmeticType.HAT,   "cosmetics/hats/crown",       "An ornate gold crown with gem insets.");
    public static final CosmeticEntry HAT_WITCH        = CosmeticEntry.of("hat_witch",       "Witch Hat",        CosmeticType.HAT,   "cosmetics/hats/witch",       "Tall pointed hat, slightly crumpled.");
    public static final CosmeticEntry HAT_CHERRY       = CosmeticEntry.of("hat_cherry",      "Cherry Blossom Hat", CosmeticType.HAT, "cosmetics/hats/cherry",     "Wide-brimmed hat adorned with cherry blossoms.");
    public static final CosmeticEntry HAT_HALO         = CosmeticEntry.of("hat_halo",        "Halo",             CosmeticType.HAT,   "cosmetics/hats/halo",        "A floating golden halo.");

    // ── Horns ────────────────────────────────────────────────────────────────
    public static final CosmeticEntry HORNS_DEMON      = CosmeticEntry.of("horns_demon",     "Demon Horns",      CosmeticType.HORNS, "cosmetics/horns/demon",      "Classic curved demon horns, deep crimson.");
    public static final CosmeticEntry HORNS_SPIRAL     = CosmeticEntry.of("horns_spiral",    "Spiral Horns",     CosmeticType.HORNS, "cosmetics/horns/spiral",     "Elegant spiralling horns with ivory tones.");
    public static final CosmeticEntry HORNS_SAKURA     = CosmeticEntry.of("horns_sakura",    "Sakura Horns",     CosmeticType.HORNS, "cosmetics/horns/sakura",     "Delicate branch-like horns blooming with sakura.");
    public static final CosmeticEntry HORNS_CRYSTAL    = CosmeticEntry.of("horns_crystal",   "Crystal Horns",    CosmeticType.HORNS, "cosmetics/horns/crystal",    "Translucent crystalline horns catching the light.");

    private static final List<CosmeticEntry> ALL = Arrays.asList(
            WINGS_CHERRY, WINGS_OBSIDIAN, WINGS_AURORA, WINGS_FEATHER,
            CAPE_VALTIC, CAPE_BLOSSOM, CAPE_MIDNIGHT, CAPE_GOLD,
            HAT_CROWN, HAT_WITCH, HAT_CHERRY, HAT_HALO,
            HORNS_DEMON, HORNS_SPIRAL, HORNS_SAKURA, HORNS_CRYSTAL
    );

    public static List<CosmeticEntry> getAll() {
        return ALL;
    }

    public static List<CosmeticEntry> getByType(CosmeticType type) {
        return ALL.stream().filter(e -> e.type() == type).collect(Collectors.toList());
    }

    public static CosmeticEntry getById(String id) {
        return ALL.stream().filter(e -> e.id().equals(id)).findFirst().orElse(null);
    }
}
