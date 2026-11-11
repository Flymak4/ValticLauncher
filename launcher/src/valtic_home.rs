//! Valtic's own streamlined launcher dashboard. Official Minecraft release
//! notes and local Minecraft game textures replace generic or synthetic art.
use super::*;
use crate::glass_ui::{self as ui, white};
use horizon_theme::glass::{ink, radius};

impl Launcher {
    /// The cover is Minecraft's REAL title panorama for the selected build,
    /// read from the game's local assets, never a generated or mismatched image.
    pub(super) fn valtic_cover(&mut self, tag: &str, size: f32) -> AnyElement {
        let image = if tag.is_empty() {
            None
        } else {
            let panorama = backend::resources(tag)
                .join("assets/minecraft/textures/gui/title/background/panorama_0.png");
            self.icon_image(format!("valtic:cover:{tag}"), size, false, || {
                std::fs::read(&panorama).ok()
            })
        };
        let plate = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(size))
            .rounded(px(radius::PANEL))
            .overflow_hidden()
            .bg(ui::color(0x24362F));
        match image {
            Some(png) => plate
                .child(gpui::img(png).size(px(size)).object_fit(gpui::ObjectFit::Cover))
                .into_any_element(),
            None => plate.child(self.horizon_mark(size * 0.42)).into_any_element(),
        }
    }

    /// Main Play surface: one game, its true Minecraft version, Play, settings.
    pub(super) fn valtic_home_head(
        &mut self, tag: &str, prepared: bool, cx: &mut Context<Self>,
    ) -> Div {
        let selected = self.selected_instance().cloned();
        let java_version = selected.as_ref()
            .and_then(|i| i.java.as_ref().map(|j| j.minecraft.clone()));
        let java = java_version.is_some();
        let version = java_version.unwrap_or_else(|| backend::game_version(tag));
        let title = format!("Valtic {version}");
        let instance_name = selected.as_ref().map(|i| i.name.clone())
            .unwrap_or_else(|| "My Valtic".into());
        let ready = prepared || java;
        let small = self.window_width < 1080.;
        let cover = self.valtic_cover(tag, 192.);
        let picker = (!java).then(|| self.glass_picker(cx));
        let edit = selected.clone();
        let content = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap(px(15.))
            .child(ui::text(12., FontWeight::SEMIBOLD, ink::NOTE).child("YOUR GAME"))
            .child(ui::text(38., FontWeight::BOLD, ink::FULL)
                .line_height(px(44.)).child(title))
            .child(ui::text(15., FontWeight::NORMAL, ink::BODY).child(instance_name))
            .child(ui::text(13., FontWeight::NORMAL, ink::NOTE).child(
                if ready { "Ready to play" } else { "Game files will be prepared on first launch" }
            ))
            .child(div().flex().flex_wrap().gap(px(10.)).mt(px(12.))
                .child(self.play_group(tag, ready, cx))
                .children(picker))
            .children(edit.map(|instance| {
                ui::quiet("valtic-edit-instance", "Instance settings", 32., &self.theme)
                    .mt(px(6.))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.open_instance_dialog(Some(instance.clone()));
                        cx.notify();
                    }))
            }));
        let hero = div()
            .flex()
            .gap(px(30.))
            .p(px(28.))
            .w_full()
            .rounded(px(18.))
            .bg(ui::color(0x171d26))
            .when(small, |d| d.flex_col())
            .child(content)
            .child(div().flex_none().child(cover));
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .pt(px(14.))
            .child(div().flex().justify_between().items_center().pb(px(16.))
                .child(ui::text(28., FontWeight::BOLD, ink::FULL).child("Ready to play"))
                .child(ui::text(12., FontWeight::MEDIUM, ink::NOTE)
                    .child("MINECRAFT JAVA EDITION")))
            .child(hero)
    }

    /// Mojang's news for the *active* game version. Other versions do not
    /// inherit Minecraft 26.3 artwork or a misleading "latest" label.
    pub(super) fn valtic_home_bottom(
        &mut self, rows: Div, cx: &mut Context<Self>,
    ) -> Div {
        let tag = self.selected().cloned().unwrap_or_default();
        let version = self.selected_instance()
            .and_then(|i| i.java.as_ref().map(|j| j.minecraft.clone()))
            .unwrap_or_else(|| backend::game_version(&tag));
        let v263 = version == "26.3";
        let url = if v263 {
            "https://www.minecraft.net/en-us/article/minecraft-java-edition-26-3"
        } else {
            "https://www.minecraft.net/en-us/updates/minecraft-updates-timeline-and-evolution"
        };
        let news = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .pt(px(22.))
            .child(ui::text(22., FontWeight::BOLD, ink::FULL).child("Latest Minecraft news"))
            .child(div()
                .id("valtic-news-release")
                .flex()
                .flex_col()
                .items_start()
                .gap(px(10.))
                .w_full()
                .p(px(22.))
                .rounded(px(16.))
                .bg(ui::color(0x181e27))
                .hover(|d| d.bg(ui::color(0x222a34)))
                .cursor_pointer()
                .child(ui::text(12., FontWeight::MEDIUM, ink::NOTE)
                    .child(format!("MINECRAFT {version}  /  {}", if v263 { "WILDERNESS BOUND" } else { "RELEASE NOTES" })))
                .child(ui::text(20., FontWeight::SEMIBOLD, ink::FULL)
                    .child(if v263 { "Explore the Dappled Forest" } else { "What's new in Minecraft" }))
                .child(ui::text(14., FontWeight::NORMAL, ink::BODY).child(
                    if v263 {
                        "Poplar forests, abandoned camps and new building blocks arrive in Minecraft Java 26.3."
                    } else {
                        "Discover the official changes and latest news for Minecraft."
                    }
                ))
                .child(ui::text(13., FontWeight::MEDIUM, ink::STRONG)
                    .child("Read official release notes  →"))
                .on_click(move |_, _, cx| cx.open_url(url)));
        let mut shortcuts = div().flex().flex_wrap().gap(px(10.)).mt(px(20.));
        for (label, tab) in [
            ("My worlds", home_tabs::HomeTab::Worlds),
            ("Instance mods", home_tabs::HomeTab::Mods),
        ] {
            shortcuts = shortcuts.child(ui::quiet(
                format!("valtic-shortcut-{label}"), label, 38., &self.theme,
            ).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.home_tab = tab;
                cx.notify();
            })));
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
}
