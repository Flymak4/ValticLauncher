package dev.valtic.cosmetics.client.render;

import dev.valtic.cosmetics.data.CosmeticEntry;
import dev.valtic.cosmetics.data.CosmeticType;
import dev.valtic.cosmetics.data.PlayerCosmeticState;

import com.mojang.blaze3d.vertex.PoseStack;
import net.minecraft.client.Minecraft;
import net.minecraft.client.model.geom.ModelLayerLocation;
import net.minecraft.client.model.geom.ModelPart;
import net.minecraft.client.model.geom.PartPose;
import net.minecraft.client.model.geom.builders.CubeListBuilder;
import net.minecraft.client.model.geom.builders.LayerDefinition;
import net.minecraft.client.model.geom.builders.MeshDefinition;
import net.minecraft.client.model.geom.builders.PartDefinition;
import net.minecraft.client.model.player.PlayerModel;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.RenderLayerParent;
import net.minecraft.client.renderer.entity.layers.RenderLayer;
import net.minecraft.client.renderer.entity.state.AvatarRenderState;
import net.minecraft.client.renderer.rendertype.RenderTypes;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.client.model.geom.EntityModelSet;
import net.minecraft.resources.Identifier;

/** Four baked 3D models rendered as player attachments. Textures are bundled inside the mod JAR. */
public final class CosmeticsLayer extends RenderLayer<AvatarRenderState, PlayerModel> {
    private static final String MOD = "valtic_cosmetics";
    public static final ModelLayerLocation CAPE_LAYER = layer("cape");
    public static final ModelLayerLocation WINGS_LAYER = layer("wings");
    public static final ModelLayerLocation HAT_LAYER = layer("hat");
    public static final ModelLayerLocation HORNS_LAYER = layer("horns");

    private final ModelPart cape;
    private final ModelPart wings;
    private final ModelPart wingLeft;
    private final ModelPart wingRight;
    private final ModelPart hat;
    private final ModelPart horns;

    public CosmeticsLayer(RenderLayerParent<AvatarRenderState, PlayerModel> parent, EntityModelSet models) {
        super(parent);
        this.cape = models.bakeLayer(CAPE_LAYER).getChild("ornament");
        this.wings = models.bakeLayer(WINGS_LAYER).getChild("ornament");
        this.wingLeft = wings.getChild("left");
        this.wingRight = wings.getChild("right");
        this.hat = models.bakeLayer(HAT_LAYER).getChild("ornament");
        this.horns = models.bakeLayer(HORNS_LAYER).getChild("ornament");
    }

    private static ModelLayerLocation layer(String name) {
        return new ModelLayerLocation(Identifier.fromNamespaceAndPath(MOD, name), "main");
    }

    @Override
    public void submit(PoseStack pose, SubmitNodeCollector collector, int light, AvatarRenderState state,
                       float yRot, float xRot) {
        // Preview cosmetics are local-only. Other players should not have cosmetics forced on them.
        if (Minecraft.getInstance().player == null || state.id != Minecraft.getInstance().player.getId()
            || state.isSpectator || state.isInvisible) return;

        PlayerCosmeticState selection = PlayerCosmeticState.get();
        CosmeticEntry capeEntry = selection.getEquippedEntry(CosmeticType.CAPE);
        CosmeticEntry wingsEntry = selection.getEquippedEntry(CosmeticType.WINGS);
        CosmeticEntry hatEntry = selection.getEquippedEntry(CosmeticType.HAT);
        CosmeticEntry hornsEntry = selection.getEquippedEntry(CosmeticType.HORNS);
        if (capeEntry != null || wingsEntry != null) {
            pose.pushPose();
            getParentModel().body.translateAndRotate(pose);
            if (capeEntry != null) {
                cape.xRot = 0.025f + (float)Math.sin(System.nanoTime() * 0.0000000015) * 0.045f;
                collector.submitModelPart(cape, pose, RenderTypes.entityCutout(capeEntry.texture()), light,
                    OverlayTexture.NO_OVERLAY, null, 0xFFFFFFFF);
            }
            if (wingsEntry != null) {
                // Gentle motion; no particles or gameplay effects.
                float flap = (float)Math.sin(System.nanoTime() * 0.0000000022) * 0.07f;
                wingLeft.yRot = 0.22f + flap;
                wingRight.yRot = -0.22f - flap;
                wingLeft.zRot = 0.08f + flap * 0.45f;
                wingRight.zRot = -0.08f - flap * 0.45f;
                collector.submitModelPart(wings, pose, RenderTypes.entityCutout(wingsEntry.texture()), light,
                    OverlayTexture.NO_OVERLAY, null, 0xFFFFFFFF);
            }
            pose.popPose();
        }
        if (hatEntry != null || hornsEntry != null) {
            pose.pushPose();
            getParentModel().head.translateAndRotate(pose);
            if (hatEntry != null) {
                collector.submitModelPart(hat, pose, RenderTypes.entityCutout(hatEntry.texture()), light,
                    OverlayTexture.NO_OVERLAY, null, 0xFFFFFFFF);
            }
            if (hornsEntry != null) {
                collector.submitModelPart(horns, pose, RenderTypes.entityCutout(hornsEntry.texture()), light,
                    OverlayTexture.NO_OVERLAY, null, 0xFFFFFFFF);
            }
            pose.popPose();
        }
    }

    private static PartDefinition part(MeshDefinition mesh) {
        return mesh.getRoot().addOrReplaceChild("ornament", CubeListBuilder.create(), PartPose.ZERO);
    }

    /** Layered cloak with fitted shoulders and a shaped hem. Back of the torso is +Z. */
    public static LayerDefinition capeModel() {
        MeshDefinition mesh = new MeshDefinition();
        PartDefinition p = part(mesh);
        p.addOrReplaceChild("collar", CubeListBuilder.create().texOffs(0, 0)
            .addBox(-4.25f, -1.0f, 2.2f, 8.5f, 1.35f, 1.15f), PartPose.ZERO);
        p.addOrReplaceChild("shoulders", CubeListBuilder.create().texOffs(0, 8)
            .addBox(-5.0f, 0.0f, 2.5f, 10.0f, 2.1f, 1.1f), PartPose.ZERO);
        p.addOrReplaceChild("cloth", CubeListBuilder.create().texOffs(0, 16)
            .addBox(-5.0f, 1.1f, 2.75f, 10.0f, 10.2f, 0.7f), PartPose.ZERO);
        p.addOrReplaceChild("hem", CubeListBuilder.create().texOffs(0, 41)
            .addBox(-5.25f, 10.9f, 2.8f, 10.5f, 1.25f, 0.95f), PartPose.ZERO);
        p.addOrReplaceChild("stitch_left", CubeListBuilder.create().texOffs(30, 16)
            .addBox(-4.65f, 1.7f, 3.48f, 0.45f, 8.9f, 0.15f), PartPose.ZERO);
        p.addOrReplaceChild("stitch_right", CubeListBuilder.create().texOffs(30, 16)
            .addBox(4.2f, 1.7f, 3.48f, 0.45f, 8.9f, 0.15f), PartPose.ZERO);
        return LayerDefinition.create(mesh, 64, 64);
    }

    /** Original stylised layered feather geometry, symmetrical around the player's spine. */
    public static LayerDefinition wingsModel() {
        MeshDefinition mesh = new MeshDefinition();
        PartDefinition p = part(mesh);
        addWing(p, "left", 1f);
        addWing(p, "right", -1f);
        return LayerDefinition.create(mesh, 128, 128);
    }

    private static void addWing(PartDefinition body, String name, float side) {
        PartDefinition wing = body.addOrReplaceChild(name, CubeListBuilder.create(),
            PartPose.offset(side * 2.4f, -0.6f, 2.7f));
        // Structural scapula, tapered from its attachment point to the outer edge.
        for (int i = 0; i < 4; i++) {
            float start = side * (i * 2.15f);
            float x = side > 0 ? start : start - 2.35f;
            wing.addOrReplaceChild("bone" + i, CubeListBuilder.create()
                .texOffs(0, 0).addBox(x, -1.3f - i * 0.62f, 0.0f,
                    2.45f, 1.65f, 1.2f), PartPose.ZERO);
        }
        // Each feather is its own layered cuboid, instead of one rectangular wing.
        // Long feathers hang toward the edges: the fan has a recognizable curved silhouette.
        for (int i = 0; i < 9; i++) {
            float extent = 2.0f + i * 1.43f;
            float rootY = -1.5f - Math.min(i, 5) * 0.65f + Math.max(0, i - 5) * 0.26f;
            float featherLength = 4.3f + Math.min(i, 5) * 1.45f - Math.max(0, i - 5) * 1.1f;
            float x = side > 0 ? extent : -extent - 1.46f;
            wing.addOrReplaceChild("feather" + i, CubeListBuilder.create()
                .texOffs(0, 22 + (i % 3) * 17)
                .addBox(x, rootY, 0.25f + (i % 3) * 0.21f,
                    1.46f, featherLength, 0.82f), PartPose.ZERO);
            if (i < 6) {
                wing.addOrReplaceChild("highlight" + i, CubeListBuilder.create()
                    .texOffs(42, 24 + (i % 3) * 17)
                    .addBox(x + 0.48f, rootY + 0.7f, 1.04f + (i % 3) * 0.21f,
                        0.5f, featherLength * 0.65f, 0.22f), PartPose.ZERO);
            }
        }
        wing.addOrReplaceChild("shoulder_plate", CubeListBuilder.create().texOffs(40, 0)
            .addBox(side > 0 ? 0f : -3f, -1.2f, 0.95f, 3f, 2.7f, 1.35f), PartPose.ZERO);
    }

    /** A sculpted petal crown that moves with the actual head bone. */
    public static LayerDefinition hatModel() {
        MeshDefinition mesh = new MeshDefinition();
        PartDefinition p = part(mesh);
        p.addOrReplaceChild("front_band", CubeListBuilder.create().texOffs(0, 0)
            .addBox(-4.8f, -9.0f, -5.25f, 9.6f, 1.25f, 1.1f), PartPose.ZERO);
        p.addOrReplaceChild("back_band", CubeListBuilder.create().texOffs(0, 0)
            .addBox(-4.8f, -9.0f, 4.2f, 9.6f, 1.25f, 1.1f), PartPose.ZERO);
        p.addOrReplaceChild("left_band", CubeListBuilder.create().texOffs(0, 0)
            .addBox(-5.25f, -9.0f, -4.8f, 1.1f, 1.25f, 9.6f), PartPose.ZERO);
        p.addOrReplaceChild("right_band", CubeListBuilder.create().texOffs(0, 0)
            .addBox(4.2f, -9.0f, -4.8f, 1.1f, 1.25f, 9.6f), PartPose.ZERO);
        for (int i = 0; i < 5; i++) {
            float x = -4.1f + i * 2.05f;
            float peak = (i % 2 == 0) ? 2.7f : 3.4f;
            p.addOrReplaceChild("petal_front_" + i, CubeListBuilder.create().texOffs(0, 15)
                .addBox(x, -8.8f - peak, -5.5f, 1.6f, peak, 0.9f), PartPose.ZERO);
            p.addOrReplaceChild("bud_" + i, CubeListBuilder.create().texOffs(27, 15)
                .addBox(x + 0.25f, -9.1f - peak, -5.65f, 1.1f, 1.1f, 1.1f), PartPose.ZERO);
        }
        p.addOrReplaceChild("center_gem", CubeListBuilder.create().texOffs(42, 0)
            .addBox(-1.0f, -10.4f, -5.9f, 2.0f, 2.0f, 0.8f), PartPose.ZERO);
        return LayerDefinition.create(mesh, 64, 64);
    }

    /** Pair of tapered amethyst-style horns with stepped tips. */
    public static LayerDefinition hornsModel() {
        MeshDefinition mesh = new MeshDefinition();
        PartDefinition p = part(mesh);
        for (int side : new int[]{-1, 1}) {
            float x = side * 3.0f;
            p.addOrReplaceChild("horn_base_" + side, CubeListBuilder.create().texOffs(0, 0)
                .addBox(x - 1.2f, -10.0f, -1.0f, 2.4f, 3.0f, 2.4f), PartPose.ZERO);
            p.addOrReplaceChild("horn_middle_" + side, CubeListBuilder.create().texOffs(12, 0)
                .addBox(x - 0.8f + side * 0.5f, -12.7f, -0.8f, 1.6f, 3.0f, 1.8f), PartPose.ZERO);
            p.addOrReplaceChild("horn_tip_" + side, CubeListBuilder.create().texOffs(22, 0)
                .addBox(x - 0.45f + side * 1.0f, -14.7f, -0.5f, 0.9f, 2.2f, 1.0f), PartPose.ZERO);
        }
        return LayerDefinition.create(mesh, 64, 64);
    }
}
