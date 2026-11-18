package dev.valtic.cosmetics.client;

import dev.valtic.cosmetics.client.model.CosmeticModelLayers;
import dev.valtic.cosmetics.client.render.CosmeticRenderer;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.rendering.v1.EntityModelLayerRegistry;
import net.fabricmc.fabric.api.client.rendering.v1.LivingEntityFeatureRendererRegistrationCallback;
import net.minecraft.client.render.entity.LivingEntityRenderer;
import net.minecraft.client.render.entity.model.EntityModel;
import net.minecraft.client.render.entity.state.PlayerEntityRenderState;
import net.minecraft.entity.EntityType;
import net.minecraft.entity.player.PlayerEntity;

public class ValticCosmeticsClient implements ClientModInitializer {

    @Override
    public void onInitializeClient() {
        // Register model layer definitions first
        EntityModelLayerRegistry.registerModelLayer(
                CosmeticModelLayers.WINGS, dev.valtic.cosmetics.client.model.WingsModel::getTexturedModelData);
        EntityModelLayerRegistry.registerModelLayer(
                CosmeticModelLayers.CAPE,  dev.valtic.cosmetics.client.model.CapeModel::getTexturedModelData);
        EntityModelLayerRegistry.registerModelLayer(
                CosmeticModelLayers.HAT,   dev.valtic.cosmetics.client.model.HatModel::getTexturedModelData);
        EntityModelLayerRegistry.registerModelLayer(
                CosmeticModelLayers.HORNS, dev.valtic.cosmetics.client.model.HornsModel::getTexturedModelData);

        // Register the feature renderer for the player
        LivingEntityFeatureRendererRegistrationCallback.EVENT.register(
                (entityType, entityRenderer, registrationHelper, context) -> {
                    if (entityType == EntityType.PLAYER) {
                        @SuppressWarnings("unchecked")
                        LivingEntityRenderer<PlayerEntity, EntityModel<PlayerEntityRenderState>> playerRenderer =
                                (LivingEntityRenderer<PlayerEntity, EntityModel<PlayerEntityRenderState>>) entityRenderer;
                        registrationHelper.register(
                                new CosmeticRenderer<>(playerRenderer, context)
                        );
                    }
                }
        );
    }
}
