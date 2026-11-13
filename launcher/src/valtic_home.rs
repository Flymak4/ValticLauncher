//! Valtic's home: a cinematic Minecraft launchpad, not a settings panel.
//! Scenes are read from the *selected version's* official game panoramas.
//! No generated or unrelated version artwork is used.
use super::*;
use crate::glass_ui::{self as ui};
use horizon_theme::glass::{ink, radius};
use std::sync::Arc;

impl Launcher {
    /// Every image is from the Minecraft installation selected by the player.
    /// Its six title panoramas are real in-game screenshots for that exact build.
    fn valtic_scene(&mut self, tag: &str, index: u8, size: f32) -> Option<Arc<gpui::RenderImage>> {
        if tag.is_empty() {
            return None;
        }
        let file = backend::resources(tag).join(format!(
            "assets/minecraft/textures/gui/title/background/panorama_{index}.png"
        ));
        self.icon_image(format!("valtic:panorama:{tag}:{index}"), size, false, || {
            std::fs::read(file).ok()
        })
    }

    /// Used on the version page too. The locally drawn grass block is a branded
    /// placeholder until Minecraft's genuine panorama has been prepared.
    pub(super) fn valtic_cover(&mut self, tag: &str, size: f32) -> AnyElement {
        let picture = self.valtic_scene(tag, 0, size);
        let fallback = if picture.is_none() {
            self.grass_mark(size * 0.58)
        } else {
            None
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(size))
            .rounded(px(radius::PANEL))
            .overflow_hidden()
            .bg(gpui::linear_gradient(
                130.,
                gpui::linear_color_stop(ui::color(0x263f3b), 0.),
                gpui::linear_color_stop(ui::color(0x15252b), 1.),
            ))
            .children(picture.map(|p| {
                gpui::img(p).size(px(size)).object_fit(gpui::ObjectFit::Cover)
            }))
            .children(fallback.map(|p| {
                gpui::img(p)
                    .size(px(size * 0.58))
                    .object_fit(gpui::ObjectFit::Contain)
            }))
            .into_any_element()
    }

    /// Valtic's own scenic launchpad. The native backend still owns Play,
    /// version switching, Microsoft login and instance editing.
    /// Screenshot-inspired Valtic homepage. The logo is a bundled Valtic
    /// SVG, while Play, the version picker, and links remain live native controls.
    /// An unobtrusive glass card for the actual Valtic SMP status.
    /// The favicon comes from the server's 64x64 PNG, not the mockup artwork.
    fn valtic_server_card(&mut self) -> Div {
        use std::hash::{Hash, Hasher};

        let snapshot = self.valtic_status.clone();
        let icon = snapshot.as_ref().and_then(|s| s.icon.clone());
        let picture = icon.and_then(|bytes| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut hasher);
            let key = format!("valtic:smp-icon:{:x}", hasher.finish());
            self.icon_image(key, 100., false, move || Some(bytes))
        });
        let thumbnail: AnyElement = match picture {
            Some(image) => gpui::img(image)
                .w(px(77.)).h(px(77.))
                .object_fit(gpui::ObjectFit::Contain)
                .into_any_element(),
            None => gpui::svg()
                .path("valtic/cherry-emblem.svg")
                .w(px(66.)).h(px(66.))
                .into_any_element(),
        };

        let state = match snapshot.as_ref().map(|s| s.online) {
            Some(true) => "Online",
            Some(false) => "Offline",
            None if self.valtic_status_failed => "Unavailable",
            None => "Checking...",
        };
        let colour = match snapshot.as_ref().map(|s| s.online) {
            Some(true) => 0x60ea95,
            Some(false) => 0xff858e,
            None => 0xffc7de,
        };
        let players = match snapshot.as_ref() {
            Some(s) if s.online && s.capacity > 0 => {
                format!("{} / {} players online", s.players, s.capacity)
            }
            Some(s) if s.online => format!("{} players online", s.players),
            Some(_) => "0 players online".to_string(),
            None => "Player count unavailable".to_string(),
        };
        let motd = snapshot.as_ref()
            .filter(|s| s.online && !s.motd.is_empty())
            .map(|s| s.motd.as_str())
            .unwrap_or("Minecraft survival · Java & Bedrock");

        div()
            .absolute().top(px(23.)).left(px(26.))
            .w(px(if self.window_width < 850. { 328. } else { 440. }))
            .rounded(px(20.))
            .border_1()
            .border_color(ui::color(0xf7a7cc).opacity(0.64))
            .bg(ui::color(0x27152d).opacity(0.91))
            .shadow_lg()
            .p(px(16.))
            .flex().flex_col().gap(px(11.))
            .child(div().flex().items_center().gap(px(14.))
                .child(div().flex_none().w(px(82.)).h(px(82.))
                    .rounded(px(16.))
                    .overflow_hidden()
                    .border_1().border_color(ui::color(0xfb94c7).opacity(0.57))
                    .bg(ui::color(0x56304d).opacity(0.8))
                    .flex().items_center().justify_center()
                    .child(thumbnail))
                .child(div().flex_1().min_w_0()
                    .flex().flex_col().gap(px(8.))
                    .child(div().flex().items_center().flex_wrap().gap(px(10.))
                        .child(ui::text(20., FontWeight::BOLD, ink::FULL)
                            .child("Valtic SMP"))
                        .child(div().flex().items_center().gap(px(6.))
                            .rounded(px(12.)).px(px(9.)).py(px(5.))
                            .bg(ui::color(0x3b303c).opacity(0.84))
                            .child(div().size(px(7.)).rounded_full()
                                .bg(ui::color(colour)))
                            .child(ui::text(11., FontWeight::MEDIUM, ink::FULL)
                                .child(state))))
                    .child(ui::text(12., FontWeight::SEMIBOLD, ink::FULL)
                        .child(valtic_status::ADDRESS))
                    .child(ui::text(12., FontWeight::MEDIUM, ink::SOFT)
                        .child(players))))
            .child(div().w_full()
                .rounded(px(11.))
                .bg(ui::color(0x160f1d).opacity(0.52))
                .px(px(11.)).py(px(8.))
                .child(ui::text(12., FontWeight::NORMAL, ink::FULL)
                    .truncate()
                    .child(motd.to_string())))
    }

    pub(super) fn valtic_home_head(
        &mut self, tag: &str, prepared: bool, cx: &mut Context<Self>,
    ) -> Div {
        let java = self.selected_instance()
            .and_then(|i| i.java.as_ref().map(|j| j.minecraft.clone()));
        let ready = prepared || java.is_some();
        let compact = self.window_width < 1050.;

        let server_card = self.valtic_server_card();
        let mut petals = div().absolute().top_0().left_0().size_full();
        for (i, (x, y, size)) in [
            (0.06, 39., 10.), (0.22, 112., 7.), (0.43, -18., 9.),
            (0.68, 49., 8.), (0.90, 78., 12.),
            (0.12, 275., 8.), (0.76, 355., 12.),
        ].into_iter().enumerate() {
            let petal = div().absolute().left(gpui::relative(x)).top(px(y))
                .w(px(size * 1.4)).h(px(size * 0.8))
                .rounded(px(size / 2.))
                .bg(ui::color(if i % 3 == 0 { 0xffe3ed } else { 0xf6a5c9 })
                    .opacity(0.70));
            if motion::still() || self.settings.performance() {
                petals = petals.child(petal);
            } else {
                let drift = if i % 2 == 0 { 0.07 } else { -0.06 };
                petals = petals.child(petal.with_animation(
                    gpui::ElementId::named_usize("valtic-hero-petal", i),
                    gpui::Animation::new(
                        std::time::Duration::from_secs(11 + (i % 7) as u64)
                    ).repeat(),
                    move |d, p| d.top(px(y + p * 560.))
                        .left(gpui::relative(x + p * drift)),
                ));
            }
        }

        let play = div().flex().flex_wrap().items_center().justify_center()
            .gap(px(16.)).mt(px(33.))
            .child(self.play_group(tag, ready, cx))
            .children(java.is_none().then(|| self.glass_picker(cx)));

        let mut links = div().flex().flex_wrap()
            .items_center().justify_center()
            .gap(px(15.)).mt(px(22.));
        for (id, title, symbol, url) in [
            ("valtic-hero-discord", "Discord", "discord", self.links.discord.clone()),
            ("valtic-hero-website", "Website", "globe-02", self.links.website.clone()),
        ] {
            if url.is_empty() { continue; }
            links = links.child(
                div().id(id)
                    .flex().items_center().justify_center()
                    .gap(px(9.)).h(px(48.)).min_w(px(170.))
                    .px(px(18.))
                    .rounded(px(19.))
                    .border_1()
                    .border_color(ui::color(0xffb8d5).opacity(0.48))
                    .bg(ui::color(0x2c182f).opacity(0.88))
                    .hover(|d| d.bg(ui::color(0x754266).opacity(0.93)))
                    .cursor_pointer()
                    .child(ui::icon(symbol, 19., gpui::white()))
                    .child(ui::text(14., FontWeight::SEMIBOLD, ink::FULL)
                        .child(title))
                    .on_click(move |_, _, cx| cx.open_url(&url))
            );
        }

        div().relative().w_full()
            .min_h(px(if compact { 535. } else { 690. }))
            .flex().flex_col().items_center().justify_center()
            .px(px(14.)).py(px(32.))
            .child(div().absolute().top_0().left_0().size_full()
                .bg(gpui::linear_gradient(
                    90.,
                    gpui::linear_color_stop(ui::color(0x241228).opacity(0.07), 0.),
                    gpui::linear_color_stop(ui::color(0x1b0e22).opacity(0.37), 1.),
                )))
            .child(petals)
            .child(server_card)
            .child(div().relative()
                .flex().flex_col().items_center().justify_center()
                // The existing custom SVG has Valtic's block lettering,
                // torii roof and cherry blossoms, like the reference image.
                .child(gpui::svg()
                    .path("valtic/cherry-logo.svg")
                    .w(px(if compact { 430. } else { 660. }))
                    .h(px(if compact { 170. } else { 244. })))
                .child(ui::text(if compact { 33. } else { 46. },
                    FontWeight::NORMAL, ink::FULL)
                    .font_family(theme::DISPLAY_FONT)
                    .text_center()
                    .mt(px(7.))
                    .child("Welcome to Valtic"))
                .child(ui::text(if compact { 14. } else { 16. },
                    FontWeight::NORMAL, ink::FULL)
                    .mt(px(10.)).text_center()
                    .child("Your Minecraft experience, built for speed."))
                .child(play)
                .child(links))
    }

    /// A news card uses ONLY the downloaded official Minecraft article image.
    fn valtic_news_card(
        &mut self, article: minecraft_news::Article, index: usize,
    ) -> AnyElement {
        let picture = article.image_url.as_ref().and_then(|source| {
            let source = source.clone();
            self.icon_image(
                format!("valtic:official-news:{source}"), 540., false,
                || minecraft_news::cached_image(&source),
            )
        });
        let url = article.url.clone();
        let description = if article.summary.trim().is_empty() {
            "Read the latest official Minecraft article.".to_string()
        } else {
            article.summary.clone()
        };
        div()
            .id(SharedString::from(format!("valtic-news-card-{index}")))
            .flex().flex_col().flex_1().min_w(px(220.))
            .rounded(px(17.))
            .overflow_hidden()
            .bg(ui::color(0x1e1725).opacity(0.96))
            .border_1()
            .border_color(ui::color(0xf7b9d6).opacity(0.20))
            .cursor_pointer()
            .hover(|d| d.bg(ui::color(0x362337)))
            .child(div()
                .relative().w_full().h(px(155.))
                .overflow_hidden()
                .bg(gpui::linear_gradient(
                    130.,
                    gpui::linear_color_stop(ui::color(0x6c3c59), 0.),
                    gpui::linear_color_stop(ui::color(0x1b2832), 1.),
                ))
                .children(picture.map(|image| {
                    gpui::img(image)
                        .absolute().top_0().left_0().size_full()
                        .object_fit(gpui::ObjectFit::Cover)
                }))
                .child(div()
                    .absolute().top_0().left_0().size_full()
                    .bg(gpui::linear_gradient(
                        180.,
                        gpui::linear_color_stop(ui::color(0x110d17).opacity(0.), 0.),
                        gpui::linear_color_stop(ui::color(0x110d17).opacity(0.78), 1.),
                    )))
                .child(div()
                    .absolute().bottom(px(12.)).left(px(13.))
                    .px(px(9.)).py(px(5.))
                    .rounded(px(8.))
                    .bg(ui::color(0x171322).opacity(0.88))
                    .child(ui::text(11., FontWeight::SEMIBOLD, ink::FULL)
                        .child("OFFICIAL MINECRAFT")))
            )
            .child(div().flex().flex_col().gap(px(8.)).p(px(16.))
                .child(ui::text(17., FontWeight::BOLD, ink::FULL)
                    .child(article.title))
                .child(ui::text(12., FontWeight::NORMAL, ink::BODY)
                    .truncate().child(description))
                .child(ui::text(12., FontWeight::SEMIBOLD, ink::FULL)
                    .pt(px(5.)).child("Read article  ↗")))
            .on_click(move |_, _, cx| cx.open_url(&url))
            .into_any_element()
    }

    /// Tell unsigned visitors about restricted game access; Play opens the
    /// actual guest-locked native game once its assets are prepared.
    fn valtic_guest_preview(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let entries = ["Singleplayer", "Multiplayer", "Minecraft Realms"];
        let mut menu = div().flex().flex_col().gap(px(8.));
        for entry in entries {
            menu = menu.child(div()
                .px(px(18.)).py(px(13.))
                .rounded(px(8.))
                .border_1()
                .border_color(ui::color(0xffffff).opacity(0.12))
                .bg(ui::color(0x27222e).opacity(0.85))
                .child(ui::text(15., FontWeight::MEDIUM, ink::BODY)
                    .text_center().child(format!("{entry}  ·  Locked"))));
        }
        div()
            .id("valtic-guest-preview")
            .flex().flex_col().gap(px(14.))
            .p(px(22.)).rounded(px(18.))
            .bg(ui::color(0x1d1724).opacity(0.94))
            .border_1().border_color(ui::color(0xf7b8d8).opacity(0.24))
            .child(ui::text(20., FontWeight::BOLD, ink::FULL)
                .child("Guest Minecraft Menu"))
            .child(ui::text(13., FontWeight::NORMAL, ink::BODY)
                .child("Open the real game menu with Guest Play. Worlds, servers and Realms stay locked until you sign in with Microsoft. A prepared Valtic version is required."))
            .child(menu)
            .child(div().id("valtic-guest-sign-in")
                .cursor_pointer()
                .rounded(px(10.)).px(px(18.)).py(px(13.))
                .bg(ui::color(0x75435f))
                .hover(|d| d.bg(ui::color(0x9c587b)))
                .child(ui::text(14., FontWeight::BOLD, ink::FULL)
                    .text_center().child("Sign in with Microsoft to play"))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.sign_in_to_play();
                    cx.notify();
                })))
            .into_any_element()
    }

    /// Fresh Minecraft headlines, with images from Minecraft.net, not mockups or AI.
    pub(super) fn valtic_home_bottom(
        &mut self, rows: Div, cx: &mut Context<Self>,
    ) -> Div {
        let mut cards = div().flex().flex_wrap().gap(px(14.));
        for (index, article) in self.minecraft_news.clone().into_iter().take(3).enumerate() {
            cards = cards.child(self.valtic_news_card(article, index));
        }
        let no_news = self.minecraft_news.is_empty();
        let (news_heading, news_subtitle) = if self.minecraft_news_failed {
            ("Minecraft news is temporarily unavailable", "Open Minecraft.net using the button above, or check your connection.")
        } else {
            ("Checking official Minecraft news…", "Fetching the newest Minecraft.net stories and cover images.")
        };
        let news = div()
            .id("valtic-official-minecraft-news")
            .flex().flex_col().gap(px(14.))
            .p(px(20.))
            .rounded(px(19.))
            .border_1()
            .border_color(ui::color(0xf7b8d8).opacity(0.24))
            .bg(ui::color(0x1d1224).opacity(0.90))
            .child(div().flex().items_center().flex_wrap().gap(px(15.))
                .child(ui::text(18., FontWeight::BOLD, ink::FULL)
                    .child("Latest Minecraft News"))
                .child(div().id("valtic-minecraft-news-all").ml_auto().cursor_pointer()
                    .px(px(13.)).py(px(9.))
                    .rounded(px(11.))
                    .bg(ui::color(0x66344f).opacity(0.92))
                    .hover(|d| d.bg(ui::color(0x96506d)))
                    .child(ui::text(12., FontWeight::SEMIBOLD, ink::FULL)
                        .child("All Minecraft news  ↗"))
                    .on_click(|_, _, cx| cx.open_url(minecraft_news::NEWS_PAGE))))
            .child(if no_news {
                div().flex().flex_col().gap(px(6.))
                    .py(px(25.))
                    .child(ui::text(14., FontWeight::MEDIUM, ink::FULL)
                        .child(news_heading))
                    .child(ui::text(12., FontWeight::NORMAL, ink::BODY)
                        .child(news_subtitle))
            } else {
                cards
            })
            .child(ui::text(11., FontWeight::NORMAL, ink::BODY)
                .child("From Minecraft.net · Auto-refreshes every 45 minutes · Works offline with cached stories"));
        let advanced = if self.home_tab == home_tabs::HomeTab::Overview {
            None
        } else {
            Some(self.glass_home_tabs(rows, cx))
        };
        div().flex().flex_col()
            .gap(px(18.))
            .children((self.owner != Some(true)).then(|| self.valtic_guest_preview(cx)))
            .child(news)
            .child(self.glass_library(cx))
            .children(advanced)
            .child(ui::text(11., FontWeight::NORMAL, ink::DIM)
                .child("Valtic is an independent community launcher, not an official Mojang product."))
    }

    /// Minimal build manager: show only the build that corresponds to the
    /// currently published Minecraft release. Older downloads stay behind a
    /// clearly labelled opt-in archive for people who need them.
    pub(super) fn valtic_versions_page(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let latest = match &self.releases {
            Load::Ready(releases) => releases.iter().find(|r| r.asset.is_some()).cloned(),
            _ => None,
        };
        let tag = latest.as_ref().map(|r| r.tag.clone())
            .or_else(|| self.selected().cloned())
            .unwrap_or_default();
        let version = if tag.is_empty() {
            "Minecraft".to_string()
        } else {
            format!("Valtic {}", backend::game_version(&tag))
        };
        let installed = self.installed.iter().any(|v| v == &tag);
        let selected = self.selected().is_some_and(|v| v == &tag);
        let busy = self.installing.as_ref().is_some_and(|(v, _)| v == &tag);
        let cover = self.valtic_cover(&tag, 178.);
        let mut actions = div().flex().items_center().gap(px(10.));
        if installed && !selected {
            let choice = tag.clone();
            actions = actions.child(
                ui::solid("valtic-use-current", "Use version", 42., 14., &t)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.settings.selected = Some(choice.clone());
                        this.save_settings();
                        cx.notify();
                    }))
            );
        } else if installed {
            actions = actions.child(ui::text(14., FontWeight::MEDIUM, ink::STRONG)
                .child("Installed and ready"));
        } else if let Some(release) = latest {
            actions = actions.child(
                ui::solid("valtic-install-current",
                    if busy { "Installing…" } else { "Install version" },
                    42., 14., &t)
                    .when(!busy, |d| d.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.install(&release);
                        cx.notify();
                    })))
            );
        }
        let archive = ui::quiet("valtic-more-builds", "Browse older builds", 36., &t)
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.show_versions_archive = true;
                cx.notify();
            }));
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .pt(px(12.))
            .child(ui::text(30., FontWeight::BOLD, ink::FULL).child("Game version"))
            .child(ui::text(15., FontWeight::NORMAL, ink::BODY)
                .child("Just the Minecraft version you need. Valtic takes care of the game files."))
            .child(div()
                .flex()
                .flex_wrap()
                .gap(px(24.))
                .p(px(26.))
                .rounded(px(18.))
                .bg(ui::color(0x171d26))
                .child(cover)
                .child(div()
                    .flex()
                    .flex_col()
                    .items_start()
                    .flex_1()
                    .min_w(px(220.))
                    .gap(px(12.))
                    .child(ui::text(12., FontWeight::SEMIBOLD, ink::NOTE)
                        .child("MINECRAFT JAVA EDITION"))
                    .child(ui::text(30., FontWeight::BOLD, ink::FULL).child(version))
                    .child(ui::text(14., FontWeight::NORMAL, ink::BODY)
                        .child("Official game files. Your worlds and instances stay where they are."))
                    .child(actions)))
            .child(archive)
    }
}
