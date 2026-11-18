package dev.valtic.cosmetics.data;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonObject;
import net.fabricmc.loader.api.FabricLoader;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.IOException;
import java.io.Reader;
import java.io.Writer;
import java.nio.file.Files;
import java.nio.file.Path;

/**
 * Tracks which cosmetics the local player has equipped.
 * Saved to .minecraft/valtic_cosmetics.json — one ID per slot, or null.
 */
public class PlayerCosmeticState {

    private static final Logger LOGGER = LoggerFactory.getLogger("valtic_cosmetics");
    private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();
    private static final Path SAVE_PATH = FabricLoader.getInstance()
            .getGameDir().resolve("valtic_cosmetics.json");

    private static PlayerCosmeticState INSTANCE = null;

    private String equippedWings  = null;
    private String equippedCape   = null;
    private String equippedHat    = null;
    private String equippedHorns  = null;

    private PlayerCosmeticState() {}

    public static PlayerCosmeticState get() {
        if (INSTANCE == null) INSTANCE = load();
        return INSTANCE;
    }

    // ── Getters / setters ────────────────────────────────────────────────────

    public String getEquipped(CosmeticType type) {
        return switch (type) {
            case WINGS  -> equippedWings;
            case CAPE   -> equippedCape;
            case HAT    -> equippedHat;
            case HORNS  -> equippedHorns;
        };
    }

    public CosmeticEntry getEquippedEntry(CosmeticType type) {
        String id = getEquipped(type);
        return id == null ? null : CosmeticRegistry.getById(id);
    }

    public void equip(CosmeticEntry entry) {
        switch (entry.type()) {
            case WINGS  -> equippedWings  = entry.id();
            case CAPE   -> equippedCape   = entry.id();
            case HAT    -> equippedHat    = entry.id();
            case HORNS  -> equippedHorns  = entry.id();
        }
        save();
    }

    public void unequip(CosmeticType type) {
        switch (type) {
            case WINGS  -> equippedWings  = null;
            case CAPE   -> equippedCape   = null;
            case HAT    -> equippedHat    = null;
            case HORNS  -> equippedHorns  = null;
        }
        save();
    }

    public boolean isEquipped(CosmeticEntry entry) {
        return entry.id().equals(getEquipped(entry.type()));
    }

    // ── Persistence ──────────────────────────────────────────────────────────

    private static PlayerCosmeticState load() {
        PlayerCosmeticState state = new PlayerCosmeticState();
        if (!Files.exists(SAVE_PATH)) return state;
        try (Reader r = Files.newBufferedReader(SAVE_PATH)) {
            JsonObject obj = GSON.fromJson(r, JsonObject.class);
            if (obj != null && obj.has("wings"))  state.equippedWings  = obj.get("wings").isJsonNull()  ? null : obj.get("wings").getAsString();
            if (obj != null && obj.has("cape"))   state.equippedCape   = obj.get("cape").isJsonNull()   ? null : obj.get("cape").getAsString();
            if (obj != null && obj.has("hat"))    state.equippedHat    = obj.get("hat").isJsonNull()    ? null : obj.get("hat").getAsString();
            if (obj != null && obj.has("horns"))  state.equippedHorns  = obj.get("horns").isJsonNull()  ? null : obj.get("horns").getAsString();
        } catch (IOException e) {
            LOGGER.error("[Valtic Cosmetics] Failed to load save: {}", e.getMessage());
        }
        return state;
    }

    private void save() {
        JsonObject obj = new JsonObject();
        obj.addProperty("wings",  equippedWings);
        obj.addProperty("cape",   equippedCape);
        obj.addProperty("hat",    equippedHat);
        obj.addProperty("horns",  equippedHorns);
        try (Writer w = Files.newBufferedWriter(SAVE_PATH)) {
            GSON.toJson(obj, w);
        } catch (IOException e) {
            LOGGER.error("[Valtic Cosmetics] Failed to save state: {}", e.getMessage());
        }
    }
}
