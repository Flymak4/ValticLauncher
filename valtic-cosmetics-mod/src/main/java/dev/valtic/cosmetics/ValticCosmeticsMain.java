package dev.valtic.cosmetics;

import net.fabricmc.api.ModInitializer;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class ValticCosmeticsMain implements ModInitializer {
    public static final String MOD_ID = "valtic_cosmetics";
    public static final Logger LOGGER = LoggerFactory.getLogger(MOD_ID);

    @Override
    public void onInitialize() {
        LOGGER.info("[Valtic Cosmetics] Initialised.");
    }
}
