//! Valtic's home: a cinematic Minecraft launchpad, not a settings panel.
//! Scenes are read from the *selected version's* official game panoramas.
//! No generated or unrelated version artwork is used.
use super::*;
use crate::glass_home::HomeMenu;
use crate::glass_ui::{self as ui, white};
use gpui::deferred;
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


    /// Home's instance selector. Picking an instance changes the Play target;
    /// it never downloads a different version or starts the game by itself.
    fn valtic_instance_picker(&self, cx: &mut Context<Self>) -> Div {
        let theme = self.theme;
        let selected_id = self.selected_instance().map(|i| i.id.clone());
        let selected_name = self.selected_instance()
            .map(|i| i.name.clone())
            .unwrap_or_else(|| "Choose an instance".to_string());
        let open = self.picker.open;
        let button = div()
            .id("valtic-instance-picker")
            .w_full()
            .h(px(70.))
            .flex().items_center().justify_between().gap(px(16.))
            .px(px(16.))
            .rounded(px(radius::LARGE))
            .bg(ui::color(0x39263b).opacity(0.90))
            .hover(|d| d.bg(ui::color(0x54344d)))
            .cursor_pointer()
            .child(div().flex().flex_col().min_w_0()
                .child(ui::text(12., FontWeight::NORMAL, ink::QUIET)
                    .child("Instance"))
                .child(ui::text(15., FontWeight::MEDIUM, ink::FULL)
                    .truncate().child(selected_name)))
            .child(ui::icon("arrow-down-01", 18., white(ink::NOTE)))
            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                if !this.closed_by_this_press(event) {
                    this.home_menu = HomeMenu::None;
                    this.picker.open = !this.picker.open;
                }
                cx.notify();
            }));
        let mut wrap = div().relative().w_full().child(button);
        if open {
            let mut menu = ui::popover(300.)
                .id("valtic-instance-menu")
                .occlude()
                .bg(ui::color(0x16112b).opacity(0.96))
                .on_mouse_down_out(cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    this.picker.open = false;
                    this.menu_closed_at = Some(event.position);
                    cx.notify();
                }));
            let playable: Vec<_> = self.instances.list.iter()
                .filter(|i| matches!(i.kind,
                    instances::Kind::Default | instances::Kind::Own))
                .cloned().collect();
            if playable.is_empty() {
                menu = menu.child(
                    ui::text(12., FontWeight::NORMAL, ink::SOFT)
                        .px(px(12.)).py(px(10.))
                        .child("No instances yet. Create one to start playing.")
                );
            }
            for instance in playable {
                let chosen = selected_id.as_deref() == Some(instance.id.as_str());
                let note = instance.java.as_ref()
                    .map(|j| format!("Minecraft {}", j.minecraft))
                    .unwrap_or_else(|| "Valtic Client".to_string());
                let target = instance.clone();
                menu = menu.child(
                    ui::menu_item(
                        SharedString::from(format!("valtic-choose-{}", instance.id)),
                        instance.name.into(),
                        note,
                        false,
                        &theme,
                    )
                    .when(chosen, |d| d.child(
                        ui::icon("tick-02", 17., ui::color(self.look.accent_soft))
                    ))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.select_instance(&target);
                        this.picker.open = false;
                        cx.notify();
                    }))
                );
            }
            menu = menu.child(
                ui::menu_row(
                    "valtic-create-instance", "plus-sign",
                    "Create Instance", false, &theme,
                ).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.picker.open = false;
                    this.open_instance_dialog(None);
                    cx.notify();
                }))
            );
            wrap = wrap.child(deferred(
                div().absolute().top(px(70. + 8.)).left_0()
                    .child(menu.max_h(px(380.)).overflow_y_scroll())
            ).with_priority(1));
        }
        wrap
    }

    pub(super) fn valtic_home_head(
        &mut self, tag: &str, prepared: bool, cx: &mut Context<Self>,
    ) -> Div {
        let java = self.selected_instance()
            .and_then(|i| i.java.as_ref().map(|j| j.minecraft.clone()));
        let ready = prepared || java.is_some();
        let compact = self.window_width < 1050.;

        // This is Valtic's bundled, real Minecraft cherry-grove artwork. Decode it once
        // through the image cache rather than constructing new GPU textures each frame.
        let cherry_backdrop = self.icon_image(
            "valtic:cherry-grove-backdrop".into(),
            1400.,
            true,
            || Some(include_bytes!("../assets/valtic/cherry_grove.jpg").to_vec()),
        );
        let mut petals = div().absolute().top_0().left_0().size_full();
        for (i, (x, y, size)) in [
            (0.06, 39., 10.), (0.22, 112., 7.), (0.43, -18., 9.),
            (0.68, 49., 8.), (0.90, 78., 12.),
            (0.12, 275., 8.), (0.76, 355., 12.),
            (0.34, 315., 6.), (0.55, 205., 9.), (0.84, 235., 7.),
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

        // The reference design uses a two-column dashboard, not a full-height
        // Welcome hero. Keep the real native Play/picker controls on the right.
        let play = div().flex().flex_col().gap(px(12.))
            .child(self.play_group(tag, ready, cx))
            .child(self.valtic_instance_picker(cx));
        let banner = div().relative().flex_1().min_w_0()
            .h(px(if compact { 215. } else { 245. }))
            .rounded(px(20.)).overflow_hidden()
            .border_1().border_color(ui::color(0xffb8d5).opacity(0.30))
            .bg(ui::color(0x25132d))
            .children(cherry_backdrop.map(|image| {
                gpui::img(image).absolute().top_0().left_0().size_full()
                    .object_fit(gpui::ObjectFit::Cover)
            }))
            .child(div().absolute().top_0().left_0().size_full()
                .bg(gpui::linear_gradient(
                    90.,
                    gpui::linear_color_stop(ui::color(0x140d1b).opacity(0.38), 0.),
                    gpui::linear_color_stop(ui::color(0x1d0d20).opacity(0.72), 1.),
                )))
            .child(petals)
            .child(div().relative().size_full().flex().flex_col()
                .items_center().justify_center()
                .child(ui::text(if compact { 50. } else { 78. },
                    FontWeight::NORMAL, ink::FULL)
                    .font_family(theme::DISPLAY_FONT)
                    .child("Valtic"))
                .child(ui::text(14., FontWeight::MEDIUM, ink::FULL)
                    .child("MINECRAFT CLIENT"))
                .child(ui::text(12., FontWeight::NORMAL, ink::SOFT)
                    .mt(px(12.)).child("Your Minecraft experience, built for speed.")));
        let play_panel = div().w(px(if compact { 255. } else { 330. }))
            .flex_none().flex().flex_col().gap(px(14.))
            .p(px(15.)).rounded(px(20.))
            .border_1().border_color(ui::color(0xffb8d5).opacity(0.20))
            .bg(ui::color(0x241629).opacity(0.96))
            .child(play);
        div().w_full().flex().flex_wrap().gap(px(18.))
            .child(banner)
            .child(play_panel)
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
        // Keep every quick action connected to the existing native Rust page.
        let actions = div().flex().flex_col().gap(px(10.))
            .p(px(20.)).rounded(px(19.))
            .border_1().border_color(ui::color(0xf7b8d8).opacity(0.24))
            .bg(ui::color(0x1d1224).opacity(0.92))
            .child(ui::text(19., FontWeight::BOLD, ink::FULL).child("Quick Actions"))
            .child(ui::menu_row("valtic-quick-mods", "puzzle", "Mod Browser", false, &self.theme)
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.go(Page::Mods); cx.notify();
                })))
            .child(ui::menu_row("valtic-quick-settings", "settings-01", "Game Settings", false, &self.theme)
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.go(Page::Settings); cx.notify();
                })))
            .child(ui::menu_row("valtic-quick-instance", "plus-sign", "Create Instance", false, &self.theme)
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.open_instance_dialog(None); cx.notify();
                })));
        let snapshot = self.valtic_status.clone();
        let status = snapshot.as_ref().map(|s| {
            if s.online {
                format!("{} / {} players online", s.players, s.capacity)
            } else {
                "Server offline".to_string()
            }
        }).unwrap_or_else(|| "Checking server status…".to_string());
        let server = div().flex().flex_col().gap(px(12.))
            .p(px(20.)).rounded(px(19.))
            .border_1().border_color(ui::color(0xf7b8d8).opacity(0.24))
            .bg(ui::color(0x1d1224).opacity(0.92))
            .child(ui::text(19., FontWeight::BOLD, ink::FULL).child("ValticSMP"))
            .child(ui::text(13., FontWeight::MEDIUM, ink::SOFT)
                .child(valtic_status::ADDRESS))
            .child(ui::text(14., FontWeight::SEMIBOLD, ink::FULL).child(status));
        let left = div().flex_1().min_w(px(330.))
            .flex().flex_col().gap(px(18.))
            .child(self.glass_library(cx))
            .child(news)
            .children(advanced);
        let right = div().w(px(330.)).flex_none()
            .flex().flex_col().gap(px(18.))
            .child(actions).child(server);
        div().flex().flex_col().gap(px(14.))
            .child(div().w_full().flex().flex_wrap().gap(px(18.))
                .child(left).child(right))
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
