package dev.valtic.cosmetics.client.model;

import net.minecraft.client.render.entity.model.EntityModelLayer;
import net.minecraft.util.Identifier;

public class CosmeticModelLayers {
    public static final EntityModelLayer WINGS = new EntityModelLayer(
            Identifier.of("valtic_cosmetics", "wings"), "main");
    public static final EntityModelLayer CAPE = new EntityModelLayer(
            Identifier.of("valtic_cosmetics", "cape"), "main");
    public static final EntityModelLayer HAT = new EntityModelLayer(
            Identifier.of("valtic_cosmetics", "hat"), "main");
    public static final EntityModelLayer HORNS = new EntityModelLayer(
            Identifier.of("valtic_cosmetics", "horns"), "main");
}
