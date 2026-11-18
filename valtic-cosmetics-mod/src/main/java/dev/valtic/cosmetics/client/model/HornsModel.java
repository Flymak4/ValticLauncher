package dev.valtic.cosmetics.client.model;

import net.minecraft.client.model.*;
import net.minecraft.client.render.entity.model.EntityModel;
import net.minecraft.client.render.entity.state.PlayerEntityRenderState;

/**
 * Horns model — two articulated horns at the temples.
 *
 * Each horn has 3 segments tapering upward and curving outward,
 * giving a realistic curved-horn silhouette.
 *
 * Bone hierarchy:
 *   root
 *   ├─ hornBaseL  (left temple attachment, 3×3×3)
 *   │  ├─ hornMidL   (mid segment, 2×5×2)
 *   │  └─ hornTipL   (tip segment, 1×4×1)
 *   └─ hornBaseR  (right, mirrored)
 *      ├─ hornMidR
 *      └─ hornTipR
 *
 * UV layout: 32×32
 */
public class HornsModel extends EntityModel<PlayerEntityRenderState> {

    private final ModelPart hornBaseL, hornBaseR;
    private final ModelPart hornMidL,  hornMidR;
    private final ModelPart hornTipL,  hornTipR;

    public HornsModel(ModelPart root) {
        super(root);
        hornBaseL = root.getChild("hornBaseL");
        hornBaseR = root.getChild("hornBaseR");
        hornMidL  = hornBaseL.getChild("hornMidL");
        hornMidR  = hornBaseR.getChild("hornMidR");
        hornTipL  = hornMidL.getChild("hornTipL");
        hornTipR  = hornMidR.getChild("hornTipR");
    }

    public static TexturedModelData getTexturedModelData() {
        ModelData data = new ModelData();
        ModelPartData root = data.getRoot();

        // ── Left horn ─────────────────────────────────────────────────────────
        // Base sits at left temple: x=-4 (outside the 8px head), y=-7 (near top of head)
        ModelPartData bL = root.addChild("hornBaseL",
                ModelPartBuilder.create().uv(0, 0)
                        .cuboid(-1.5f, -3f, -1.5f, 3, 3, 3),
                ModelTransform.of(-4f, -7f, 0f,
                        0f, 0f, (float)Math.toRadians(-20f)));  // slight outward lean

        ModelPartData mL = bL.addChild("hornMidL",
                ModelPartBuilder.create().uv(0, 6)
                        .cuboid(-1f, -5f, -1f, 2, 5, 2),
                ModelTransform.of(0f, -3f, 0f,
                        0f, 0f, (float)Math.toRadians(-15f)));

        mL.addChild("hornTipL",
                ModelPartBuilder.create().uv(0, 16)
                        .cuboid(-0.5f, -4f, -0.5f, 1, 4, 1),
                ModelTransform.of(0f, -5f, 0f,
                        0f, 0f, (float)Math.toRadians(-10f)));

        // ── Right horn (mirrored x) ───────────────────────────────────────────
        ModelPartData bR = root.addChild("hornBaseR",
                ModelPartBuilder.create().mirrored().uv(0, 0)
                        .cuboid(-1.5f, -3f, -1.5f, 3, 3, 3),
                ModelTransform.of(4f, -7f, 0f,
                        0f, 0f, (float)Math.toRadians(20f)));

        ModelPartData mR = bR.addChild("hornMidR",
                ModelPartBuilder.create().mirrored().uv(0, 6)
                        .cuboid(-1f, -5f, -1f, 2, 5, 2),
                ModelTransform.of(0f, -3f, 0f,
                        0f, 0f, (float)Math.toRadians(15f)));

        mR.addChild("hornTipR",
                ModelPartBuilder.create().mirrored().uv(0, 16)
                        .cuboid(-0.5f, -4f, -0.5f, 1, 4, 1),
                ModelTransform.of(0f, -5f, 0f,
                        0f, 0f, (float)Math.toRadians(10f)));

        return TexturedModelData.of(data, 32, 32);
    }

    @Override
    public void setAngles(PlayerEntityRenderState state) {
        // Static — head rotation is handled by the renderer pivot in CosmeticRenderer
    }
}
