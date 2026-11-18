package dev.valtic.cosmetics.client.model;

import net.minecraft.client.model.*;
import net.minecraft.client.render.entity.model.EntityModel;
import net.minecraft.client.render.entity.state.PlayerEntityRenderState;

/**
 * Hat model — a layered crown/headwear sitting flush on top of the 8×8×8 head.
 *
 * Bone structure:
 *   root
 *   └─ headMount  (zero-size pivot, tracks head rotation)
 *      ├─ brim    (optional wide brim ring, 12×2×12)
 *      ├─ band    (hat band / crown base, 10×4×10)
 *      └─ crown   (upper dome or peak, 8×8×8)
 *         └─ tip  (decorative peak, 4×6×4)
 *
 * UV layout: 64×32
 *
 * Note: the headMount pivot sits at y=-4 (the top of the player head in entity model space),
 * so all child parts are positioned relative to the head top.
 */
public class HatModel extends EntityModel<PlayerEntityRenderState> {

    private final ModelPart headMount;
    private final ModelPart brim;
    private final ModelPart band;
    private final ModelPart crown;
    private final ModelPart tip;

    public HatModel(ModelPart root) {
        super(root);
        headMount = root.getChild("headMount");
        brim      = headMount.getChild("brim");
        band      = headMount.getChild("band");
        crown     = headMount.getChild("crown");
        tip       = crown.getChild("tip");
    }

    public static TexturedModelData getTexturedModelData() {
        ModelData data = new ModelData();
        ModelPartData root = data.getRoot();

        // headMount: sits at the very top of the 8px tall head
        // In Minecraft's player model the head pivot is at y=0 from the renderer's perspective;
        // the head itself occupies y = -8..0 (top to pivot). We offset by -8 to ride the top.
        ModelPartData mount = root.addChild("headMount",
                ModelPartBuilder.create(),
                ModelTransform.pivot(0f, -8f, 0f));

        // Wide brim — centred, extends 2px each side beyond head (head=8px wide, brim=12)
        mount.addChild("brim",
                ModelPartBuilder.create().uv(0, 0)
                        .cuboid(-6f, -2f, -6f, 12, 2, 12),
                ModelTransform.pivot(0f, 0f, 0f));

        // Hat band / lower crown ring
        mount.addChild("band",
                ModelPartBuilder.create().uv(0, 14)
                        .cuboid(-5f, -6f, -5f, 10, 4, 10),
                ModelTransform.pivot(0f, 0f, 0f));

        // Crown body — same footprint as head, rises above
        ModelPartData crownPart = mount.addChild("crown",
                ModelPartBuilder.create().uv(0, 28)  // second UV row (wraps, 64×32 sheet)
                        .cuboid(-4f, -13f, -4f, 8, 7, 8),
                ModelTransform.pivot(0f, 0f, 0f));

        // Pointed tip
        crownPart.addChild("tip",
                ModelPartBuilder.create().uv(40, 0)
                        .cuboid(-2f, -8f, -2f, 4, 8, 4),
                ModelTransform.pivot(0f, -13f, 0f));

        return TexturedModelData.of(data, 64, 64);
    }

    @Override
    public void setAngles(PlayerEntityRenderState state) {
        // No animated parts — hat follows head via the renderer pivot transforms
    }
}
