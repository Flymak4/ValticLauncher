//! Valtic's home: a cinematic Minecraft launchpad, not a settings panel.
//! Scenes are read from the *selected version's* official game panoramas.
//! No generated or unrelated version artwork is used.
use super::*;
use crate::glass_ui::{self as ui, white};
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

    /// The main launch moment. Play, version selection and instance settings
    /// retain their original behaviour; only the composition changes.
    pub(super) fn valtic_home_head(
        &mut self, tag: &str, prepared: bool, cx: &mut Context<Self>,
    ) -> Div {
        let selected = self.selected_instance().cloned();
        let java_version = selected.as_ref()
            .and_then(|i| i.java.as_ref().map(|j| j.minecraft.clone()));
        let java = java_version.is_some();
        let version = java_version.unwrap_or_else(|| backend::game_version(tag));
        let current_263 = version == "26.3";
        let ready = prepared || java;
        let compact = self.window_width < 1090.;
        let instance_name = selected.as_ref().map(|i| i.name.clone())
            .unwrap_or_else(|| "My Valtic".to_string());

        // No AI art, fake scenery or art from a different Minecraft version.
        // On an unprepared install the fallback is deliberately abstract.
        let panorama = self.valtic_scene(tag, 0, 1080.);
        let no_panorama = panorama.is_none();
        let grass = if no_panorama { self.grass_mark(176.) } else { None };
        // The official Valtic cherry-blossom banner is loaded from the
        // launcher's bundled assets when present; no generated replacement.
        let banner_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/valtic/cherry_banner.png");
        let valtic_banner = self.icon_image(
            "valtic:cherry-banner".to_string(), 860., false,
            || std::fs::read(&banner_path).ok(),
        );
        let picker = (!java).then(|| self.glass_picker(cx));
        let edit = selected.clone();

        let scene = div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .bg(gpui::linear_gradient(
                115.,
                gpui::linear_color_stop(ui::color(0x69465f), 0.),
                gpui::linear_color_stop(ui::color(0x2c4b49), 1.),
            ))
            .children(panorama.map(|p| {
                gpui::img(p)
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .object_fit(gpui::ObjectFit::Cover)
            }))
            // The image recedes beneath the controls; this is a normal UI veil,
            // not synthetic imagery.
            .child(div()
                .absolute().top_0().left_0().size_full()
                .bg(gpui::linear_gradient(
                    90.,
                    gpui::linear_color_stop(ui::color(0x1b1521).opacity(0.97), 0.),
                    gpui::linear_color_stop(ui::color(0x1b1521).opacity(0.38), 1.),
                )))
            .child(div()
                .absolute().bottom_0().left_0().w_full().h(px(140.))
                .bg(gpui::linear_gradient(
                    180.,
                    gpui::linear_color_stop(ui::color(0x1b1521).opacity(0.), 0.),
                    gpui::linear_color_stop(ui::color(0x1b1521).opacity(0.6), 1.),
                )));

        let unprepared_art = div()
            .absolute()
            .right(px(28.))
            .top(px(45.))
            .size(px(240.))
            .rounded(px(26.))
            .border_1()
            .border_color(ui::color(0xffffff).opacity(0.09))
            .bg(ui::color(0xffffff).opacity(0.045))
            .flex()
            .items_center()
            .justify_center()
            .children(grass.map(|p| {
                gpui::img(p).size(px(176.)).object_fit(gpui::ObjectFit::Contain)
            }));
        let launch_controls = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(11.))
            .pt(px(15.))
            .child(self.play_group(tag, ready, cx))
            .children(picker);

        let details = div()
            .relative()
            .flex()
            .flex_col()
            .items_start()
            .max_w(px(if compact { 490. } else { 620. }))
            .gap(px(10.))
            .child(div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(div().size(px(7.)).rounded_full().bg(ui::color(0xffb9d5)))
                .child(ui::text(12., FontWeight::BOLD, ink::FULL)
                    .child("✿  A WORLD IN BLOOM")))
            .children(valtic_banner.map(|banner| {
                gpui::img(banner)
                    .w(px(if compact { 340. } else { 560. }))
                    .h(px(if compact { 116. } else { 188. }))
                    .object_fit(gpui::ObjectFit::Contain)
            }))
            .child(ui::text(49., FontWeight::BOLD, ink::FULL)
                .line_height(px(54.))
                .font_family(theme::DISPLAY_FONT)
                .child(if banner_path.exists() { "" } else { "Valtic" }))
            .child(ui::text(19., FontWeight::SEMIBOLD, ink::FULL).child(
                if current_263 { "Your next adventure is blooming" } else { "A new adventure is blooming" }
            ))
            .child(ui::text(14., FontWeight::NORMAL, ink::FULL).child(
                if current_263 {
                    "Step into a world of pink skies, falling petals and endless Minecraft adventures."
                } else {
                    "Your worlds, mods and favourite servers. Everything in one place."
                }
            ))
            .child(launch_controls)
            .child(div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(12.))
                .pt(px(9.))
                .child(ui::text(12., FontWeight::MEDIUM, ink::FULL)
                    .child(format!("●  {}", if ready { "READY TO PLAY" } else { "FIRST LAUNCH SETUP" })))
                .child(ui::text(12., FontWeight::NORMAL, ink::BODY)
                    .child(format!("INSTANCE  ·  {instance_name}"))))
            .children(edit.map(|instance| {
                ui::quiet("valtic-edit-instance", "Configure instance  ↗", 32., &self.theme)
                    .mt(px(2.))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.open_instance_dialog(Some(instance.clone()));
                        cx.notify();
                    }))
            }));

        // Soft blossom petals are native GPUI shapes: no generated art or external assets.
        let blossom_petals = div()
            .absolute().top_0().right_0().size_full()
            .child(div().absolute().top(px(34.)).right(px(84.))
                .size(px(13.)).rounded(px(5.))
                .bg(ui::color(0xffd2e3).opacity(0.72)))
            .child(div().absolute().top(px(96.)).right(px(165.))
                .size(px(8.)).rounded(px(3.))
                .bg(ui::color(0xffa8cb).opacity(0.66)))
            .child(div().absolute().top(px(173.)).right(px(46.))
                .size(px(11.)).rounded(px(5.))
                .bg(ui::color(0xffc5dc).opacity(0.65)))
            .child(div().absolute().bottom(px(52.)).right(px(228.))
                .size(px(7.)).rounded(px(3.))
                .bg(ui::color(0xffe3ed).opacity(0.78)));

        let hero = div()
            .relative()
            .w_full()
            .min_h(px(332.))
            .rounded(px(19.))
            .overflow_hidden()
            .bg(ui::color(0x2d2735))
            .child(scene)
            .child(blossom_petals)
            .children((no_panorama && !compact).then_some(unprepared_art))
            .child(div()
                .relative()
                .p(px(if compact { 25. } else { 32. }))
                .child(details));

        div()
            .flex()
            .flex_col()
            .pt(px(14.))
            .gap(px(16.))
            .child(div()
                .flex()
                .justify_between()
                .items_center()
                .child(ui::text(25., FontWeight::BOLD, ink::FULL)
                    .child("Welcome to the Blossom Grove"))
                .child(ui::text(11., FontWeight::SEMIBOLD, ink::NOTE)
                    .child("MINECRAFT JAVA EDITION")))
            .child(hero)
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
        let stories = if current_263 {
            div()
                .flex()
                .flex_wrap()
                .gap(px(14.))
                .child(self.valtic_feature_card(
                    &tag, 1, "NEW BIOME", "The Dappled Forest",
                    "Explore autumn poplar forests and discover a beautiful new biome.",
                    notes_url, 0x86566d,
                ))
                .child(self.valtic_feature_card(
                    &tag, 2, "EXPLORATION", "Abandoned Camps",
                    "Follow explorer maps, discover camps and head into the unknown.",
                    notes_url, 0x547d6b,
                ))
                .child(self.valtic_feature_card(
                    &tag, 3, "NEW CONTENT", "Build Something New",
                    "Fresh poplar wood, cushions, wool stairs and concrete slabs.",
                    notes_url, 0x6d648a,
                ))
        } else {
            div()
                .flex()
                .gap(px(14.))
                .child(self.valtic_feature_card(
                    &tag, 1, "RELEASE NOTES", "Explore Minecraft updates",
                    "See the official features and news for Minecraft.",
                    notes_url, 0x3e535f,
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
                        .child("Stories from the Grove"))
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
