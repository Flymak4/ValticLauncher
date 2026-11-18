package dev.valtic.cosmetics.client.render;

import dev.valtic.cosmetics.client.model.*;
import dev.valtic.cosmetics.data.CosmeticEntry;
import dev.valtic.cosmetics.data.CosmeticType;
import dev.valtic.cosmetics.data.PlayerCosmeticState;
import net.minecraft.client.render.OverlayTexture;
import net.minecraft.client.render.RenderLayer;
import net.minecraft.client.render.VertexConsumer;
import net.minecraft.client.render.VertexConsumerProvider;
import net.minecraft.client.render.entity.EntityRendererProvider;
import net.minecraft.client.render.entity.LivingEntityRenderer;
import net.minecraft.client.render.entity.feature.FeatureRenderer;
import net.minecraft.client.render.entity.model.EntityModel;
import net.minecraft.client.render.entity.state.PlayerEntityRenderState;
import net.minecraft.client.util.math.MatrixStack;
import net.minecraft.entity.player.PlayerEntity;
import net.minecraft.util.math.RotationAxis;

public class CosmeticRenderer<T extends PlayerEntity, M extends EntityModel<PlayerEntityRenderState>>
        extends FeatureRenderer<PlayerEntityRenderState, M> {

    private final WingsModel    wingsModel;
    private final CapeModel     capeModel;
    private final HatModel      hatModel;
    private final HornsModel    hornsModel;

    public CosmeticRenderer(LivingEntityRenderer<T, M> renderer,
                            EntityRendererProvider.Context context) {
        super(renderer);
        wingsModel = new WingsModel(context.getPart(CosmeticModelLayers.WINGS));
        capeModel  = new CapeModel(context.getPart(CosmeticModelLayers.CAPE));
        hatModel   = new HatModel(context.getPart(CosmeticModelLayers.HAT));
        hornsModel = new HornsModel(context.getPart(CosmeticModelLayers.HORNS));
    }

    @Override
    public void render(MatrixStack matrices,
                       VertexConsumerProvider vertexConsumers,
                       int light,
                       PlayerEntityRenderState state,
                       float limbAngle,
                       float limbDistance) {

        PlayerCosmeticState cosState = PlayerCosmeticState.get();

        // ── Wings ─────────────────────────────────────────────────────────────
        CosmeticEntry wings = cosState.getEquippedEntry(CosmeticType.WINGS);
        if (wings != null) {
            renderWings(matrices, vertexConsumers, light, state, wings, limbAngle, limbDistance);
        }

        // ── Cape ──────────────────────────────────────────────────────────────
        CosmeticEntry cape = cosState.getEquippedEntry(CosmeticType.CAPE);
        if (cape != null) {
            renderCape(matrices, vertexConsumers, light, state, cape, limbAngle, limbDistance);
        }

        // ── Hat ───────────────────────────────────────────────────────────────
        CosmeticEntry hat = cosState.getEquippedEntry(CosmeticType.HAT);
        if (hat != null) {
            renderHat(matrices, vertexConsumers, light, state, hat);
        }

        // ── Horns ─────────────────────────────────────────────────────────────
        CosmeticEntry horns = cosState.getEquippedEntry(CosmeticType.HORNS);
        if (horns != null) {
            renderHorns(matrices, vertexConsumers, light, state, horns);
        }
    }

    // ── Wings rendering ───────────────────────────────────────────────────────

    private void renderWings(MatrixStack matrices,
                              VertexConsumerProvider vcp,
                              int light,
                              PlayerEntityRenderState state,
                              CosmeticEntry entry,
                              float limbAngle,
                              float limbDistance) {
        matrices.push();

        // Wings sit at upper back — shift from body pivot to between shoulder blades
        matrices.translate(0.0f, -0.4f, 0.12f);

        // Flap animation: gentle oscillation, faster when moving
        float speed    = limbDistance > 0.1f ? 2.5f : 0.6f;
        float flapAmp  = limbDistance > 0.1f ? 0.35f : 0.12f;
        float tick     = (System.currentTimeMillis() % 100000L) / 1000f;
        float flapAngle = (float)(Math.sin(tick * speed) * flapAmp);

        wingsModel.setFlapAngle(flapAngle);
        wingsModel.setMoving(limbDistance > 0.1f);

        VertexConsumer vc = vcp.getBuffer(RenderLayer.getEntityCutoutNoCull(entry.texture()));
        wingsModel.render(matrices, vc, light, OverlayTexture.DEFAULT_UV, 0xFFFFFFFF);

        matrices.pop();
    }

    // ── Cape rendering ────────────────────────────────────────────────────────

    private void renderCape(MatrixStack matrices,
                             VertexConsumerProvider vcp,
                             int light,
                             PlayerEntityRenderState state,
                             CosmeticEntry entry,
                             float limbAngle,
                             float limbDistance) {
        matrices.push();

        // Cape attaches at the neck/shoulders — same y as body top, slightly behind
        matrices.translate(0.0f, -0.5f, 0.125f);

        // Cape sway when walking
        float sway = limbDistance > 0.05f
                ? (float)(Math.sin(limbAngle * 0.5f) * 0.1f)
                : 0f;
        capeModel.setSwayAngle(sway);
        capeModel.setMoving(limbDistance > 0.05f);

        VertexConsumer vc = vcp.getBuffer(RenderLayer.getEntityCutoutNoCull(entry.texture()));
        capeModel.render(matrices, vc, light, OverlayTexture.DEFAULT_UV, 0xFFFFFFFF);

        matrices.pop();
    }

    // ── Hat rendering ─────────────────────────────────────────────────────────

    private void renderHat(MatrixStack matrices,
                            VertexConsumerProvider vcp,
                            int light,
                            PlayerEntityRenderState state,
                            CosmeticEntry entry) {
        matrices.push();

        // Hat sits on top of the head. In Minecraft's model space the head pivot
        // is at y=0 when the entity renderer calls render(). The head is 8px tall
        // (0.5 blocks). We translate up to sit above it.
        matrices.translate(0.0f, -0.5f, 0.0f);  // move to head top

        // Apply head yaw/pitch so the hat follows head rotation
        matrices.multiply(RotationAxis.POSITIVE_Y.rotationDegrees(-state.headYaw));
        matrices.multiply(RotationAxis.POSITIVE_X.rotationDegrees(state.pitch));

        VertexConsumer vc = vcp.getBuffer(RenderLayer.getEntityCutoutNoCull(entry.texture()));
        hatModel.render(matrices, vc, light, OverlayTexture.DEFAULT_UV, 0xFFFFFFFF);

        matrices.pop();
    }

    // ── Horns rendering ───────────────────────────────────────────────────────

    private void renderHorns(MatrixStack matrices,
                              VertexConsumerProvider vcp,
                              int light,
                              PlayerEntityRenderState state,
                              CosmeticEntry entry) {
        matrices.push();

        // Horns attach to sides of the head — same pivot as hat
        matrices.translate(0.0f, -0.3f, 0.0f);
        matrices.multiply(RotationAxis.POSITIVE_Y.rotationDegrees(-state.headYaw));
        matrices.multiply(RotationAxis.POSITIVE_X.rotationDegrees(state.pitch));

        VertexConsumer vc = vcp.getBuffer(RenderLayer.getEntityCutoutNoCull(entry.texture()));
        hornsModel.render(matrices, vc, light, OverlayTexture.DEFAULT_UV, 0xFFFFFFFF);

        matrices.pop();
    }
}
