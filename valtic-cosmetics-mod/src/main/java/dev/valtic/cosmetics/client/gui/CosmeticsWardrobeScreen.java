package dev.valtic.cosmetics.client.gui;

import dev.valtic.cosmetics.data.CosmeticEntry;
import dev.valtic.cosmetics.data.CosmeticRegistry;
import dev.valtic.cosmetics.data.CosmeticType;
import dev.valtic.cosmetics.data.PlayerCosmeticState;
import net.minecraft.client.gui.DrawContext;
import net.minecraft.client.gui.screen.Screen;
import net.minecraft.client.gui.screen.ingame.InventoryScreen;
import net.minecraft.client.gui.widget.ButtonWidget;
import net.minecraft.text.Text;
import net.minecraft.util.Identifier;

import java.util.List;

/**
 * The Valtic Cosmetics wardrobe — a cherry-blossom themed screen with:
 *  - Left panel: rotating 3-D character preview
 *  - Right panel: category tabs + cosmetic grid
 *  - Bottom bar: equip / unequip buttons + equipped indicator
 */
public class CosmeticsWardrobeScreen extends Screen {

    // ── Colour palette ────────────────────────────────────────────────────────
    private static final int COL_BG          = 0xFF1A0E12;  // deep cherry-dark
    private static final int COL_PANEL       = 0xCC2D1420;  // semi-transparent panel
    private static final int COL_ACCENT      = 0xFFE87DA8;  // sakura pink
    private static final int COL_ACCENT2     = 0xFFFFBDD4;  // lighter pink
    private static final int COL_TAB_ACTIVE  = 0xFFE87DA8;
    private static final int COL_TAB_IDLE    = 0xFF6B3350;
    private static final int COL_ITEM_SEL    = 0x88E87DA8;
    private static final int COL_TEXT_MAIN   = 0xFFFFEBF3;
    private static final int COL_TEXT_SUB    = 0xFFD4A0B8;
    private static final int COL_EQUIPPED    = 0xFF78FF98;

    // ── Layout ────────────────────────────────────────────────────────────────
    private static final int PREVIEW_W = 160;
    private static final int PANEL_PAD = 10;
    private static final int TAB_H     = 22;
    private static final int ITEM_SIZE = 54;
    private static final int ITEM_COLS = 3;
    private static final int ITEM_PAD  = 6;

    // ── State ─────────────────────────────────────────────────────────────────
    private CosmeticType selectedTab       = CosmeticType.WINGS;
    private CosmeticEntry selectedEntry    = null;
    private float previewRotation          = 0f;
    private long  lastFrameMs             = 0L;
    private int   scrollOffset            = 0;

    private ButtonWidget equipButton;
    private ButtonWidget unequipButton;

    private final Screen parent;

    public CosmeticsWardrobeScreen(Screen parent) {
        super(Text.literal("Valtic Cosmetics"));
        this.parent = parent;
    }

    @Override
    protected void init() {
        int cx = width / 2;
        int cy = height / 2;
        int screenH = Math.min(height - 20, 280);
        int screenW = Math.min(width - 20, 500);
        int left    = cx - screenW / 2;
        int top     = cy - screenH / 2;

        int gridLeft = left + PREVIEW_W + PANEL_PAD * 2;
        int gridTop  = top + PANEL_PAD + TAB_H + 4;
        int gridW    = screenW - PREVIEW_W - PANEL_PAD * 3;
        int btnY     = top + screenH - 32;

        // Equip button
        equipButton = ButtonWidget.builder(Text.literal("Equip"), btn -> onEquip())
                .dimensions(gridLeft, btnY, gridW / 2 - 4, 20)
                .build();
        addDrawableChild(equipButton);

        // Unequip button
        unequipButton = ButtonWidget.builder(Text.literal("Unequip"), btn -> onUnequip())
                .dimensions(gridLeft + gridW / 2 + 4, btnY, gridW / 2 - 4, 20)
                .build();
        addDrawableChild(unequipButton);

        // Close button
        addDrawableChild(ButtonWidget.builder(Text.literal("✕"), btn -> close())
                .dimensions(left + screenW - 18, top + 4, 14, 14)
                .build());

        refreshButtons();
        lastFrameMs = System.currentTimeMillis();
    }

    @Override
    public void render(DrawContext ctx, int mouseX, int mouseY, float delta) {
        // Animate preview rotation
        long now = System.currentTimeMillis();
        float dt = (now - lastFrameMs) / 1000f;
        previewRotation += dt * 30f;   // 30°/sec
        if (previewRotation > 360f) previewRotation -= 360f;
        lastFrameMs = now;

        int cx = width / 2;
        int cy = height / 2;
        int screenH = Math.min(height - 20, 280);
        int screenW = Math.min(width - 20, 500);
        int left = cx - screenW / 2;
        int top  = cy - screenH / 2;

        // ── Background overlay ────────────────────────────────────────────────
        ctx.fill(0, 0, width, height, 0x99000000);

        // ── Main panel ────────────────────────────────────────────────────────
        drawRoundedPanel(ctx, left, top, screenW, screenH, COL_PANEL);

        // ── Title bar ─────────────────────────────────────────────────────────
        ctx.fill(left, top, left + screenW, top + 24, COL_BG);
        ctx.drawCenteredTextWithShadow(textRenderer,
                Text.literal("✿  Valtic Cosmetics  ✿"),
                cx, top + 7, COL_ACCENT);

        // ── Character preview panel ───────────────────────────────────────────
        int previewLeft = left + PANEL_PAD;
        int previewTop  = top + 28;
        int previewH    = screenH - 36;
        drawRoundedPanel(ctx, previewLeft, previewTop, PREVIEW_W, previewH, 0xAA100608);

        // 3D skin preview — centred in preview panel
        if (client != null && client.player != null) {
            int skinX = previewLeft + PREVIEW_W / 2;
            int skinY = previewTop  + (int)(previewH * 0.72f);
            int skinSize = (int)(previewH * 0.55f);
            InventoryScreen.drawEntity(ctx, skinX, skinY, skinSize,
                    skinX - mouseX, (skinY - skinSize / 2) - mouseY,
                    client.player);
        }

        ctx.drawCenteredTextWithShadow(textRenderer,
                Text.literal("Your Character"),
                previewLeft + PREVIEW_W / 2, previewTop + 6, COL_TEXT_SUB);

        // Show currently equipped items in preview panel
        drawEquippedSummary(ctx, previewLeft + 4, previewTop + 20, PREVIEW_W - 8);

        // ── Grid panel ────────────────────────────────────────────────────────
        int gridLeft = left + PREVIEW_W + PANEL_PAD * 2;
        int gridTop  = top + 28;
        int gridW    = screenW - PREVIEW_W - PANEL_PAD * 3;
        int gridH    = screenH - 36;
        drawRoundedPanel(ctx, gridLeft, gridTop, gridW, gridH, 0xAA100608);

        // Category tabs
        drawTabs(ctx, gridLeft, gridTop + 2, gridW);

        // Cosmetic items grid
        int itemsTop = gridTop + TAB_H + 8;
        int itemsH   = gridH - TAB_H - 50;
        drawItemGrid(ctx, gridLeft + ITEM_PAD, itemsTop, gridW - ITEM_PAD * 2, itemsH, mouseX, mouseY);

        // Selected item info
        if (selectedEntry != null) {
            int infoY = itemsTop + itemsH + 4;
            ctx.drawTextWithShadow(textRenderer,
                    Text.literal(selectedEntry.displayName()),
                    gridLeft + 6, infoY, COL_ACCENT);
            ctx.drawTextWithShadow(textRenderer,
                    Text.literal(selectedEntry.description()),
                    gridLeft + 6, infoY + 11, COL_TEXT_SUB);
        }

        super.render(ctx, mouseX, mouseY, delta);
    }

    // ── Tab bar ───────────────────────────────────────────────────────────────

    private void drawTabs(DrawContext ctx, int x, int y, int w) {
        CosmeticType[] types = CosmeticType.values();
        int tw = (w - ITEM_PAD * 2) / types.length;
        for (int i = 0; i < types.length; i++) {
            CosmeticType t = types[i];
            int tx = x + ITEM_PAD + i * tw;
            int col = (t == selectedTab) ? COL_TAB_ACTIVE : COL_TAB_IDLE;
            ctx.fill(tx, y, tx + tw - 2, y + TAB_H, col | 0xFF000000);
            ctx.drawCenteredTextWithShadow(textRenderer,
                    Text.literal(t.getDisplayName()),
                    tx + tw / 2 - 1, y + 7, COL_TEXT_MAIN);
        }
    }

    // ── Item grid ─────────────────────────────────────────────────────────────

    private void drawItemGrid(DrawContext ctx, int x, int y, int w, int h,
                               int mouseX, int mouseY) {
        List<CosmeticEntry> items = CosmeticRegistry.getByType(selectedTab);
        PlayerCosmeticState state = PlayerCosmeticState.get();

        int col = 0, row = 0;
        int cellW = (w - ITEM_PAD * (ITEM_COLS - 1)) / ITEM_COLS;
        int cellH = ITEM_SIZE;

        // Scissor clip so items don't overflow into button bar
        ctx.enableScissor(x, y, x + w, y + h);

        for (CosmeticEntry entry : items) {
            int ix = x + col * (cellW + ITEM_PAD);
            int iy = y + row * (cellH + ITEM_PAD) - scrollOffset;

            if (iy + cellH < y) { advanceGrid(col, row); col++; if (col >= ITEM_COLS) { col = 0; row++; } continue; }
            if (iy > y + h) break;

            // Cell background
            boolean isSel      = entry.equals(selectedEntry);
            boolean isEquipped = state.isEquipped(entry);
            int cellBg = isSel ? COL_ITEM_SEL : 0x55FFFFFF;
            ctx.fill(ix, iy, ix + cellW, iy + cellH, cellBg);

            // Equipped indicator
            if (isEquipped) {
                ctx.fill(ix, iy, ix + cellW, iy + 2, COL_EQUIPPED);
                ctx.drawTextWithShadow(textRenderer,
                        Text.literal("✓"), ix + 2, iy + 3, COL_EQUIPPED);
            }

            // Cosmetic name (centred, two lines if needed)
            ctx.drawCenteredTextWithShadow(textRenderer,
                    Text.literal(entry.displayName()),
                    ix + cellW / 2, iy + cellH - 18, isSel ? COL_ACCENT : COL_TEXT_MAIN);

            // Hover highlight
            if (mouseX >= ix && mouseX < ix + cellW && mouseY >= iy && mouseY < iy + cellH) {
                ctx.fill(ix, iy, ix + cellW, iy + cellH, 0x33FFFFFF);
            }

            col++;
            if (col >= ITEM_COLS) { col = 0; row++; }
        }

        ctx.disableScissor();
    }

    private void advanceGrid(int col, int row) { /* helper stub */ }

    // ── Equipped summary ──────────────────────────────────────────────────────

    private void drawEquippedSummary(DrawContext ctx, int x, int y, int w) {
        PlayerCosmeticState state = PlayerCosmeticState.get();
        int lineH = 11;
        ctx.drawTextWithShadow(textRenderer, Text.literal("Equipped:"), x, y, COL_ACCENT2);
        int ly = y + lineH + 2;
        for (CosmeticType type : CosmeticType.values()) {
            CosmeticEntry e = state.getEquippedEntry(type);
            String label = type.getDisplayName() + ": " + (e != null ? e.displayName() : "None");
            ctx.drawTextWithShadow(textRenderer, Text.literal(label), x, ly, e != null ? COL_TEXT_MAIN : COL_TEXT_SUB);
            ly += lineH;
        }
    }

    // ── Rounded panel helper ──────────────────────────────────────────────────

    private void drawRoundedPanel(DrawContext ctx, int x, int y, int w, int h, int color) {
        ctx.fill(x + 2, y,     x + w - 2, y + h,     color);
        ctx.fill(x,     y + 2, x + 2,     y + h - 2, color);
        ctx.fill(x + w - 2, y + 2, x + w, y + h - 2, color);
        // border
        ctx.fill(x + 2, y,         x + w - 2, y + 1,     COL_ACCENT);
        ctx.fill(x + 2, y + h - 1, x + w - 2, y + h,     COL_ACCENT);
        ctx.fill(x,     y + 2,     x + 1,     y + h - 2, COL_ACCENT);
        ctx.fill(x + w - 1, y + 2, x + w, y + h - 2,     COL_ACCENT);
    }

    // ── Mouse click ───────────────────────────────────────────────────────────

    @Override
    public boolean mouseClicked(double mouseX, double mouseY, int button) {
        if (button == 0) {
            // Check tab clicks
            int cx = width / 2;
            int cy = height / 2;
            int screenW = Math.min(width - 20, 500);
            int screenH = Math.min(height - 20, 280);
            int left = cx - screenW / 2;
            int top  = cy - screenH / 2;
            int gridLeft = left + PREVIEW_W + PANEL_PAD * 2;
            int gridTop  = top + 28;
            int gridW    = screenW - PREVIEW_W - PANEL_PAD * 3;

            CosmeticType[] types = CosmeticType.values();
            int tw = (gridW - ITEM_PAD * 2) / types.length;
            for (int i = 0; i < types.length; i++) {
                int tx = gridLeft + ITEM_PAD + i * tw;
                int ty = gridTop + 2;
                if (mouseX >= tx && mouseX < tx + tw - 2 && mouseY >= ty && mouseY < ty + TAB_H) {
                    selectedTab   = types[i];
                    selectedEntry = null;
                    scrollOffset  = 0;
                    refreshButtons();
                    return true;
                }
            }

            // Check item grid clicks
            List<CosmeticEntry> items = CosmeticRegistry.getByType(selectedTab);
            int itemsTop = gridTop + TAB_H + 8;
            int cellW = (gridW - ITEM_PAD * 2 - ITEM_PAD * (ITEM_COLS - 1)) / ITEM_COLS;
            int col = 0, row = 0;
            for (CosmeticEntry entry : items) {
                int ix = gridLeft + ITEM_PAD + col * (cellW + ITEM_PAD);
                int iy = itemsTop + row * (ITEM_SIZE + ITEM_PAD) - scrollOffset;
                if (mouseX >= ix && mouseX < ix + cellW && mouseY >= iy && mouseY < iy + ITEM_SIZE) {
                    selectedEntry = entry;
                    refreshButtons();
                    return true;
                }
                col++;
                if (col >= ITEM_COLS) { col = 0; row++; }
            }
        }
        return super.mouseClicked(mouseX, mouseY, button);
    }

    @Override
    public boolean mouseScrolled(double mouseX, double mouseY, double horizontalAmount, double verticalAmount) {
        scrollOffset = Math.max(0, scrollOffset - (int)(verticalAmount * (ITEM_SIZE + ITEM_PAD)));
        return true;
    }

    // ── Button actions ────────────────────────────────────────────────────────

    private void onEquip() {
        if (selectedEntry == null) return;
        PlayerCosmeticState.get().equip(selectedEntry);
        refreshButtons();
    }

    private void onUnequip() {
        if (selectedEntry == null) return;
        PlayerCosmeticState.get().unequip(selectedEntry.type());
        refreshButtons();
    }

    private void refreshButtons() {
        boolean hasSelection = selectedEntry != null;
        boolean isEquipped   = hasSelection && PlayerCosmeticState.get().isEquipped(selectedEntry);
        if (equipButton   != null) equipButton.active   = hasSelection && !isEquipped;
        if (unequipButton != null) unequipButton.active = hasSelection && isEquipped;
    }

    @Override
    public void close() {
        if (client != null) client.setScreen(parent);
    }

    @Override
    public boolean shouldPause() { return false; }
}
