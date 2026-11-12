//! Valtic's home: a cinematic Minecraft launchpad, not a settings panel.
//! Scenes are read from the *selected version's* official game panoramas.
//! No generated or unrelated version artwork is used.
use super::*;
use crate::glass_ui::{self as ui};
use horizon_theme::glass::{ink, radius};
use std::sync::Arc;

const RELEASE_NOTES_263: &str =
    "https://www.minecraft.net/en-us/article/minecraft-java-edition-26-3";
const MINECRAFT_UPDATES: &str =
    "https://www.minecraft.net/en-us/updates/minecraft-updates-timeline-and-evolution";

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
    pub(super) fn valtic_home_head(
        &mut self, tag: &str, prepared: bool, cx: &mut Context<Self>,
    ) -> Div {
        let instance = self.selected_instance().cloned();
        let java = instance.as_ref()
            .and_then(|i| i.java.as_ref().map(|j| j.minecraft.clone()));
        let version = java.clone().unwrap_or_else(|| backend::game_version(tag));
        let ready = prepared || java.is_some();
        let compact = self.window_width < 1050.;
        let name = instance.as_ref()
            .map(|i| i.name.clone()).unwrap_or_else(|| "My Valtic".to_string());

        // Small live blossom petals float over the hero. The scenery itself is
        // drawn across the entire window by main.rs.
        let mut petals = div().absolute().top_0().left_0().size_full();
        for (i, (x, y, size)) in [
            (0.06, 19., 10.), (0.23, 68., 7.), (0.41, -15., 10.),
            (0.63, 27., 8.), (0.88, 54., 13.), (0.96, 196., 9.),
            (0.15, 288., 8.), (0.72, 342., 12.), (0.31, 399., 7.),
        ].into_iter().enumerate() {
            let petal = div().absolute().left(gpui::relative(x)).top(px(y))
                .w(px(size * 1.4)).h(px(size * 0.8))
                .rounded(px(size / 2.))
                .bg(ui::color(if i % 3 == 0 { 0xffdfe9 } else { 0xf6adcf })
                    .opacity(0.72));
            if motion::still() || self.settings.performance() {
                petals = petals.child(petal);
            } else {
                let drift = if i % 2 == 0 { 0.07 } else { -0.06 };
                petals = petals.child(petal.with_animation(
                    gpui::ElementId::named_usize("valtic-falling-petal-v2", i),
                    gpui::Animation::new(
                        std::time::Duration::from_secs(11 + (i % 7) as u64)
                    ).repeat(),
                    move |d, p| d.top(px(y + p * 560.))
                        .left(gpui::relative(x + p * drift)),
                ));
            }
        }

        let play = div()
            .flex().flex_wrap().items_center().justify_center()
            .gap(px(16.)).mt(px(21.))
            .child(self.play_group(tag, ready, cx))
            .children(java.is_none().then(|| self.glass_picker(cx)));

        // These buttons remain real actions, not decorative placeholders.
        let mut links = div().flex().flex_wrap()
            .items_center().justify_center()
            .gap(px(12.)).mt(px(23.));
        for (id, title, symbol, url) in [
            ("blossom-discord", "Discord", "discord", self.links.discord.clone()),
            ("blossom-website", "Website", "globe-02", self.links.website.clone()),
        ] {
            if url.is_empty() { continue; }
            links = links.child(
                div().id(id)
                    .flex().items_center().justify_center()
                    .gap(px(8.)).h(px(46.)).min_w(px(108.))
                    .px(px(16.))
                    .rounded(px(13.))
                    .border_1()
                    .border_color(ui::color(0xffb7d7).opacity(0.33))
                    .bg(ui::color(0x3b263b).opacity(0.86))
                    .hover(|d| d.bg(ui::color(0x71425f).opacity(0.95)))
                    .cursor_pointer()
                    .child(ui::icon(symbol, 19., gpui::white()))
                    .child(ui::text(14., FontWeight::SEMIBOLD, ink::FULL).child(title))
                    .on_click(move |_, _, cx| cx.open_url(&url)),
            );
        }

        let status = div().flex().flex_wrap()
            .items_center().justify_center().gap(px(14.)).mt(px(19.))
            .child(ui::text(12., FontWeight::MEDIUM, ink::FULL)
                .child(format!("●  {}  ·  Minecraft {version}",
                    if ready { "Ready to play" } else { "First launch setup" })))
            .child(ui::text(12., FontWeight::MEDIUM, ink::STRONG)
                .child(name))
            .children(instance.map(|i| {
                ui::quiet("valtic-configure", "Configure  ↗", 32., &self.theme)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.open_instance_dialog(Some(i.clone()));
                        cx.notify();
                    }))
            }));

        let logo = self.icon_image(
            "valtic:real-cherry-banner-v3".to_string(), 1000., false,
            || Some(include_bytes!("../assets/valtic/cherry_banner.webp").to_vec()),
        );
        let logo_missing = logo.is_none();
        let hero_height = if compact { 490. } else { 560. };
        let hero = div().relative()
            .w_full().min_h(px(hero_height))
            .flex().flex_col().items_center().justify_center()
            .px(px(if compact { 12. } else { 24. }))
            .py(px(18.))
            .child(
                div().absolute().top_0().left_0().size_full()
                    .bg(gpui::linear_gradient(
                        90.,
                        gpui::linear_color_stop(ui::color(0x2a1731).opacity(0.08), 0.),
                        gpui::linear_color_stop(ui::color(0x251228).opacity(0.26), 1.),
                    ))
            )
            .child(petals)
            .child(div().relative()
                .flex().flex_col().items_center().justify_center()
                .children(logo.map(|picture| {
                    gpui::img(picture)
                        .w(px(if compact { 390. } else { 630. }))
                        .h(px(if compact { 145. } else { 205. }))
                        .object_fit(gpui::ObjectFit::Contain)
                }))
                .when(logo_missing, |d| d.child(
                    gpui::svg().path("valtic/cherry-logo.svg")
                        .w(px(if compact { 390. } else { 615. }))
                        .h(px(if compact { 145. } else { 205. }))
                ))
                .child(ui::text(if compact { 29. } else { 39. },
                        FontWeight::BOLD, ink::FULL)
                    .font_family(theme::DISPLAY_FONT)
                    .text_center()
                    .child("Welcome to Valtic"))
                .child(ui::text(16., FontWeight::NORMAL, ink::FULL)
                    .mt(px(7.)).text_center()
                    .child("Your Minecraft experience, built for Valtic SMP."))
                .child(play).child(links).child(status));

        div().flex().flex_col().child(hero)
    }

    /// Rich discovery tiles use only panoramas taken from the selected build.
    /// On first launch they stay as attractive, clearly abstract colour panels.
    fn valtic_feature_card(
        &mut self,
        tag: &str,
        picture_index: u8,
        eyebrow: &'static str,
        title: &'static str,
        description: &'static str,
        url: &'static str,
        hue: u32,
    ) -> AnyElement {
        let picture = self.valtic_scene(tag, picture_index, 580.);
        let missing_picture = picture.is_none();
        div()
            .id(SharedString::from(format!("valtic-story-{picture_index}")))
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(215.))
            .rounded(px(16.))
            .overflow_hidden()
            .bg(ui::color(0x29212e))
            .border_1()
            .border_color(ui::color(0xffffff).opacity(0.06))
            .hover(|d| d.bg(ui::color(0x3a2a3b)))
            .cursor_pointer()
            .child(div()
                .relative()
                .w_full()
                .h(px(142.))
                .overflow_hidden()
                .bg(gpui::linear_gradient(
                    140.,
                    gpui::linear_color_stop(ui::color(hue), 0.),
                    gpui::linear_color_stop(ui::color(0x161e2c), 1.),
                ))
                .children(picture.map(|p| {
                    gpui::img(p)
                        .absolute().top_0().left_0().size_full()
                        .object_fit(gpui::ObjectFit::Cover)
                }))
                .children(missing_picture.then(|| {
                    gpui::svg().path(match picture_index {
                        1 => "valtic/news-forest.svg",
                        2 => "valtic/news-camp.svg",
                        _ => "valtic/news-cherry.svg",
                    })
                    .absolute().top_0().left_0().size_full()
                }))
                .child(div()
                    .absolute().bottom_0().left_0().size_full()
                    .bg(gpui::linear_gradient(
                        180.,
                        gpui::linear_color_stop(ui::color(0x1b1521).opacity(0.), 0.),
                        gpui::linear_color_stop(ui::color(0x1b1521).opacity(0.76), 1.),
                    )))
                .child(div()
                    .absolute().bottom(px(14.)).left(px(15.))
                    .px(px(10.)).py(px(5.))
                    .rounded(px(8.))
                    .bg(ui::color(0x11161e).opacity(0.80))
                    .child(ui::text(11., FontWeight::BOLD, ink::FULL).child(eyebrow))))
            .child(div()
                .flex()
                .flex_col()
                .gap(px(7.))
                .p(px(17.))
                .child(ui::text(18., FontWeight::BOLD, ink::FULL).child(title))
                .child(ui::text(13., FontWeight::NORMAL, ink::BODY).child(description))
                .child(ui::text(12., FontWeight::SEMIBOLD, ink::FULL)
                    .pt(px(6.))
                    .child("Discover more  ↗")))
            .on_click(move |_, _, cx| cx.open_url(url))
            .into_any_element()
    }

    pub(super) fn valtic_home_bottom(
        &mut self, rows: Div, cx: &mut Context<Self>,
    ) -> Div {
        let tag = self.selected().cloned().unwrap_or_default();
        let version = self.selected_instance()
            .and_then(|i| i.java.as_ref().map(|j| j.minecraft.clone()))
            .unwrap_or_else(|| backend::game_version(&tag));
        let current_263 = version == "26.3";
        let notes_url = if current_263 { RELEASE_NOTES_263 } else { MINECRAFT_UPDATES };
        // Real Minecraft content only: the three news tiles correspond to
        // documented 26.3 features, not invented Valtic announcements.
        let stories = if current_263 {
            div().flex().flex_wrap().gap(px(14.))
                .child(self.valtic_feature_card(
                    &tag, 1, "WILDERNESS BOUND", "The Dappled Forest",
                    "Explore the new forest biome and the poplar wood set.",
                    notes_url, 0x658d7d,
                ))
                .child(self.valtic_feature_card(
                    &tag, 2, "EXPLORATION", "Abandoned Camps",
                    "Follow explorer maps to hidden camps in the wild.",
                    notes_url, 0x83638e,
                ))
                .child(self.valtic_feature_card(
                    &tag, 3, "JAVA 26.3", "What's new?",
                    "Cushions, straw beds, concrete stairs and more.",
                    notes_url, 0x9d6483,
                ))
        } else {
            div().flex().flex_wrap().gap(px(14.))
                .child(self.valtic_feature_card(
                    &tag, 1, "OFFICIAL NEWS", "Minecraft updates",
                    "Discover the latest official news and releases.",
                    MINECRAFT_UPDATES, 0x86536f,
                ))
                .child(self.valtic_feature_card(
                    &tag, 2, "JAVA EDITION", "Your next adventure",
                    "Explore Minecraft worlds and discoveries.",
                    MINECRAFT_UPDATES, 0x53766c,
                ))
                .child(self.valtic_feature_card(
                    &tag, 3, "COMMUNITY", "More to explore",
                    "Read news from the creators of Minecraft.",
                    MINECRAFT_UPDATES, 0x6c638f,
                ))
        };
        let news = div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .pt(px(24.))
            .child(div()
                .flex()
                .justify_between()
                .items_center()
                .child(div()
                    .flex().flex_col().gap(px(4.))
                    .child(ui::text(23., FontWeight::BOLD, ink::FULL)
                        .child("Latest News"))
                    .child(ui::text(13., FontWeight::NORMAL, ink::BODY)
                        .child(format!(
                            "The latest highlights for Minecraft {version}"
                        ))))
                .child(ui::text(12., FontWeight::SEMIBOLD, ink::STRONG)
                    .child("OFFICIAL MINECRAFT NEWS")))
            .child(stories)
            .child(div()
                .id("valtic-all-notes")
                .flex()
                .items_center()
                .justify_between()
                .px(px(20.))
                .py(px(15.))
                .rounded(px(12.))
                .bg(ui::color(0x29212e))
                .hover(|d| d.bg(ui::color(0x3b2c40)))
                .cursor_pointer()
                .child(ui::text(14., FontWeight::SEMIBOLD, ink::FULL)
                    .child(format!("Read the official Minecraft {version} release notes")))
                .child(ui::text(16., FontWeight::BOLD, ink::FULL).child("↗"))
                .on_click(move |_, _, cx| cx.open_url(notes_url)));

        let mut shortcuts = div().flex().flex_wrap().gap(px(10.)).mt(px(22.));
        for (label, tab) in [
            ("Explore your worlds  ↗", home_tabs::HomeTab::Worlds),
            ("Manage your mods  ↗", home_tabs::HomeTab::Mods),
        ] {
            shortcuts = shortcuts.child(
                ui::quiet(format!("valtic-shortcut-{label}"), label, 40., &self.theme)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.home_tab = tab;
                        cx.notify();
                    }))
            );
        }
        let advanced = if self.home_tab == home_tabs::HomeTab::Overview {
            None
        } else {
            Some(self.glass_home_tabs(rows, cx))
        };
        div()
            .flex()
            .flex_col()
            .child(news)
            .child(shortcuts)
            .child(self.glass_library(cx))
            .children(advanced)
            .child(ui::text(11., FontWeight::NORMAL, ink::DIM).pt(px(20.))
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
