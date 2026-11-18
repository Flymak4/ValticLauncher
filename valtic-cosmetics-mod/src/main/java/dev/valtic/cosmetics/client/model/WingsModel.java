package dev.valtic.cosmetics.client.model;

import net.minecraft.client.model.*;
import net.minecraft.client.render.entity.model.EntityModel;
import net.minecraft.client.render.entity.state.PlayerEntityRenderState;

/**
 * Wings model — 3-segment bat-style wing with articulated bones.
 *
 * Bone hierarchy (each side mirrored):
 *   root
 *   └─ shoulderL / shoulderR   (connects to player back)
 *      └─ upperArmL / upperArmR
 *         └─ foreArmL / foreArmR
 *            └─ membraneL / membraneR   (flat quad, full spread)
 *
 * UV layout: 128×64 texture sheet.
 *   Shoulder segment : u=0,  v=0,  size 6×6×4
 *   Upper arm        : u=22, v=0,  size 4×14×3
 *   Forearm          : u=36, v=0,  size 3×12×2
 *   Membrane tip     : u=48, v=0,  size 16×18×1
 */
public class WingsModel extends EntityModel<PlayerEntityRenderState> {

    private final ModelPart shoulderL, shoulderR;
    private final ModelPart upperArmL, upperArmR;
    private final ModelPart foreArmL,  foreArmR;
    private final ModelPart membraneL, membraneR;

    private float flapAngle = 0f;
    private boolean moving  = false;

    public WingsModel(ModelPart root) {
        super(root);
        shoulderL  = root.getChild("shoulderL");
        shoulderR  = root.getChild("shoulderR");
        upperArmL  = shoulderL.getChild("upperArmL");
        upperArmR  = shoulderR.getChild("upperArmR");
        foreArmL   = upperArmL.getChild("foreArmL");
        foreArmR   = upperArmR.getChild("foreArmR");
        membraneL  = foreArmL.getChild("membraneL");
        membraneR  = foreArmR.getChild("membraneR");
    }

    public static TexturedModelData getTexturedModelData() {
        ModelData data = new ModelData();
        ModelPartData root = data.getRoot();

        // ── Left shoulder ─────────────────────────────────────────────────────
        ModelPartData sL = root.addChild("shoulderL",
                ModelPartBuilder.create()
                        .uv(0, 0).cuboid(-3f, -3f, 0f, 6, 6, 4),
                ModelTransform.pivot(-4f, -8f, 2f));  // upper back, left

        ModelPartData uL = sL.addChild("upperArmL",
                ModelPartBuilder.create()
                        .uv(22, 0).cuboid(-2f, 0f, -1.5f, 4, 14, 3),
                ModelTransform.pivot(-2f, -2f, 2f));

        ModelPartData fL = uL.addChild("foreArmL",
                ModelPartBuilder.create()
                        .uv(36, 0).cuboid(-1.5f, 0f, -1f, 3, 12, 2),
                ModelTransform.pivot(-1f, 13f, 0f));

        fL.addChild("membraneL",
                ModelPartBuilder.create()
                        .uv(48, 0).cuboid(-1f, 0f, 0f, 16, 18, 1),
                ModelTransform.pivot(0f, 10f, 0f));

        // ── Right shoulder (mirrored) ─────────────────────────────────────────
        ModelPartData sR = root.addChild("shoulderR",
                ModelPartBuilder.create().mirrored()
                        .uv(0, 0).cuboid(-3f, -3f, 0f, 6, 6, 4),
                ModelTransform.pivot(4f, -8f, 2f));

        ModelPartData uR = sR.addChild("upperArmR",
                ModelPartBuilder.create().mirrored()
                        .uv(22, 0).cuboid(-2f, 0f, -1.5f, 4, 14, 3),
                ModelTransform.pivot(2f, -2f, 2f));

        ModelPartData fR = uR.addChild("foreArmR",
                ModelPartBuilder.create().mirrored()
                        .uv(36, 0).cuboid(-1.5f, 0f, -1f, 3, 12, 2),
                ModelTransform.pivot(1f, 13f, 0f));

        fR.addChild("membraneR",
                ModelPartBuilder.create().mirrored()
                        .uv(48, 0).cuboid(-15f, 0f, 0f, 16, 18, 1),
                ModelTransform.pivot(0f, 10f, 0f));

        return TexturedModelData.of(data, 128, 64);
    }

    public void setFlapAngle(float angle) { this.flapAngle = angle; }
    public void setMoving(boolean moving) { this.moving = moving; }

    @Override
    public void setAngles(PlayerEntityRenderState state) {
        // Shoulder: spread outward, flap up/down
        float spread = moving ? 0.6f : 0.3f;

        shoulderL.roll  = -spread - flapAngle;
        shoulderL.yaw   = -0.1f;
        shoulderL.pitch =  0.05f;

        shoulderR.roll  =  spread + flapAngle;
        shoulderR.yaw   =  0.1f;
        shoulderR.pitch =  0.05f;

        // Upper arm droops further out
        upperArmL.roll  = -(spread * 0.6f) - flapAngle * 0.5f;
        upperArmL.pitch =  0.1f;
        upperArmR.roll  =  (spread * 0.6f) + flapAngle * 0.5f;
        upperArmR.pitch =  0.1f;

        // Forearm: folds slightly during downstroke
        float fold = moving ? flapAngle * 0.4f : 0.1f;
        foreArmL.pitch = fold;
        foreArmR.pitch = fold;

        // Membrane tip: flair on upstroke
        membraneL.roll  = flapAngle * 0.3f;
        membraneR.roll  = -flapAngle * 0.3f;
    }
}
