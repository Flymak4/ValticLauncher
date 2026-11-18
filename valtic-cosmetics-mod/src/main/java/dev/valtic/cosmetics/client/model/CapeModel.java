package dev.valtic.cosmetics.client.model;

import net.minecraft.client.model.*;
import net.minecraft.client.render.entity.model.EntityModel;
import net.minecraft.client.render.entity.state.PlayerEntityRenderState;

/**
 * Cape model — attaches at both shoulders, hangs down the back.
 *
 * Bone structure:
 *   root
 *   ├─ collarL  (left shoulder strap, 4×3×1)
 *   ├─ collarR  (right shoulder strap)
 *   └─ body     (main cape panel, 16×24×1)
 *      └─ hem   (bottom hem, slightly forward-angled for movement)
 *
 * UV layout: 64×32
 */
public class CapeModel extends EntityModel<PlayerEntityRenderState> {

    private final ModelPart collarL, collarR;
    private final ModelPart body;
    private final ModelPart hem;

    private float swayAngle = 0f;
    private boolean moving  = false;

    public CapeModel(ModelPart root) {
        super(root);
        collarL = root.getChild("collarL");
        collarR = root.getChild("collarR");
        body    = root.getChild("body");
        hem     = body.getChild("hem");
    }

    public static TexturedModelData getTexturedModelData() {
        ModelData data = new ModelData();
        ModelPartData root = data.getRoot();

        // Left collar
        root.addChild("collarL",
                ModelPartBuilder.create().uv(0, 0)
                        .cuboid(-2f, 0f, 0f, 4, 3, 1),
                ModelTransform.pivot(-6f, -11f, 2f));

        // Right collar
        root.addChild("collarR",
                ModelPartBuilder.create().uv(0, 0)
                        .cuboid(-2f, 0f, 0f, 4, 3, 1),
                ModelTransform.pivot(2f, -11f, 2f));

        // Main cape body — 16 wide, 24 tall, 1 deep, hangs behind
        ModelPartData bodyPart = root.addChild("body",
                ModelPartBuilder.create().uv(0, 4)
                        .cuboid(-8f, 0f, 0f, 16, 24, 1),
                ModelTransform.pivot(0f, -10f, 2.5f));

        // Hem (bottom 4 rows, offset so it flows)
        bodyPart.addChild("hem",
                ModelPartBuilder.create().uv(0, 28)
                        .cuboid(-8f, 0f, 0f, 16, 4, 1),
                ModelTransform.pivot(0f, 24f, 0f));

        return TexturedModelData.of(data, 64, 32);
    }

    public void setSwayAngle(float angle) { this.swayAngle = angle; }
    public void setMoving(boolean moving)  { this.moving = moving; }

    @Override
    public void setAngles(PlayerEntityRenderState state) {
        // Body flares backward slightly when moving, sways left-right with limb swing
        body.pitch  = moving ? -0.08f : -0.04f;
        body.roll   = swayAngle;

        // Hem flows further on movement
        hem.pitch   = moving ? 0.25f : 0.1f;
        hem.roll    = swayAngle * 0.5f;
    }
}
