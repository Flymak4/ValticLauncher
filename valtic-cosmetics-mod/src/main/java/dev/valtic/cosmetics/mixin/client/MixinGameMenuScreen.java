package dev.valtic.cosmetics.mixin.client;

import dev.valtic.cosmetics.client.gui.CosmeticsWardrobeScreen;
import net.minecraft.client.gui.screen.GameMenuScreen;
import net.minecraft.client.gui.screen.Screen;
import net.minecraft.client.gui.widget.ButtonWidget;
import net.minecraft.text.Text;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(GameMenuScreen.class)
public abstract class MixinGameMenuScreen extends Screen {

    protected MixinGameMenuScreen() { super(Text.empty()); }

    @Inject(method = "initWidgets", require = 0, at = @At("TAIL"))
    private void valtic_addCosmeticsButton(CallbackInfo ci) {
        int bx = this.width  - 108;
        int by = this.height - 48;
        addDrawableChild(ButtonWidget.builder(
                Text.literal("✿ Cosmetics"),
                btn -> this.client.setScreen(new CosmeticsWardrobeScreen(this))
        ).dimensions(bx, by, 104, 20).build());
    }
}
