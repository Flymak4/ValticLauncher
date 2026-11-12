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

    /// Full-bleed Cherry Grove home. Reuses the existing real launch controls.
    pub(super) fn valtic_home_head(
        &mut self, tag: &str, prepared: bool, cx: &mut Context<Self>,
    ) -> Div {
        let selected = self.selected_instance().cloned();
        let java = selected.as_ref()
            .and_then(|i| i.java.as_ref().map(|j| j.minecraft.clone()));
        let version = java.clone().unwrap_or_else(|| backend::game_version(tag));
        let ready = prepared || java.is_some();
        let compact = self.window_width < 980.;
        let instance_name = selected.as_ref().map(|i| i.name.clone())
            .unwrap_or_else(|| "My Valtic".to_string());

        // Bundle a Valtic scene so even a clean, first-time installation has art.
        // Include the scene in the executable: it must survive distribution as
        // a standalone Windows .exe, not just work in the GitHub build folder.
        let art = self.icon_image(
            "valtic:cherry-scene-v2".to_string(), 1800., false,
            || Some(include_bytes!("../assets/valtic/cherry_grove.jpg").to_vec()),
        );
        let custom_art = if art.is_none() {
            self.home_background().and_then(|path| {
                self.picture(&path, picture_cache::Look::Fit(self.window_side()), cx)
            })
        } else { None };
        let panorama = if art.is_none() && custom_art.is_none() {
            self.valtic_scene(tag, 0, 1500.)
        } else { None };
        let art = art.or(custom_art).or(panorama);

        // Always embed the actual Valtic voxel banner, including in standalone .exe builds.
        let banner = self.icon_image(
            "valtic:cherry-banner-v2".to_string(), 900., false,
            || Some(include_bytes!("../assets/valtic/cherry_banner.webp").to_vec()),
        );
        let no_banner = banner.is_none();
        let picker = java.is_none().then(|| self.glass_picker(cx));
        let scene = div()
            .absolute().top_0().left_0().size_full()
            .bg(gpui::linear_gradient(
                115.,
                gpui::linear_color_stop(ui::color(0x854767), 0.),
                gpui::linear_color_stop(ui::color(0x182d35), 1.),
            ))
            .children(art.map(|p| {
                gpui::img(p).absolute().top_0().left_0().size_full()
                    .object_fit(gpui::ObjectFit::Cover)
            }))
            .child(div().absolute().top_0().left_0().size_full()
                .bg(gpui::linear_gradient(
                    90.,
                    gpui::linear_color_stop(ui::color(0x201024).opacity(0.20), 0.),
                    gpui::linear_color_stop(ui::color(0x190e1d).opacity(0.53), 1.),
                )))
            .child(div().absolute().bottom_0().left_0().w_full().h(px(135.))
                .bg(gpui::linear_gradient(
                    180.,
                    gpui::linear_color_stop(ui::color(0x1d1220).opacity(0.), 0.),
                    gpui::linear_color_stop(ui::color(0x1d1220).opacity(0.95), 1.),
                )));

        // Independent looping petals; honour reduced-motion and performance mode.
        let petals_data: [(f32, f32, f32); 11] = [
            (0.08, -20., 10.), (0.20, 60., 7.), (0.37, -80., 13.),
            (0.55, 100., 8.), (0.76, 10., 9.), (0.91, 90., 7.),
            (0.14, 300., 11.), (0.31, 390., 8.), (0.59, 330., 10.),
            (0.82, 390., 7.), (0.95, 270., 9.),
        ];
        let mut petals = div().absolute().top_0().left_0().size_full();
        for (i, (left, top, size)) in petals_data.into_iter().enumerate() {
            let petal = div()
                .absolute().left(gpui::relative(left)).top(px(top))
                .size(px(size)).rounded(px(size * 0.4))
                .bg(ui::color(if i % 3 == 0 { 0xffe0f0 } else { 0xffaecf })
                    .opacity(if i % 2 == 0 { 0.68 } else { 0.50 }));
            if motion::still() || self.settings.performance() {
                petals = petals.child(petal);
            } else {
                let drift = if i % 2 == 0 { 0.05 } else { -0.035 };
                petals = petals.child(petal.with_animation(
                    gpui::ElementId::named_usize("valtic-blossom-petal", i),
                    gpui::Animation::new(std::time::Duration::from_secs(9 + (i % 7) as u64))
                        .repeat(),
                    move |d, p| d.top(px(top + p * 550.))
                        .left(gpui::relative(left + p * drift)),
                ));
            }
        }

        let controls = div()
            .flex().flex_wrap().items_center().justify_center()
            .gap(px(14.)).mt(px(16.))
            .child(self.play_group(tag, ready, cx))
            .children(picker);
        let discord = self.links.discord.clone();
        let website = self.links.website.clone();
        let mut links = div()
            .flex().flex_wrap().justify_center().gap(px(10.)).mt(px(20.));
        if !discord.is_empty() {
            links = links.child(
                ui::quiet("blossom-discord", "Discord  ↗", 40., &self.theme)
                    .on_click(move |_, _, cx| cx.open_url(&discord)),
            );
        }
        if !website.is_empty() {
            links = links.child(
                ui::quiet("blossom-website", "Website  ↗", 40., &self.theme)
                    .on_click(move |_, _, cx| cx.open_url(&website)),
            );
        }
        links = links.child(
            ui::quiet("blossom-news", "Minecraft news  ↗", 40., &self.theme)
                .on_click(|_, _, cx| cx.open_url(MINECRAFT_UPDATES)),
        );

        let title = div().flex().flex_col().items_center().gap(px(10.))
            .children(banner.map(|p| {
                gpui::img(p)
                    .w(px(if compact { 360. } else { 590. }))
                    .h(px(if compact { 120. } else { 190. }))
                    .object_fit(gpui::ObjectFit::Contain)
            }))
            .when(no_banner, |d| d.child(
                ui::text(if compact { 56. } else { 74. },
                    FontWeight::BOLD, ink::FULL)
                    .font_family(theme::DISPLAY_FONT)
                    .child("VALTIC"),
            ))
            .child(ui::text(32., FontWeight::BOLD, ink::FULL)
                .font_family(theme::DISPLAY_FONT)
                .text_center()
                .child("Welcome to Valtic"))
            .child(ui::text(16., FontWeight::MEDIUM, ink::FULL)
                .text_center()
                .child("Your Minecraft experience, built for Valtic SMP."));

        let details = div()
            .flex().flex_wrap().items_center().justify_center()
            .gap(px(14.)).mt(px(20.))
            .child(ui::text(12., FontWeight::SEMIBOLD, ink::FULL)
                .child(if ready { "●  READY TO PLAY" } else { "●  FIRST LAUNCH SETUP" }))
            .child(ui::text(12., FontWeight::MEDIUM, ink::FULL)
                .child(format!("MINECRAFT {version}  ·  {instance_name}")))
            .children(selected.map(|instance| {
                ui::quiet("blossom-edit-instance", "Configure  ↗", 32., &self.theme)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.open_instance_dialog(Some(instance.clone()));
                        cx.notify();
                    }))
            }));

        let hero_height = if compact { 535. } else { 610. };
        let hero = div()
            .relative().w_full().min_h(px(hero_height))
            .rounded(px(14.)).overflow_hidden()
            .bg(ui::color(0x2d1c33))
            .child(scene).child(petals)
            .child(div().relative().flex().flex_col()
                .items_center().justify_center().min_h(px(hero_height))
                .px(px(if compact { 12. } else { 28. }))
                .py(px(25.))
                .child(title).child(controls).child(links).child(details));
        div().flex().flex_col().pt(px(12.)).child(hero)
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
        // Every card has a real image even before Minecraft has been downloaded.
        let bundled = match picture_index {
            1 => self.icon_image(
                "valtic:grove-news-1".to_string(), 640., false,
                || Some(include_bytes!("../assets/valtic/news_1.jpg").to_vec()),
            ),
            2 => self.icon_image(
                "valtic:grove-news-2".to_string(), 640., false,
                || Some(include_bytes!("../assets/valtic/news_2.jpg").to_vec()),
            ),
            _ => self.icon_image(
                "valtic:grove-news-3".to_string(), 640., false,
                || Some(include_bytes!("../assets/valtic/news_3.jpg").to_vec()),
            ),
        };
        let picture = bundled.or_else(|| self.valtic_scene(tag, picture_index, 580.));
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
                    div().absolute().top_0().left_0().size_full()
                        .flex().items_center().justify_center()
                        .child(ui::text(76., FontWeight::BOLD, ink::FULL)
                            .child(match picture_index {
                                1 => "✿",
                                2 => "❖",
                                _ => "✦",
                            }))
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
        let notes_url = if version == "26.3" { RELEASE_NOTES_263 } else { MINECRAFT_UPDATES };
        // Use official news destinations instead of invented release stories.
        let stories = div().flex().flex_wrap().gap(px(14.))
            .child(self.valtic_feature_card(
                &tag, 1, "OFFICIAL NEWS", "Minecraft updates",
                "Discover the latest official Minecraft news and features.",
                MINECRAFT_UPDATES, 0x86536f,
            ))
            .child(self.valtic_feature_card(
                &tag, 2, "JAVA EDITION", "Explore the changes",
                "See the official changes for your selected Minecraft release.",
                notes_url, 0x486e67,
            ))
            .child(self.valtic_feature_card(
                &tag, 3, "RELEASE NOTES", "Build something new",
                "Keep up with updates before your next adventure.",
                notes_url, 0x645b80,
            ));
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
                    .child("MINECRAFT NEWS & UPDATES")))
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
