//! Dedicated Minecraft-style Berk / Book of Dragons SMP page.
use super::*;
use crate::glass_ui as ui;
use horizon_theme::glass::ink;

const GOLD: u32 = 0xe5bb78;
const CREAM: u32 = 0xf4e7cf;
const SEAM: u32 = 0x5f827e;

impl Launcher {
    pub(super) fn valtic_smp_page(&mut self, cx: &mut Context<Self>) -> Div {
        let installed = self.instances.list.iter()
            .find(|i| horizon_launcher::valtic_smp::is_smp(i)).cloned();
        let running = self.client.is_some() || self.starting_client.is_some();
        let installing = self.importing.location_busy();
        let status = match self.valtic_status.as_ref() {
            Some(s) if s.online => format!("ONLINE  ·  {} DRAGON RIDERS", s.players),
            Some(_) => "SERVER OFFLINE".into(),
            None => "CHECKING SERVER STATUS".into(),
        };
        let header = div().flex().items_center().justify_between().flex_wrap().gap(px(12.))
            .child(div().flex().items_center().gap(px(12.))
                .child(gpui::svg().path("valtic/dragon-mark.svg").w(px(42.)).h(px(42.)))
                .child(div().flex().flex_col().gap(px(4.))
                    .child(ui::text(24., FontWeight::BOLD, ink::FULL)
                        .text_color(ui::color(CREAM)).child("VALTICSMP SERVER"))
                    .child(ui::text(12., FontWeight::MEDIUM, ink::FULL)
                        .text_color(ui::color(GOLD))
                        .child("OFFICIAL BOOK OF DRAGONS EXPERIENCE"))))
            .child(div().px(px(13.)).py(px(8.)).rounded(px(5.))
                .border_1().border_color(ui::color(SEAM)).bg(ui::color(0x173c3d))
                .child(ui::text(12., FontWeight::SEMIBOLD, ink::FULL)
                    .text_color(ui::color(GOLD)).child(status)));
        let landscape = div().relative().w_full().h(px(285.))
            .overflow_hidden().rounded(px(9.))
            .border_1().border_color(ui::color(SEAM)).bg(ui::color(0x122e37))
            .child(gpui::svg().path("valtic/berk-islands.svg").w_full().h(px(285.)))
            .child(div().absolute().top_0().left_0().size_full()
                .bg(gpui::linear_gradient(0.,
                    gpui::linear_color_stop(ui::color(0x07151c).opacity(0.74), 0.),
                    gpui::linear_color_stop(ui::color(0x07151c).opacity(0.03), 0.80))))
            .child(div().absolute().left(px(30.)).top(px(26.))
                .flex().flex_col().gap(px(8.))
                .child(ui::text(12., FontWeight::BOLD, ink::FULL)
                    .text_color(ui::color(GOLD)).child("THE ARCHIPELAGO OF BERK"))
                .child(ui::text(40., FontWeight::BOLD, ink::FULL)
                    .font_family(theme::DISPLAY_FONT).text_color(ui::color(CREAM))
                    .child("VALTIC SMP"))
                .child(ui::text(16., FontWeight::MEDIUM, ink::FULL)
                    .text_color(ui::color(0xc2d9d3))
                    .child("Train your dragon. Explore the open seas.")));
        let stats = div().flex().flex_wrap().gap(px(10.))
            .child(Self::smp_stat("MINECRAFT", "1.21.1"))
            .child(Self::smp_stat("MOD LOADER", "NEOFORGE 21.1.266"))
            .child(Self::smp_stat("DRAGON PACK", "30 MODS"))
            .child(Self::smp_stat("VISUALS", "4 SHADERS + 3 PACKS"));
        let action = div().id("valtic-smp-action")
            .flex().items_center().justify_center().gap(px(10.))
            .w_full().h(px(57.)).rounded(px(6.))
            .border_1().border_color(ui::color(0xf4d4a0))
            .bg(ui::color(if running || installing { 0x536d61 } else { 0x4b8873 }))
            .child(gpui::svg().path("valtic/dragon-mark.svg").w(px(25.)).h(px(25.)))
            .child(ui::text(16., FontWeight::BOLD, ink::FULL)
                .text_color(ui::color(0xfff2d9))
                .child(if installing { "INSTALLING DRAGON PACK..." }
                    else if running { "GAME RUNNING" }
                    else if installed.is_some() { "PLAY VALTIC SMP" }
                    else { "INSTALL RECOMMENDED MODPACK" }));
        let action = if running || installing { action }
            else if let Some(instance) = installed.clone() {
                action.cursor_pointer().hover(|d| d.bg(ui::color(0x5eaa89)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.select_instance(&instance);
                        this.quick_play("--server", "valticsmp.ultramc.co".into());
                        cx.notify();
                    }))
            } else {
                action.cursor_pointer().hover(|d| d.bg(ui::color(0x5eaa89)))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.install_valtic_smp(cx);
                        cx.notify();
                    }))
            };
        let mut panel = div().flex().flex_col().gap(px(14.))
            .p(px(21.)).rounded(px(9.))
            .border_1().border_color(ui::color(SEAM))
            .bg(ui::color(0x112b31))
            .child(ui::text(18., FontWeight::BOLD, ink::FULL)
                .text_color(ui::color(GOLD))
                .child(if installed.is_some() { "YOUR DRAGON RIDER PROFILE" }
                    else { "BEGIN YOUR DRAGON RIDER JOURNEY" }))
            .child(ui::text(13., FontWeight::NORMAL, ink::FULL)
                .text_color(ui::color(0xc2d9d3))
                .child(if installed.is_some() {
                    "Your exclusive Valtic SMP profile is ready. Play it from this page."
                } else {
                    "Install Book of Dragons, Viking ships, voice chat, shaders, resource packs and your recommended settings in one click."
                }))
            .child(action)
            .child(ui::text(12., FontWeight::MEDIUM, ink::FULL)
                .text_color(ui::color(0x9fc0b9))
                .child("valticsmp.ultramc.co  ·  Minecraft Java account required"));
        if let Some(Err(err)) = self.importing.note.as_ref() {
            panel = panel.child(ui::text(12., FontWeight::MEDIUM, ink::FULL)
                .text_color(ui::color(0xff9f98)).child(err.clone()));
        }
        div().w_full().flex().flex_col().gap(px(18.)).p(px(8.))
            .child(header).child(landscape).child(stats).child(panel)
            .child(ui::text(12., FontWeight::NORMAL, ink::FULL)
                .text_color(ui::color(0xb8cfc7))
                .child("This dedicated SMP instance stays separate from your other Minecraft profiles."))
    }

    fn smp_stat(label: &'static str, value: &'static str) -> Div {
        div().flex().flex_1().flex_col().gap(px(5.))
            .min_w(px(165.)).px(px(15.)).py(px(14.))
            .rounded(px(5.)).border_1().border_color(ui::color(0x486e68))
            .bg(ui::color(0x173a3d))
            .child(ui::text(10., FontWeight::BOLD, ink::FULL)
                .text_color(ui::color(GOLD)).child(label))
            .child(ui::text(13., FontWeight::SEMIBOLD, ink::FULL)
                .text_color(ui::color(CREAM)).child(value))
    }
}
