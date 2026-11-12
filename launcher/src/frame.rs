//! The glass look's frame (the mockup's `launcher-app.tsx`): the sidebar plate with the
//! instances, the launcher's update, Sculk's card, Discord and the links; the top bar with the
//! pages in a glass tab strip, the player, Microsoft sign-in and Settings; and the menus they open.
//! Dark appearance only for now (`docs/design-system.md`, "Glass").
use super::*;
use crate::glass_ui::{self as ui, white, Tone};
use crate::instances_page::java_line;
use gpui::{anchored, canvas, deferred, point};
use horizon_launcher::instances::{Icon, Instance, Kind as InstanceKind};
use horizon_theme::glass::{fill, ink, radius};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The sidebar's width and its distance from the window's edges (`w-[248px]`, `m-3.5`).
pub const SIDEBAR: f32 = 228.;
pub const MARGIN: f32 = 0.;

/// The ⋮ menu of a sidebar row while open: whose, and whether Delete… was pressed once.
#[derive(Clone, Default)]
pub struct RowMenu {
    pub open: Option<(String, bool)>,
    /// Where each row was drawn last, by instance id: the menu opens beside its row.
    pub rows: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
}

/// Records where the element it sits in is drawn, under `id`.
fn measure(rows: &Rc<RefCell<HashMap<String, Bounds<Pixels>>>>, id: String) -> impl IntoElement {
    let rows = rows.clone();
    canvas(
        move |bounds, _, _| {
            rows.borrow_mut().insert(id, bounds);
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

impl Launcher {
    /// An instance's icon (`InstanceIcon`), `size` square with no frame: Horizon's mark tile for
    /// Horizon's own instance; else its picture or the block chosen for it, round at 26 %; else
    /// its loader's or Minecraft's mark; else its initial on its colour (the accent when none was
    /// chosen), as `ColorTile` draws it.
    pub(super) fn glass_instance_icon(&mut self, i: &Instance, size: f32) -> AnyElement {
        if i.kind == InstanceKind::Default && i.icon.is_none() {
            return self.horizon_mark(size);
        }
        if let Some(picture) = self.instance_picture(i, size) {
            return instance_icons::centred(picture, size, self.scale).into_any_element();
        }
        let r = px((size * 0.26).round());
        let mut colour = self.look.accent;
        match &i.icon {
            Some(Icon::Colour(hex)) => {
                colour = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(colour);
            }
            Some(Icon::Texture(name)) => {
                if let Some(image) = self.texture_image(name, size) {
                    return gpui::img(image)
                        .flex_none()
                        .size(px(size))
                        .rounded(r)
                        .into_any_element();
                }
            }
            None => {
                if let Some(mark) = self.instance_mark(i, size) {
                    return instance_icons::centred(mark, size, self.scale).into_any_element();
                }
            }
        }
        // Bricolage at the optical size a browser would pick for the letter.
        let letter = (size * 0.46).round();
        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(size))
            .rounded(r)
            .bg(ui::color(colour))
            .font_family(if letter >= 40. {
                horizon_theme::glass::DISPLAY_FONT
            } else {
                horizon_theme::glass::HEADING_FONT
            })
            .font_weight(FontWeight::SEMIBOLD)
            .text_size(px(letter))
            .line_height(px(letter))
            .text_color(gpui::white())
            .child(instances_page::initial(&i.name))
            .into_any_element()
    }

    /// A sidebar row's state line and its dot (the mockup's `statusOf`), from what the launcher
    /// knows of the instance.
    fn row_state(&self, i: &Instance) -> (Tone, String) {
        let running =
            self.client.is_some() && self.selected_instance().is_some_and(|s| s.id == i.id);
        if running {
            return (Tone::Live, tr("Running").into());
        }
        let starting = self.starting_client.is_some()
            && self.selected_instance().is_some_and(|s| s.id == i.id);
        if starting {
            return (Tone::Busy, tr("Starting").into());
        }
        match i.kind {
            InstanceKind::Default | InstanceKind::Own => {
                let tag = self.version_of(i).cloned();
                let prepared = tag.as_deref().is_some_and(backend::is_prepared);
                match (&i.java, &tag, &i.version) {
                    (Some(java), _, _) => (Tone::Live, java_line(java)),
                    (None, None, Some(own)) => (
                        Tone::Warn,
                        trf("{version} is not installed", &[("version", own)]),
                    ),
                    (None, None, None) => (Tone::Off, tr("No version installed").into()),
                    // Following the launcher's version, which has a newer one waiting.
                    (None, Some(_), None) if self.updates.horizon.is_some() => (
                        Tone::Accent(self.look.accent_soft),
                        tr("Update available").into(),
                    ),
                    (None, Some(_), _) if !prepared => (Tone::Warn, tr("Needs preparing").into()),
                    (None, Some(tag), _) if self.native_build_of(tag).is_some() => {
                        let build = self.native_build_of(tag).unwrap_or_default();
                        (
                            Tone::Live,
                            trf(
                                "Native build · {version}",
                                &[("version", &build.horizon_short())],
                            ),
                        )
                    }
                    (None, Some(tag), Some(_)) => (Tone::Live, format!("{} · {tag}", tr("Ready"))),
                    (None, Some(_), None) => (Tone::Live, tr("Ready").into()),
                }
            }
            InstanceKind::Port => {
                let slug = &i.id["port:".len()..];
                let summary = self.packs.iter().find(|p| p.slug == slug);
                (
                    Tone::Live,
                    summary.map_or_else(|| tr("Modpack").into(), |p| p.summary()),
                )
            }
            InstanceKind::Pack => (Tone::Off, tr("From Valtic's registry").into()),
        }
    }

    /// The instances by group (`sections` in the mockup): those without a group first.
    fn glass_instances(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let t = self.theme;
        let selected = self.selected_instance().map(|i| i.id.clone());
        let on_packs = self.page == Page::Mods && self.mods.on_modpacks();
        let mut out = Vec::new();
        let mut group = None;
        let mut list = div().flex().flex_col().gap(px(4.));
        let flush = |list: Div, out: &mut Vec<AnyElement>| {
            out.push(list.into_any_element());
            div().flex().flex_col().gap(px(4.))
        };
        for i in self.instances.list.clone() {
            if i.group != group {
                list = flush(list, &mut out);
                group = i.group.clone();
                if let Some(g) = &group {
                    out.push(
                        ui::text(12., FontWeight::MEDIUM, ink::DIM)
                            .px(px(8.))
                            .pt(px(16.))
                            .pb(px(6.))
                            .child(g.clone())
                            .into_any_element(),
                    );
                }
            }
            // As in the mockup, the chosen instance stays marked on every page, unless a ported
            // pack's page is open: that row is the one shown then.
            let current = match i.kind {
                InstanceKind::Default | InstanceKind::Own => {
                    !(on_packs && self.pack_open.is_some()) && selected.as_ref() == Some(&i.id)
                }
                InstanceKind::Port => {
                    on_packs && self.pack_open.as_deref() == Some(&i.id["port:".len()..])
                }
                InstanceKind::Pack => false,
            };
            let state = self.row_state(&i);
            let icon = self.glass_instance_icon(&i, 38.);
            let clicked = i.clone();
            let open = self
                .row_menu
                .open
                .as_ref()
                .is_some_and(|(id, _)| *id == i.id);
            let menu_id = i.id.clone();
            // A right click on the row opens the same menu the ⋮ does.
            let right_id = i.id.clone();
            let more = div()
                .absolute()
                .top_0()
                .bottom_0()
                .right(px(6.))
                .flex()
                .items_center()
                .child(
                    ui::icon_button(
                        format!("side-more-{}", i.id),
                        "more-vertical",
                        32.,
                        18.,
                        radius::SMALL,
                        open,
                        &t,
                    )
                    .on_click(cx.listener(
                        move |this, event: &ClickEvent, _, cx| {
                            if this.closed_by_this_press(event) {
                                return;
                            }
                            this.row_menu.open =
                                row_menu_after_press(this.row_menu.open.take(), &menu_id);
                            cx.notify();
                        },
                    )),
                );
            list = list.child(
                div()
                    .relative()
                    .child(measure(&self.row_menu.rows, i.id.clone()))
                    .child(
                        ui::row(format!("side-instance-{}", i.id), current, 40., &t)
                            .child(div().relative().flex_none().child(icon))
                            .child(ui::row_text(i.name.clone(), state))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                match clicked.kind {
                                    InstanceKind::Default | InstanceKind::Own => {
                                        this.select_instance(&clicked);
                                        this.go(Page::Play);
                                    }
                                    InstanceKind::Port => {
                                        this.show_modpack(Some(clicked.id["port:".len()..].into()))
                                    }
                                    InstanceKind::Pack => this.show_modpack(None),
                                }
                                cx.notify();
                            }))
                            // The ⋮ menu, from a right click anywhere on the row.
                            .on_mouse_down(
                                gpui::MouseButton::Right,
                                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                    // The open menu's mouse-down-out already closed it on this
                                    // press: reopening here would make the toggle a no-op.
                                    if this.menu_closed_at.take() == Some(event.position) {
                                        cx.notify();
                                        return;
                                    }
                                    this.row_menu.open =
                                        row_menu_after_press(this.row_menu.open.take(), &right_id);
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(more),
            );
        }
        out.push(list.into_any_element());
        out
    }

    /// Valtic's custom navigation rail, not the upstream instance explorer.
    /// Instances are still available under the Home library and Versions pages.
    pub(super) fn glass_sidebar(&mut self, _window: &Window, cx: &mut Context<Self>) -> Div {
        let theme = self.theme;
        let mut nav = div()
            .flex().flex_col().gap(px(7.))
            .px(px(16.))
            .pt(px(22.));

        for (page, label, symbol) in [
            (Page::Play, "Home", "home"),
            (Page::Mods, "Mods", "package"),
            (Page::Versions, "Versions", "archive"),
            (Page::Servers, "Servers", "server"),
            (Page::Settings, "Settings", "sliders-horizontal"),
        ] {
            let active = self.page == page;
            nav = nav.child(
                div().id(SharedString::from(format!("valtic-rail-{}",label)))
                    .flex().items_center().gap(px(15.))
                    .h(px(52.)).px(px(19.))
                    .rounded(px(15.))
                    .border_1()
                    .border_color(if active { ui::color(0xf6accd).opacity(0.38) }
                        else { ui::color(0xffffff).opacity(0.) })
                    .bg(if active { ui::color(0xb44c82).opacity(0.92) }
                        else { ui::color(0x241728).opacity(0.) })
                    .hover(|d| d.bg(ui::color(0x663352).opacity(0.92)))
                    .cursor_pointer()
                    .child(ui::icon(symbol, 21., gpui::white()))
                    .child(ui::text(16., FontWeight::SEMIBOLD, ink::FULL).child(label))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.go(page);
                        cx.notify();
                    })),
            );
        }
        if self.wardrobe_shown() {
            let active = self.page == Page::Wardrobe;
            nav = nav.child(
                div().id("valtic-rail-skins")
                    .flex().items_center().gap(px(15.))
                    .h(px(52.)).px(px(19.))
                    .rounded(px(15.))
                    .bg(if active { ui::color(0xb44c82).opacity(0.92) }
                        else { ui::color(0x241728).opacity(0.) })
                    .hover(|d| d.bg(ui::color(0x663352).opacity(0.92)))
                    .cursor_pointer()
                    .child(ui::icon("user", 21., gpui::white()))
                    .child(ui::text(16., FontWeight::SEMIBOLD, ink::FULL).child("Skins"))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.go(Page::Wardrobe);
                        cx.notify();
                    })),
            );
        }

        let brand = div().flex().flex_col().items_center()
            .gap(px(2.))
            .pt(px(23.)).pb(px(18.))
            .child(gpui::svg()
                .path("valtic/cherry-emblem.svg").size(px(90.)))
            .child(ui::text(31., FontWeight::BOLD, ink::FULL)
                .font_family(theme::DISPLAY_FONT).child("Valtic"))
            .child(ui::text(15., FontWeight::SEMIBOLD, ink::STRONG)
                .child("✿  LAUNCHER  ✿"));

        let face = self.accounts.selected().map(|a| a.uuid.clone());
        let (account_name, state_label) = self.who_plays();
        let account = div()
            .id("valtic-sidebar-account")
            .flex().items_center().gap(px(12.))
            .min_h(px(67.))
            .px(px(12.))
            .rounded(px(15.))
            .border_1().border_color(ui::color(0xf4a1c6).opacity(0.32))
            .bg(ui::color(0x3f2841).opacity(0.96))
            .cursor_pointer()
            .hover(|d| d.bg(ui::color(0x603b5b)))
            .child(
                div().size(px(40.)).rounded(px(9.)).overflow_hidden()
                    .child(self.profile_heads.draw(face.as_deref(), 40., self.scale, &theme))
            )
            .child(div().flex().flex_col().flex_1().gap(px(3.))
                .child(ui::text(11., FontWeight::SEMIBOLD, ink::STRONG)
                    .child(if face.is_some() { "●  MICROSOFT ACCOUNT" }
                        else { "○  NOT SIGNED IN" }))
                .child(ui::text(15., FontWeight::BOLD, ink::FULL)
                    .truncate().child(account_name))
                .child(ui::text(11., FontWeight::NORMAL, ink::NOTE)
                    .truncate().child(state_label)))
            .child(ui::icon("arrow-down-01", 17., white(ink::SOFT)))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                if this.accounts.selected().is_none() {
                    this.start_sign_in();
                } else {
                    this.go(Page::Settings);
                }
                cx.notify();
            }));

        let mut footer = div()
            .flex().flex_col().gap(px(12.))
            .px(px(16.)).pb(px(20.))
            .child(account);

        if matches!(self.self_update, SelfUpdate::Ready(_)) {
            footer = footer.child(
                div().id("valtic-sidebar-updater")
                    .flex().justify_center().items_center().h(px(38.))
                    .rounded(px(10.))
                    .bg(ui::color(0xd65e93))
                    .cursor_pointer()
                    .child(ui::text(13., FontWeight::SEMIBOLD, ink::FULL)
                        .child("New launcher update  ↗"))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.apply_launcher_update(cx);
                        cx.notify();
                    }))
            );
        }

        div()
            .flex().flex_col().flex_none()
            .w(px(SIDEBAR)).h_full()
            .bg(ui::color(0x27192c).opacity(0.96))
            .border_r_1()
            .border_color(ui::color(0xf6b4d3).opacity(0.16))
            .child(brand)
            .child(nav)
            .child(div().flex_1())
            .child(footer)
    }

    /// The glass top bar: the pages in a glass strip, then the player, Microsoft sign-in and
    /// Settings (`px-8 pt-3.5`).
    pub(super) fn glass_topbar(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        // Valtic keeps all navigation in the left rail, like Modrinth's app.
        // The top bar tells the player where they are instead of duplicating tabs.
        let compact = self.window_width < crate::glass_home::COMPACT;
        let page_title = match self.page {
            Page::Play => "Home",
            Page::Mods => "Mods",
            Page::Servers => "Servers",
            Page::Versions => "Versions",
            Page::Wardrobe => "Skins",
            Page::Shop => "Shop",
            Page::Settings => "Settings",
        };
        let strip = div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(3.))
            .child(ui::text(11., FontWeight::SEMIBOLD, ink::DIM).child("VALTIC"))
            .child(ui::text(20., FontWeight::BOLD, ink::FULL).child(page_title));

        let (name, how) = self.who_plays();
        let face = self.accounts.selected().map(|a| a.uuid.clone());
        let open = self.account_menu;
        let chip =
            div()
                .id("account")
                .relative()
                .flex()
                .items_center()
                .gap(px(10.))
                .h(px(44.))
                .pl(px(8.))
                .pr(px(12.))
                .rounded(px(radius::ROW))
                .bg(white(if open { fill::ACTIVE } else { fill::ROW_HOVER }))
                .when(!open, |d| d.hover(|s| s.bg(white(0.09))))
                .cursor_pointer()
                .child(
                    div().size(px(26.)).rounded(px(7.)).overflow_hidden().child(
                        self.profile_heads
                            .draw(face.as_deref(), 26., self.scale, &t),
                    ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .max_w(px(if compact { 100. } else { 150. }))
                        .child(
                            ui::text(14., FontWeight::SEMIBOLD, ink::FULL)
                                .line_height(px(17.))
                                .truncate()
                                .child(name),
                        )
                        .children((!compact).then(|| {
                            ui::text(12., FontWeight::NORMAL, ink::QUIET)
                                .line_height(px(15.))
                                .child(how)
                        })),
                )
                .child(
                    ui::icon("arrow-down-01", 16., white(ink::FAINT))
                        .ml(px(4.))
                        .with_transformation(gpui::Transformation::rotate(gpui::radians(
                            if open { std::f32::consts::PI } else { 0. },
                        ))),
                )
                .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                    if this.closed_by_this_press(event) {
                        return;
                    }
                    this.account_menu = !this.account_menu;
                    if this.account_menu {
                        this.accounts = Accounts::load();
                        this.load_profile_heads();
                    }
                    cx.notify();
                }));
        let account = div()
            .relative()
            .child(keyboard(chip, &t))
            .children(self.glass_account_menu(cx));

        // Signing in with Microsoft, while nobody is: the mockup's button.
        let sign_in = self.accounts.selected().is_none().then(|| {
            keyboard(
                div()
                    .id("topbar-sign-in")
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .h(px(44.))
                    .px(px(16.))
                    .rounded(px(radius::ROW))
                    .bg(white(fill::BUTTON))
                    .hover(|s| s.bg(white(fill::BUTTON_HOVER)))
                    .cursor_pointer()
                    .child(microsoft::logo(16.))
                    .child(
                        ui::text(14., FontWeight::MEDIUM, ink::FULL)
                            .child(tr("Sign in with Microsoft")),
                    )
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.start_sign_in();
                        cx.notify();
                    })),
                &t,
            )
        });
        let settings = ui::icon_button(
            "nav-Settings",
            "sliders-horizontal",
            40.,
            19.,
            radius::CONTROL,
            self.page == Page::Settings,
            &t,
        )
        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
            this.go(if this.page == Page::Settings {
                Page::Play
            } else {
                Page::Settings
            });
            cx.notify();
        }));
        // The last native build's log (not in the mockup, kept), as a quiet button.
        let build_log = self.native_log.available.then(|| {
            keyboard(
                div()
                    .id("topbar-native-log")
                    .flex()
                    .items_center()
                    .h(px(36.))
                    .px(px(14.))
                    .rounded(px(radius::CONTROL))
                    .bg(white(fill::BUTTON))
                    .hover(|s| s.bg(white(fill::BUTTON_HOVER)))
                    .cursor_pointer()
                    .child(ui::text(14., FontWeight::MEDIUM, ink::FULL).child(tr("Build log")))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.native_log_open();
                        cx.notify();
                    })),
                &t,
            )
        });
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(12.))
            .px(px(32.))
            .pt(px(14.))
            .children((self.page != Page::Play).then_some(strip))
            .children(self.toast_in_topbar())
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(6.))
                    .children(build_log)
                    .children(self.accounts.selected().is_some().then_some(account))
                    .children(sign_in)
                    .child(settings),
            )
    }

    /// The menu under the player (`Switch player`): everyone who can play, then Manage accounts.
    fn glass_account_menu(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.account_menu {
            return None;
        }
        let t = self.theme;
        let soft = self.look.accent_soft;
        let now = self.accounts.selected().map(|a| a.uuid.clone());
        let mut menu = ui::popover(250.)
            .id("account-menu")
            .occlude()
            .on_mouse_down_out(cx.listener(|this, event: &MouseDownEvent, _, cx| {
                this.account_menu = false;
                this.menu_closed_at = Some(event.position);
                cx.notify();
            }));
        for (i, (who, name, how)) in self.players().into_iter().enumerate() {
            let chosen = Some(&who) == now.as_ref();
            let id = SharedString::from(format!("account-pick-{i}"));
            menu = menu.child(keyboard(
                track_hover(
                    div()
                        .id(id.clone())
                        .relative()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .h(px(48.))
                        .px(px(10.))
                        .rounded(px(radius::CONTROL))
                        .cursor_pointer()
                        .child(hover_layer(&id, white(0.07), radius::CONTROL, 0.))
                        .child(
                            div()
                                .relative()
                                .size(px(30.))
                                .rounded(px(8.))
                                .overflow_hidden()
                                .child(self.profile_heads.draw(
                                    Some(who.as_str()),
                                    30.,
                                    self.scale,
                                    &t,
                                )),
                        )
                        .child(
                            div()
                                .relative()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    ui::text(14., FontWeight::MEDIUM, ink::FULL)
                                        .truncate()
                                        .child(name),
                                )
                                .child(ui::text(12., FontWeight::NORMAL, ink::FAINT).child(how)),
                        )
                        .when(chosen, |d| {
                            d.child(ui::icon("tick-02", 16., ui::color(soft)).relative())
                        })
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.play_as(&who);
                            cx.notify();
                        })),
                    id,
                ),
                &t,
            ));
        }
        menu = menu.child(div().mt(px(4.)).child(
            ui::menu_row("account-manage", "user", tr("Manage accounts"), false, &t).on_click(
                cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.account_menu = false;
                    this.go(Page::Settings);
                    cx.notify();
                }),
            ),
        ));
        Some(deferred(
            div().absolute().top(px(52.)).right_0().child(menu),
        ))
    }

    /// The ⋮ menu of a sidebar row, beside it: Edit…, Duplicate…, Open folder, Export…,
    /// launch settings, the desktop shortcut, then Delete… (asked twice). Each acts on that instance.
    pub(super) fn row_menu(
        &mut self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let (id, confirm) = self.row_menu.open.clone()?;
        let Some(instance) = self.instances.list.iter().find(|i| i.id == id).cloned() else {
            self.row_menu.open = None;
            return None;
        };
        let row = self.row_menu.rows.borrow().get(&id).copied()?;
        let t = self.theme;
        let viewport = window.viewport_size();
        let height = f32::from(viewport.height);
        // The mockup's `rect.right + 12`, `min(rect.top - 8, innerHeight - 330)`: its ⋮ button
        // ends 6 px inside the row and sits in the row's middle.
        let x = f32::from(row.origin.x + row.size.width) - 6. + 12.;
        let button_top = f32::from(row.origin.y) + (f32::from(row.size.height) - 32.) / 2.;
        let y = (button_top - 8.).min(height - 330.).max(8.);
        let item = |key: &'static str, icon: &'static str, label: String| {
            ui::menu_row(format!("row-menu-{key}"), icon, label, false, &t)
        };
        let pick = |this: &mut Launcher, i: &Instance| {
            this.row_menu.open = None;
            this.select_instance(i);
            this.go(Page::Play);
        };
        let (edit, copy, export, launch, shortcut) = (
            instance.clone(),
            instance.clone(),
            instance.clone(),
            instance.clone(),
            instance.clone(),
        );
        let home = instance.home.clone();
        let labels = [
            tr("Edit…"),
            tr("Duplicate…"),
            tr("Open folder"),
            tr("Export…"),
            tr("Launch…"),
            if self.has_shortcut(&instance.id) {
                tr("Remove the desktop shortcut")
            } else {
                tr("Create a desktop shortcut")
            },
            if confirm {
                tr("Click again to delete")
            } else {
                tr("Delete…")
            },
        ];
        let [edit_label, copy_label, folder_label, export_label, launch_label, shortcut_label, delete_label] =
            labels;
        let viewport_width = f32::from(viewport.width);
        let width = ui::menu_content_width(window, &labels)
            .max(250.)
            .min((viewport_width - 16.).max(0.));
        let x = x.min(viewport_width - width - 8.).max(8.);
        let mut menu = ui::popover(width)
            .id("row-menu")
            .occlude()
            .on_mouse_down_out(cx.listener(|this, event: &MouseDownEvent, _, cx| {
                this.row_menu.open = None;
                this.menu_closed_at = Some(event.position);
                cx.notify();
            }))
            .child(
                item("edit", "pencil-edit-02", edit_label.into()).on_click(cx.listener(
                    move |this, _: &ClickEvent, _, cx| {
                        this.row_menu.open = None;
                        this.open_instance_dialog(Some(edit.clone()));
                        cx.notify();
                    },
                )),
            )
            .child(
                item("duplicate", "copy-01", copy_label.into()).on_click(cx.listener(
                    move |this, _: &ClickEvent, _, cx| {
                        pick(this, &copy);
                        this.start_copy();
                        cx.notify();
                    },
                )),
            )
            .child(
                item("folder", "folder-01", folder_label.into()).on_click(cx.listener(
                    move |this, _: &ClickEvent, _, cx| {
                        this.row_menu.open = None;
                        let _ = std::fs::create_dir_all(&home);
                        crate::glass_toast::open(cx, &home);
                        cx.notify();
                    },
                )),
            )
            .child(
                item("export", "upload-04", export_label.into()).on_click(cx.listener(
                    move |this, _: &ClickEvent, _, cx| {
                        pick(this, &export);
                        if this.export.is_none() {
                            this.toggle_export();
                        }
                        cx.notify();
                    },
                )),
            )
            .child(
                item("launch", "sliders-horizontal", launch_label.into()).on_click(cx.listener(
                    move |this, _: &ClickEvent, _, cx| {
                        pick(this, &launch);
                        this.home_tab = home_tabs::HomeTab::Overview;
                        // A panel left open for another instance must not turn this into Close.
                        this.launch = None;
                        this.toggle_launch();
                        cx.notify();
                    },
                )),
            )
            .child(
                item("shortcut", "link-01", shortcut_label.into()).on_click(cx.listener(
                    move |this, _: &ClickEvent, _, cx| {
                        this.row_menu.open = None;
                        this.toggle_shortcut(shortcut.id.clone(), shortcut.name.clone());
                        cx.notify();
                    },
                )),
            );
        // Horizon's own instance can't be deleted, nor one while a game runs.
        let can_delete = instance.kind != InstanceKind::Default
            && self.client.is_none()
            && self.starting_client.is_none();
        let gone = instance.id.clone();
        let delete = ui::menu_row("row-menu-delete", "delete-02", delete_label, true, &t);
        let delete = if can_delete {
            delete.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                if let Some((_, confirm)) = this.row_menu.open.as_mut().filter(|(_, c)| !*c) {
                    *confirm = true;
                } else {
                    this.row_menu.open = None;
                    this.delete_instance(&gone);
                }
                cx.notify();
            }))
        } else {
            delete.opacity(0.4).cursor_default()
        };
        menu = menu.child(div().mt(px(4.)).child(delete));
        Some(
            deferred(anchored().position(point(px(x), px(y))).child(menu))
                .with_priority(1)
                .into_any_element(),
        )
    }
}
