package dev.valtic.cosmetics.mixin.client;

import dev.valtic.cosmetics.client.gui.CosmeticsWardrobeScreen;
import net.minecraft.client.gui.screen.Screen;
import net.minecraft.client.gui.screen.TitleScreen;
import net.minecraft.client.gui.widget.ButtonWidget;
import net.minecraft.text.Text;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(TitleScreen.class)
public abstract class MixinTitleScreen extends Screen {

    protected MixinTitleScreen() { super(Text.empty()); }

    @Inject(method = "init", at = @At("TAIL"))
    private void valtic_addCosmeticsButton(CallbackInfo ci) {
        // Place button in the lower-right corner so it doesn't disrupt normal layout
        int bx = this.width  - 104;
        int by = this.height - 26;
        addDrawableChild(ButtonWidget.builder(
                Text.literal("✿ Cosmetics"),
                btn -> this.client.setScreen(new CosmeticsWardrobeScreen(this))
        ).dimensions(bx, by, 100, 20).build());
    }
}
