//! Dedicated Valtic SMP launchpad. All art is baked into a native colour image.
use super::*;
use crate::glass_ui as ui;
use horizon_theme::glass::ink;
use std::sync::Arc;

const GOLD: u32 = 0xe9bc82;
const CREAM: u32 = 0xf2eee5;
const MUTED: u32 = 0xa6b8b5;
const PANEL: u32 = 0x182a2d;
const EDGE: u32 = 0x304449;

fn copy(size: f32, weight: FontWeight, colour: u32) -> Div {
    ui::text(size, weight, ink::FULL).text_color(ui::color(colour))
}

// GPUI's svg element is an icon mask rather than a full-colour landscape.
// Bake the scene once per UI thread instead, with stepped cliffs and ocean.
fn berk_scene() -> Arc<gpui::RenderImage> {
    thread_local! {
        static SCENE: Arc<gpui::RenderImage> = {
            const W: usize = 480;
            const H: usize = 240;
            let mut pixels = vec![[0, 0, 0, 255]; W * H];
            for y in 0..H {
                for x in 0..W {
                    let c = if y < 139 {
                        let t = y as f32 / 139.;
                        [(29. + 79. * t) as u8, (55. + 71. * t) as u8, (68. + 51. * t) as u8, 255]
                    } else {
                        let t = (y - 139) as f32 / 101.;
                        [(37. - 16. * t) as u8, (82. - 31. * t) as u8, (92. - 27. * t) as u8, 255]
                    };
                    pixels[y * W + x] = c;
                    let dx = x as i32 - 369;
                    let dy = y as i32 - 57;
                    if dx * dx + dy * dy < 26 * 26 {
                        pixels[y * W + x] = [237, 193, 131, 255];
                    }
                    if y > 146 && (y % 9 == 0) && (x / 18 + y / 9) % 4 == 0 {
                        pixels[y * W + x] = [70, 110, 115, 255];
                    }
                }
            }
            // Rasterise polygon silhouettes in world order. The 2px grid gives
            // the art a Minecraft-like blocky texture without relying on SVG loading.
            let mut poly = |points: &[(i32, i32)], colour: [u8; 4]| {
                for y in (0..H).step_by(2) {
                    for x in (0..W).step_by(2) {
                        let (px, py) = (x as i32, y as i32);
                        let mut inside = false;
                        let mut j = points.len() - 1;
                        for i in 0..points.len() {
                            let (xi, yi) = points[i];
                            let (xj, yj) = points[j];
                            if (yi > py) != (yj > py)
                                && (px as f32) < (xj - xi) as f32 * (py - yi) as f32 / (yj - yi) as f32 + xi as f32
                            { inside = !inside; }
                            j = i;
                        }
                        if inside {
                            for yy in y..(y + 2).min(H) {
                                for xx in x..(x + 2).min(W) {
                                    pixels[yy * W + xx] = colour;
                                }
                            }
                        }
                    }
                }
            };
            poly(&[(0,151),(0,112),(39,112),(39,92),(62,92),(62,67),(87,67),(87,50),(105,50),(105,77),(128,77),(128,101),(158,101),(158,150)], [66,91,94,255]);
            poly(&[(174,151),(174,112),(192,112),(192,89),(215,89),(215,78),(244,78),(244,100),(266,100),(266,119),(292,119),(292,151)], [75,103,102,255]);
            poly(&[(320,155),(320,127),(342,127),(342,105),(365,105),(365,87),(386,87),(386,111),(405,111),(405,155)], [76,105,103,255]);
            poly(&[(0,240),(0,165),(28,165),(28,143),(53,143),(53,112),(82,112),(82,91),(114,91),(114,69),(143,69),(143,91),(168,91),(168,121),(192,121),(192,157),(218,157),(218,190),(196,190),(196,215),(164,215),(164,240)], [32,56,57,255]);
            poly(&[(0,165),(28,165),(28,143),(53,143),(53,112),(82,112),(82,91),(114,91),(114,69),(143,69),(143,91),(168,91),(168,121),(192,121),(192,139),(169,139),(169,117),(139,117),(139,100),(110,100),(110,123),(83,123),(83,153),(54,153),(54,175),(0,175)], [74,108,80,255]);
            poly(&[(115,70),(143,70),(143,91),(156,91),(156,98),(128,98),(128,88),(115,88)], [154,165,148,255]);
            poly(&[(54,175),(83,153),(110,123),(128,123),(128,195),(109,215),(109,240),(63,240)], [43,72,66,255]);
            poly(&[(480,240),(480,98),(457,98),(457,75),(430,75),(430,48),(409,48),(409,74),(386,74),(386,102),(363,102),(363,132),(337,132),(337,161),(311,161),(311,189),(336,189),(336,220),(361,220),(361,240)], [37,56,58,255]);
            poly(&[(480,98),(457,98),(457,75),(430,75),(430,48),(409,48),(409,74),(386,74),(386,102),(363,102),(363,132),(337,132),(337,147),(361,147),(361,123),(390,123),(390,96),(416,96),(416,83),(442,83),(442,109),(480,109)], [86,116,83,255]);
            poly(&[(409,48),(430,48),(430,75),(440,75),(440,84),(421,84),(421,72),(405,72)], [185,187,165,255]);
            poly(&[(390,123),(416,96),(442,109),(442,184),(421,184),(421,240),(387,240)], [54,79,70,255]);
            // Waterfalls and the little Viking longship below the cliffs.
            poly(&[(153,119),(160,119),(160,186),(170,186),(170,190),(148,190),(148,183),(153,183)], [108,158,158,255]);
            poly(&[(378,124),(384,124),(384,187),(392,187),(392,193),(373,193),(373,187),(378,187)], [113,165,165,255]);
            poly(&[(238,205),(251,211),(292,211),(306,201),(310,203),(300,219),(251,219)], [29,35,35,255]);
            poly(&[(274,175),(277,175),(277,211),(274,211)], [52,42,33,255]);
            poly(&[(249,181),(290,181),(288,204),(250,204)], [208,174,121,255]);
            poly(&[(264,181),(273,181),(273,204),(264,204)], [137,78,57,255]);
            // A distant dragon in flight, silhouette rather than an empty icon.
            poly(&[(222,55),(235,58),(249,71),(258,70),(269,61),(284,55),(280,64),(270,76),(258,79),(266,86),(255,82),(249,78),(237,75),(231,64)], [25,44,50,255]);
            poly(&[(253,72),(260,70),(267,71),(270,68),(270,74),(263,77),(254,77)], [25,44,50,255]);
            // Spruce trees along the nearer ridgelines.
            for (x, y) in [(62,112),(96,91),(127,69),(172,121),(351,132),(395,74),(451,75)] {
                poly(&[(x,y-18),(x-5,y-10),(x-2,y-10),(x-8,y-3),(x-2,y-3),(x-2,y+2),(x+2,y+2),(x+2,y-3),(x+8,y-3),(x+3,y-10),(x+5,y-10)], [29,61,51,255]);
            }
            art::bake(&pixels, W, H, 2., 1.)
        };
    }
    SCENE.with(Arc::clone)
}

impl Launcher {
    pub(super) fn valtic_smp_page(&mut self, cx: &mut Context<Self>) -> Div {
        let installed = self.instances.list.iter()
            .find(|i| horizon_launcher::valtic_smp::is_smp(i)).cloned();
        let running = self.client.is_some() || self.starting_client.is_some();
        let installing = self.importing.location_busy();
        let signed_in = self.accounts.selected().is_some();
        let narrow = self.window_width < 860.;
        let (status, status_colour) = match self.valtic_status.as_ref() {
            Some(s) if s.online => (format!("Online  ·  {} players", s.players), 0x91d5a6),
            Some(_) => ("Server offline".into(), 0xe89b85),
            None if self.valtic_status_failed => ("Status unavailable".into(), MUTED),
            None => ("Checking server".into(), MUTED),
        };
        let header = div().flex().items_center().justify_between().flex_wrap().gap(px(10.))
            .child(copy(13., FontWeight::SEMIBOLD, MUTED).child("VALTIC  /  OFFICIAL SERVER"))
            .child(div().flex().flex_none().items_center().gap(px(8.))
                .rounded(px(12.)).px(px(12.)).py(px(7.)).bg(ui::color(PANEL))
                .child(div().size(px(8.)).rounded_full().bg(ui::color(status_colour)))
                .child(copy(12., FontWeight::SEMIBOLD, status_colour).child(status)));

        let label = if installing { "Installing modpack…" }
            else if running { "Game running" }
            else if installed.is_some() && !signed_in { "Sign in to play" }
            else if installed.is_some() { "Play Valtic SMP" }
            else { "Install modpack" };
        let action = div().id("valtic-smp-action")
            .flex().flex_none().items_center().justify_center()
            .w_full().max_w(px(300.)).h(px(49.)).rounded(px(10.))
            .bg(ui::color(if running || installing { 0x52635d } else { GOLD }))
            .child(copy(15., FontWeight::BOLD, if running || installing { CREAM } else { 0x222c29 })
                .flex_none().text_center().child(label));
        let action = if running || installing { action }
            else if installed.is_some() && !signed_in {
                action.cursor_pointer().hover(|d| d.bg(ui::color(0xf4ce9c)))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.sign_in_to_play();
                        cx.notify();
                    }))
            } else if let Some(instance) = installed.clone() {
                action.cursor_pointer().hover(|d| d.bg(ui::color(0xf4ce9c)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.select_instance(&instance);
                        this.quick_play("--server", "valticsmp.ultramc.co".into());
                        cx.notify();
                    }))
            } else {
                action.cursor_pointer().hover(|d| d.bg(ui::color(0xf4ce9c)))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.install_valtic_smp(cx);
                        cx.notify();
                    }))
            };
        let mut introduction = div().flex().flex_col().flex_1().min_w_0()
            .justify_center().gap(px(11.)).p(px(if narrow { 21. } else { 29. }))
            .child(copy(12., FontWeight::SEMIBOLD, GOLD).child("THE ARCHIPELAGO OF BERK"))
            .child(copy(39., FontWeight::BOLD, CREAM).line_height(px(48.)).child("Valtic SMP"))
            .child(copy(16., FontWeight::MEDIUM, CREAM).child("Your dragon. Your adventure."))
            .child(copy(13., FontWeight::NORMAL, MUTED)
                .child("Explore towering islands, tame dragons and sail the open seas with the Book of Dragons modpack."))
            .child(div().mt(px(9.)).child(action))
            .child(copy(12., FontWeight::NORMAL, MUTED).child(if installed.is_some() {
                "Modpack installed  ·  Minecraft Java account required"
            } else {
                "One-click setup  ·  Recommended for Valtic SMP"
            }));
        if let Some(Err(err)) = self.importing.note.as_ref() {
            introduction = introduction.child(copy(12., FontWeight::MEDIUM, 0xffa49b).child(err.clone()));
        }
        let artwork = div().relative().flex_none().overflow_hidden()
            .bg(ui::color(0x30494d))
            .when(narrow, |d| d.w_full().h(px(190.)))
            .when(!narrow, |d| d.w(relative(0.48)).min_h(px(320.)))
            .child(gpui::img(berk_scene()).absolute().top_0().left_0().size_full()
                .object_fit(gpui::ObjectFit::Cover))
            .child(div().absolute().bottom(px(13.)).right(px(13.))
                .rounded(px(8.)).bg(ui::color(0x13252a).opacity(0.85)).px(px(10.)).py(px(6.))
                .child(copy(10., FontWeight::SEMIBOLD, CREAM).child("BERK  /  CONCEPT ART")));
        let hero = div().flex().w_full().overflow_hidden().rounded(px(18.))
            .border_1().border_color(ui::color(EDGE)).bg(ui::color(0x18282a));
        let hero = if narrow {
            hero.flex_col().child(artwork).child(introduction)
        } else {
            hero.min_h(px(320.)).child(introduction).child(artwork)
        };
        let pack = div().flex().flex_col().gap(px(12.))
            .child(div().flex().items_center().justify_between().flex_wrap().gap(px(8.))
                .child(copy(19., FontWeight::SEMIBOLD, CREAM).child("The modpack"))
                .child(copy(12., FontWeight::MEDIUM, MUTED).child("Curated for Valtic SMP")))
            .child(div().flex().flex_wrap().gap(px(10.))
                .child(Self::smp_stat("Minecraft", "1.21.1"))
                .child(Self::smp_stat("NeoForge", "21.1.266"))
                .child(Self::smp_stat("Gameplay", "30 mods"))
                .child(Self::smp_stat("Visuals", "4 shaders · 3 packs")))
            .child(copy(13., FontWeight::NORMAL, MUTED)
                .child("Includes Book of Dragons, Viking ships, voice chat, shaders, resource packs and recommended settings. Your other Minecraft profiles stay separate."));
        let server = div().flex().items_center().flex_wrap().justify_between().gap(px(8.))
            .rounded(px(12.)).bg(ui::color(PANEL)).px(px(16.)).py(px(12.))
            .child(div().flex().flex_col().gap(px(3.))
                .child(copy(13., FontWeight::SEMIBOLD, CREAM).child("Server address"))
                .child(copy(12., FontWeight::NORMAL, MUTED).child("valticsmp.ultramc.co")))
            .child(copy(12., FontWeight::MEDIUM, GOLD).child("Minecraft Java Edition"));
        div().w_full().flex().flex_col().gap(px(16.)).p(px(8.))
            .child(header).child(hero).child(pack).child(server)
    }

    fn smp_stat(label: &'static str, value: &'static str) -> Div {
        div().flex().flex_1().flex_col().gap(px(5.))
            .min_w(px(145.)).px(px(15.)).py(px(13.))
            .rounded(px(11.)).bg(ui::color(PANEL))
            .child(copy(12., FontWeight::MEDIUM, MUTED).child(label))
            .child(copy(16., FontWeight::SEMIBOLD, CREAM).child(value))
    }
}
