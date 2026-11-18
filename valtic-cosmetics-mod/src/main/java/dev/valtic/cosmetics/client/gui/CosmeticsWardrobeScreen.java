package dev.valtic.cosmetics.client.gui;

import dev.valtic.cosmetics.data.CosmeticEntry;
import dev.valtic.cosmetics.data.CosmeticRegistry;
import dev.valtic.cosmetics.data.CosmeticType;
import dev.valtic.cosmetics.data.PlayerCosmeticState;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.inventory.InventoryScreen;
import net.minecraft.network.chat.Component;

import java.util.List;

/** A 26.3-native wardrobe. The four variants per category are free to preview. */
public final class CosmeticsWardrobeScreen extends Screen {
    private static final int BG = 0xF3111020;
    private static final int PANEL = 0xF51C1429;
    private static final int SECTION = 0xA52C2139;
    private static final int BORDER = 0xFF75516F;
    private static final int WHITE = 0xFFF4E9F5;
    private static final int MUTED = 0xFFD1B8CB;
    private static final int PINK = 0xFFFF9BCE;
    private static final int GREEN = 0xFF9BE9B2;
    private final Screen previous;
    private CosmeticType tab = CosmeticType.WINGS;

    public CosmeticsWardrobeScreen(Screen previous) {
        super(Component.literal("Valtic Cosmetics"));
        this.previous = previous;
    }

    private int widthPanel() { return Math.min(width - 24, 638); }
    private int heightPanel() { return Math.min(height - 16, 316); }
    private int left() { return (width - widthPanel()) / 2; }
    private int top() { return (height - heightPanel()) / 2; }
    private boolean preview() { return widthPanel() >= 490 && minecraft != null && minecraft.player != null; }
    private int listX() { return left() + (preview() ? 232 : 16); }
    private int listWidth() { return widthPanel() - (listX() - left()) - 16; }

    @Override
    protected void init() {
        int x = listX(), right = left() + widthPanel() - 16, y = top();
        CosmeticType[] categories = CosmeticType.values();
        int tabW = Math.max(48, listWidth() / categories.length);
        for (int i = 0; i < categories.length; i++) {
            CosmeticType target = categories[i];
            addRenderableWidget(Button.builder(Component.literal(target.getDisplayName()), button -> {
                tab = target;
                rebuildWidgets();
            }).bounds(x + i * tabW, y + 47, tabW - 3, 20).build());
        }
        List<CosmeticEntry> entries = CosmeticRegistry.getByType(tab);
        for (int i = 0; i < entries.size(); i++) {
            CosmeticEntry entry = entries.get(i);
            addRenderableWidget(Button.builder(
                Component.literal(PlayerCosmeticState.get().isEquipped(entry) ? "Unequip" : "Equip"),
                button -> {
                    if (PlayerCosmeticState.get().isEquipped(entry)) {
                        PlayerCosmeticState.get().unequip(entry.type());
                    } else {
                        PlayerCosmeticState.get().equip(entry);
                    }
                    rebuildWidgets();
                }
            ).bounds(right - 83, y + 81 + i * 40, 83, 20).build());
        }
        addRenderableWidget(Button.builder(Component.literal("Done"), b -> onClose())
            .bounds(right - 82, y + heightPanel() - 29, 82, 20).build());
    }

    @Override
    public void extractBackground(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float partialTick) {
        graphics.fill(0, 0, width, height, BG);
        int x = left(), y = top(), w = widthPanel(), h = heightPanel();
        graphics.fill(x, y, x + w, y + h, PANEL);
        graphics.outline(x, y, w, h, BORDER);
        graphics.fill(x + 1, y + 1, x + w - 1, y + 4, 0xFFB85D8B);
        graphics.fill(x + 14, y + 41, x + w - 14, y + 42, BORDER);
        if (preview()) {
            graphics.fill(x + 14, y + 47, x + 220, y + h - 34, SECTION);
            graphics.outline(x + 14, y + 47, 206, h - 81, BORDER);
        }
        int listX = listX(), listW = listWidth();
        for (int i = 0; i < 4; i++) {
            int cy = y + 75 + i * 40;
            graphics.fill(listX, cy, listX + listW, cy + 32, SECTION);
            graphics.outline(listX, cy, listW, 32, BORDER);
        }
    }

    @Override
    public void extractRenderState(GuiGraphicsExtractor graphics, int mouseX, int mouseY, float partialTick) {
        super.extractRenderState(graphics, mouseX, mouseY, partialTick);
        int x = left(), y = top();
        graphics.text(font, "VALTIC  /  COSMETICS", x + 17, y + 10, WHITE);
        graphics.text(font, "4 categories  /  16 designs  /  preview build", x + 17, y + 25, MUTED);
        List<CosmeticEntry> entries = CosmeticRegistry.getByType(tab);
        int xList = listX();
        for (int i = 0; i < entries.size(); i++) {
            CosmeticEntry e = entries.get(i);
            boolean equipped = PlayerCosmeticState.get().isEquipped(e);
            int cy = y + 75 + i * 40;
            graphics.text(font, e.displayName(), xList + 9, cy + 6, equipped ? GREEN : WHITE);
            graphics.text(font, equipped ? "EQUIPPED" : "Not equipped", xList + 9, cy + 19, equipped ? GREEN : MUTED);
        }
        if (preview()) {
            int px = x + 24;
            graphics.text(font, "LIVE PLAYER PREVIEW", px + 4, y + 55, PINK);
            InventoryScreen.extractEntityInInventoryFollowsMouse(graphics,
                px + 5, y + 69, px + 179, y + 209,
                56, 0, mouseX, mouseY, minecraft.player);
            graphics.text(font, "Move mouse to rotate", px + 23, y + 222, MUTED);
            int ty = y + 237;
            for (CosmeticType type : CosmeticType.values()) {
                CosmeticEntry entry = PlayerCosmeticState.get().getEquippedEntry(type);
                graphics.text(font, type.getDisplayName() + ": " + (entry == null ? "None" : entry.displayName()),
                    px + 3, ty, entry == null ? MUTED : WHITE);
                ty += 12;
            }
        }
        graphics.text(font, "Preview only - purchase checks are not active.", x + 16,
            y + heightPanel() - 13, MUTED);
    }

    @Override
    public void onClose() { minecraft.gui.setScreen(previous); }
}
