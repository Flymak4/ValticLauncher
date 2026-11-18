package dev.valtic.cosmetics.client;

import dev.valtic.cosmetics.client.gui.CosmeticsWardrobeScreen;
import dev.valtic.cosmetics.client.render.CosmeticsLayer;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.rendering.v1.LivingEntityRenderLayerRegistrationCallback;
import net.fabricmc.fabric.api.client.rendering.v1.ModelLayerRegistry;
import net.fabricmc.fabric.api.client.screen.v1.ScreenEvents;
import net.fabricmc.fabric.api.client.screen.v1.Screens;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.PauseScreen;
import net.minecraft.client.gui.screens.TitleScreen;
import net.minecraft.client.renderer.entity.player.AvatarRenderer;
import net.minecraft.network.chat.Component;

public final class ValticCosmeticsClient implements ClientModInitializer {
    @Override
    public void onInitializeClient() {
        ModelLayerRegistry.registerModelLayer(CosmeticsLayer.CAPE_LAYER, CosmeticsLayer::capeModel);
        ModelLayerRegistry.registerModelLayer(CosmeticsLayer.WINGS_LAYER, CosmeticsLayer::wingsModel);
        ModelLayerRegistry.registerModelLayer(CosmeticsLayer.HAT_LAYER, CosmeticsLayer::hatModel);
        ModelLayerRegistry.registerModelLayer(CosmeticsLayer.HORNS_LAYER, CosmeticsLayer::hornsModel);

        LivingEntityRenderLayerRegistrationCallback.EVENT.register((entityType, renderer, helper, context) -> {
            if (renderer instanceof AvatarRenderer<?> avatar) {
                helper.register(new CosmeticsLayer(avatar, context.getModelSet()));
            }
        });

        ScreenEvents.AFTER_INIT.register((minecraft, screen, width, height) -> {
            if (screen instanceof TitleScreen || screen instanceof PauseScreen) {
                int y = screen instanceof TitleScreen ? Math.min(height - 44, height / 4 + 158) : height - 44;
                Screens.getWidgets(screen).add(Button.builder(
                    Component.literal("Valtic Cosmetics"),
                    button -> minecraft.gui.setScreen(new CosmeticsWardrobeScreen(screen))
                ).bounds(width / 2 - 100, Math.max(8, y), 200, 20).build());
            }
        });
    }
}
