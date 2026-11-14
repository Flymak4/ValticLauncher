//! Valtic's Minecraft Java skin library and cape selector.
//! Only verified account-owned capes are offered; skin changes use Minecraft Services.
use super::*;
use gpui::{img, RenderImage};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SKIN_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
const CAPE_URL: &str = "https://api.minecraftservices.com/minecraft/profile/capes/active";
const PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const MAX_SKIN_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Debug)]
pub(super) struct Cape {
    id: String,
    name: String,
    active: bool,
}

fn skins_dir() -> PathBuf {
    backend::data_dir().join("saved-skins")
}

fn verify_skin(path: &Path) -> Result<Vec<u8>, String> {
    let size = std::fs::metadata(path).map_err(|e| format!("Cannot read skin: {e}"))?.len();
    if size > MAX_SKIN_BYTES {
        return Err("Skin PNG must be smaller than 2 MB.".into());
    }
    let png = std::fs::read(path).map_err(|e| format!("Cannot open skin: {e}"))?;
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("Choose a PNG skin, not a renamed image.".into());
    }
    let image = image::load_from_memory(&png)
        .map_err(|_| "The selected skin PNG is invalid.".to_string())?;
    if image.width() != 64 || !matches!(image.height(), 32 | 64) {
        return Err("Minecraft skins must be 64x64 or legacy 64x32 pixels.".into());
    }
    Ok(png)
}

fn save_skin(path: &Path) -> Result<PathBuf, String> {
    verify_skin(path)?;
    let folder = skins_dir();
    std::fs::create_dir_all(&folder).map_err(|e| format!("Cannot create skin library: {e}"))?;
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("My skin");
    let stem: String = stem.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
        .take(38).collect();
    let stem = if stem.trim().is_empty() { "My skin" } else { stem.trim() };
    let mut dest = folder.join(format!("{stem}.png"));
    for n in 2..=999 {
        if !dest.exists() { break; }
        dest = folder.join(format!("{stem} {n}.png"));
    }
    if dest.exists() { return Err("The skin library is full.".into()); }
    std::fs::copy(path, &dest).map_err(|e| format!("Cannot save the skin: {e}"))?;
    Ok(dest)
}

fn saved_skins() -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(skins_dir()) else { return Vec::new() };
    let mut files: Vec<_> = entries.filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension()
            .and_then(|e| e.to_str()).is_some_and(|s| s.eq_ignore_ascii_case("png")))
        .take(100).collect();
    files.sort();
    files
}

fn slim_for(path: &Path) -> bool {
    std::fs::read_to_string(path.with_extension("model"))
        .is_ok_and(|s| s.trim() == "slim")
}

fn upload_skin(path: &Path, slim: bool) -> Result<(), String> {
    let png = verify_skin(path)?;
    let player = account::java_player()?;
    let boundary = format!("valtic-skin-{}", std::process::id());
    let variant = if slim { "slim" } else { "classic" };
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"variant\"\r\n\r\n{variant}\r\n"
    ).into_bytes();
    body.extend_from_slice(format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"skin.png\"\r\nContent-Type: image/png\r\n\r\n"
    ).as_bytes());
    body.extend_from_slice(&png);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    match ureq::post(SKIN_URL)
        .set("Authorization", &format!("Bearer {}", player.token))
        .set("Content-Type", &format!("multipart/form-data; boundary={boundary}"))
        .send_bytes(&body)
    {
        Ok(response) if (200..300).contains(&response.status()) => Ok(()),
        Ok(response) => Err(format!("Minecraft rejected this skin (HTTP {}).", response.status())),
        Err(ureq::Error::Status(401 | 403, _)) =>
            Err("Minecraft account authorization failed. Sign in again.".into()),
        Err(ureq::Error::Status(status, _)) =>
            Err(format!("Minecraft rejected the skin (HTTP {status}).")),
        Err(ureq::Error::Transport(e)) =>
            Err(format!("Cannot reach Minecraft Services: {e}")),
    }
}

/// Changing the active skin's arm model must update Minecraft too, not just the preview.
fn set_current_model(url: &str, slim: bool) -> Result<(), String> {
    if !url.starts_with("https://textures.minecraft.net/")
        && !url.starts_with("http://textures.minecraft.net/") {
        return Err("The current skin does not have a valid Minecraft texture URL.".into());
    }
    let player = account::java_player()?;
    let variant = if slim { "slim" } else { "classic" };
    ureq::post(SKIN_URL)
        .set("Authorization", &format!("Bearer {}", player.token))
        .set("Content-Type", "application/json")
        .send_json(serde_json::json!({"url":url,"variant":variant}))
        .map_err(|e| format!("Could not change the skin model: {e}"))?;
    Ok(())
}

fn load_capes() -> Result<Vec<Cape>, String> {
    let player = account::java_player()?;
    let response = ureq::get(PROFILE_URL)
        .set("Authorization", &format!("Bearer {}", player.token))
        .call().map_err(|e| format!("Could not load Minecraft capes: {e}"))?;
    let profile: serde_json::Value = response.into_json()
        .map_err(|e| format!("Could not read the Minecraft cape list: {e}"))?;
    let capes = profile.get("capes").and_then(|v| v.as_array())
        .ok_or("Minecraft did not return a list of capes.")?;
    Ok(capes.iter().filter_map(|v| {
        Some(Cape {
            id: v.get("id")?.as_str()?.to_owned(),
            name: v.get("alias").and_then(|s| s.as_str())
                .unwrap_or("Minecraft cape").replace('_', " "),
            active: v.get("state").and_then(|s| s.as_str()) == Some("ACTIVE"),
        })
    }).collect())
}

fn equip_cape(cape: Option<&str>) -> Result<(), String> {
    let player = account::java_player()?;
    let auth = format!("Bearer {}", player.token);
    let result = match cape {
        Some(id) => ureq::put(CAPE_URL)
            .set("Authorization", &auth)
            .set("Content-Type", "application/json")
            .send_json(serde_json::json!({"capeId": id})),
        None => ureq::delete(CAPE_URL).set("Authorization", &auth).call(),
    };
    result.map_err(|e| format!("Minecraft could not update the cape: {e}"))?;
    Ok(())
}

/// A pixel-sharp skin face for cards. Draws the outer hat layer when it has pixels.
fn skin_face(path: &Path) -> Option<Arc<RenderImage>> {
    let bytes = verify_skin(path).ok()?;
    let picture = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let pixels: Vec<[u8; 4]> = (0..8).flat_map(|y| (0..8).map(move |x| (x, y)))
        .map(|(x, y)| {
            let mut color = picture.get_pixel(8 + x, 8 + y).0;
            if picture.height() == 64 {
                let hat = picture.get_pixel(40 + x, 8 + y).0;
                if hat[3] > 0 { color = hat; }
            }
            color
        }).collect();
    Some(art::bake(&pixels, 8, 8, 12., 1.))
}

impl Launcher {
    /// The existing wardrobe renderer shows a full-body player, not a flat skin thumbnail.
    fn skin_preview_dress(&mut self, cx: &mut Context<Self>)
        -> Option<gpui::Entity<wardrobe_page::viewer::Viewer>>
    {
        let account = self.accounts.selected()?.clone();
        let id = self.skin_editing.clone().unwrap_or_else(|| "current".into());
        let slim = if self.skin_editing.is_some() {
            self.skin_slim
        } else {
            account.skin.as_ref().is_some_and(|(_, m)| m.eq_ignore_ascii_case("slim"))
        };
        let key = (if id == "current" {
            format!("current:{}:{:?}", account.uuid, account.skin)
        } else {
            id.clone()
        }, slim);
        let preview = self.skin_preview
            .get_or_insert_with(|| cx.new(|_| wardrobe_page::viewer::Viewer::sized(260., 360.)))
            .clone();
        if self.skin_preview_for.as_ref() != Some(&key) {
            let texture = if id == "current" {
                wardrobe_page::skin(&account, &self.game_resources()).map(|(skin, _)| skin)
            } else {
                std::fs::read(&id).ok()
                    .and_then(|png| wardrobe_page::skin_texture(&png))
            }.unwrap_or_else(wardrobe_page::plain_skin);
            let outfit = wardrobe_page::viewer::Outfit {
                skin: texture,
                slim,
                items: Vec::new(),
            };
            self.skin_preview_for = Some(key);
            let to = preview.clone();
            cx.defer(move |cx| to.update(cx, |viewer, cx| viewer.dress(outfit, false, cx)));
        }
        Some(preview)
    }

    fn skin_preview_drag(cx: &mut Context<Self>) -> impl Fn(bool, &mut App) + Clone + 'static {
        let page = cx.entity().downgrade();
        move |dragging, cx: &mut App| {
            let _ = page.update(cx, |this, cx| {
                if this.skin_preview_dragging != dragging {
                    this.skin_preview_dragging = dragging;
                    cx.notify();
                }
            });
        }
    }

    pub(super) fn choose_minecraft_skin(&mut self, cx: &mut Context<Self>) {
        if self.skin_uploading { return; }
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true, directories: false, multiple: false,
            prompt: Some("Add Minecraft skin PNG to Valtic".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else { return; };
            let Some(path) = paths.into_iter().next() else { return; };
            let _ = this.update(cx, |this, cx| {
                this.skin_uploading = true;
                this.skin_status = Some("Adding skin to your library…".into());
                cx.notify();
            });
            let result = cx.background_executor().spawn(async move { save_skin(&path) }).await;
            let _ = this.update(cx, |this, cx| {
                this.skin_uploading = false;
                match result {
                    Ok(path) => {
                        this.skin_editing = Some(path.to_string_lossy().into_owned());
                        this.skin_slim = false;
                        this.skin_preview_for = None;
                        this.skin_status = Some("Skin saved. Choose Wide or Slim, then Apply.".into());
                    }
                    Err(e) => this.skin_status = Some(e),
                }
                cx.notify();
            });
        }).detach();
    }

    fn skin_open_editor(&mut self, id: String, cx: &mut Context<Self>) {
        self.skin_slim = if id == "current" {
            self.accounts.selected().and_then(|a| a.skin.as_ref())
                .is_some_and(|(_, model)| model.eq_ignore_ascii_case("slim"))
        } else {
            slim_for(Path::new(&id))
        };
        self.skin_editing = Some(id);
        self.skin_status = None;
        cx.notify();
    }

    fn skin_apply(&mut self, cx: &mut Context<Self>) {
        if self.skin_uploading { return; }
        let Some(id) = self.skin_editing.clone() else { return; };
        let current = self.accounts.selected().and_then(|a| a.skin.as_ref())
            .map(|(url, _)| url.clone());
        let slim = self.skin_slim;
        self.skin_uploading = true;
        self.skin_status = Some("Applying your skin to Minecraft…".into());
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move {
                if id == "current" {
                    let url = current.ok_or("No custom skin is currently equipped. Add a PNG instead.")?;
                    set_current_model(&url, slim)
                } else {
                    let path = Path::new(&id);
                    upload_skin(path, slim)?;
                    let _ = std::fs::write(path.with_extension("model"), if slim { "slim" } else { "wide" });
                    Ok(())
                }
            }).await;
            let _ = this.update(cx, |this, cx| {
                this.skin_uploading = false;
                match result {
                    Ok(()) => {
                        this.skin_status = Some("Skin applied! Rejoin Minecraft to see it.".into());
                        this.skin_preview_for = None;
                        this.load_auth_statuses(true);
                    }
                    Err(e) => this.skin_status = Some(e),
                }
                cx.notify();
            });
        }).detach();
    }

    fn skin_refresh_capes(&mut self, cx: &mut Context<Self>) {
        if self.skin_cape_busy { return; }
        let Some(uuid) = self.accounts.selected().map(|a| a.uuid.clone()) else { return; };
        self.skin_cape_busy = true;
        cx.spawn(async move |this, cx| {
            let capes = cx.background_executor().spawn(async move { load_capes() }).await;
            let _ = this.update(cx, |this, cx| {
                if this.accounts.selected().is_some_and(|a| a.uuid == uuid) {
                    this.skin_cape_busy = false;
                    this.skin_capes_loaded = true;
                    match capes {
                        Ok(capes) => this.skin_capes = capes,
                        Err(e) => this.skin_status = Some(e),
                    }
                    cx.notify();
                }
            });
        }).detach();
    }

    fn skin_equip_cape(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        if self.skin_cape_busy { return; }
        let Some(uuid) = self.accounts.selected().map(|a| a.uuid.clone()) else { return; };
        self.skin_cape_busy = true;
        self.skin_status = Some("Updating your Minecraft cape…".into());
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move {
                equip_cape(id.as_deref())?;
                load_capes()
            }).await;
            let _ = this.update(cx, |this, cx| {
                if this.accounts.selected().is_some_and(|a| a.uuid == uuid) {
                    this.skin_cape_busy = false;
                    match result {
                        Ok(capes) => {
                            this.skin_capes = capes;
                            this.skin_status = Some("Cape updated! Rejoin Minecraft to see it.".into());
                            this.load_auth_statuses(true);
                        }
                        Err(e) => this.skin_status = Some(e),
                    }
                    cx.notify();
                }
            });
        }).detach();
    }

    fn skin_card(&self, id: String, name: String, image: Option<Arc<RenderImage>>, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let hovered = self.skin_hovered.as_deref() == Some(id.as_str());
        let editing = self.skin_editing.as_deref() == Some(id.as_str());
        let key = id.clone();
        let click_key = id.clone();
        let preview = div().flex().justify_center().items_center().h(px(124.))
            .child(match image {
                Some(face) => img(face).size(px(96.)).into_any_element(),
                None => self.profile_heads.draw(self.accounts.selected().map(|a| a.uuid.as_str()),
                    96., self.scale, &t).into_any_element(),
            });
        div()
            .id(SharedString::from(format!("skin-card-{id}")))
            .flex().flex_col().items_center().justify_between()
            .gap(px(8.)).w(px(172.)).h(px(212.)).p(px(12.))
            .rounded(px(18.)).bg(t.plate_soft)
            .border_1().border_color(if editing { t.accent } else { t.rule })
            .hover(|s| s.border_color(t.accent))
            .child(preview)
            .child(shell::text(theme::night::UI, t.ink).child(name))
            .child(if hovered || editing {
                button(SharedString::from(format!("edit-skin-{id}")), "Edit skin", Kind::Primary, &t)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.skin_open_editor(click_key.clone(), cx);
                    })).into_any_element()
            } else {
                div().h(px(34.)).into_any_element()
            })
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if *hovered {
                    this.skin_hovered = Some(key.clone());
                } else if this.skin_hovered.as_deref() == Some(key.as_str()) {
                    this.skin_hovered = None;
                }
                cx.notify();
            }))
    }

    pub(super) fn skins_page(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let selected = self.accounts.selected().cloned();
        let owner = selected.as_ref().map(|a| a.uuid.clone());
        if self.skin_capes_owner != owner {
            self.skin_capes_owner = owner;
            self.skin_capes_loaded = false;
            self.skin_cape_busy = false;
            self.skin_capes.clear();
        }
        if selected.is_some() && !self.skin_capes_loaded && !self.skin_cape_busy {
            self.skin_refresh_capes(cx);
        }
        let player = selected.as_ref().map(|a| a.name.clone())
            .unwrap_or_else(|| "Sign in to Minecraft".into());
        let viewer = self.skin_preview_dress(cx);
        let is_dragging = self.skin_preview_dragging;
        let left = div().flex().flex_col().items_center()
            .w(px(310.)).flex_none().gap(px(20.)).p(px(24.))
            .rounded(px(22.)).bg(t.plate_soft)
            .child(shell::text(theme::night::TITLE, t.ink).child("Skin Selector"))
            .child(shell::text(theme::night::UI, t.ink_soft).child(player))
            .child(div().flex().items_center().justify_center()
                .h(px(380.)).w_full().rounded(px(20.))
                .bg(t.plate)
                .children(viewer.map(|preview| {
                    wardrobe_page::viewer::frame(
                        preview, (260., 360.), is_dragging, Self::skin_preview_drag(cx)
                    )
                })))
            .child(shell::text(theme::night::UI, t.ink_soft)
                .child("Drag to rotate • Scroll to zoom"))
            .child(button("skin-pick-upload", "Add skin PNG", Kind::Primary, &t)
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.choose_minecraft_skin(cx))));
        let mut cards = div().flex().flex_wrap().gap(px(14.));
        cards = cards.child(
            div().flex().flex_col().items_center().justify_center()
                .gap(px(12.)).w(px(172.)).h(px(212.))
                .rounded(px(18.)).border_1().border_color(t.rule)
                .bg(t.plate_soft)
                .child(shell::text(theme::night::TITLE, t.accent).child("+"))
                .child(shell::text(theme::night::UI, t.ink).child("Add skin"))
                .child(shell::text(theme::night::MICRO, t.ink_soft).child("Choose a PNG"))
                .cursor_pointer()
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.choose_minecraft_skin(cx)))
        );
        cards = cards.child(self.skin_card("current".into(), "Current skin".into(), None, cx));
        for path in saved_skins() {
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Custom skin").to_owned();
            let id = path.to_string_lossy().into_owned();
            cards = cards.child(self.skin_card(id, name, skin_face(&path), cx));
        }

        let mut right = div().flex().flex_col().gap(px(18.)).flex_1().min_w_0()
            .child(div().flex().flex_col().gap(px(16.)).p(px(22.))
                .rounded(px(22.)).bg(t.plate_soft)
                .child(shell::text(theme::night::TITLE, t.ink).child("Saved Skins"))
                .child(cards));
        if let Some(ref editing) = self.skin_editing {
            let name = if editing == "current" { "Current skin".to_owned() }
                else { Path::new(editing).file_stem().and_then(|s| s.to_str())
                    .unwrap_or("Custom skin").to_owned() };
            let wide = button("skin-wide", "Wide (Steve)", if self.skin_slim { Kind::Ghost } else { Kind::Primary }, &t)
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| { this.skin_slim = false; cx.notify(); }));
            let slim = button("skin-slim", "Slim (Alex)", if self.skin_slim { Kind::Primary } else { Kind::Ghost }, &t)
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| { this.skin_slim = true; cx.notify(); }));
            let apply = button("skin-apply", if self.skin_uploading { "Applying…" } else { "Apply skin" },
                Kind::Primary, &t);
            let apply = if self.skin_uploading { disabled(apply) } else {
                apply.on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.skin_apply(cx)))
            };
            right = right.child(div().flex().flex_col().items_start().gap(px(14.)).p(px(22.))
                .rounded(px(22.)).bg(t.plate_soft)
                .child(shell::text(theme::night::TITLE, t.ink).child(format!("Edit · {name}")))
                .child(shell::text(theme::night::UI, t.ink_soft)
                    .child("Choose arm width, then apply it to your Minecraft Java account."))
                .child(div().flex().flex_wrap().gap(px(10.)).child(wide).child(slim))
                .child(apply));
        }
        let mut capes = div().flex().flex_wrap().gap(px(10.));
        let none_equipped = !self.skin_capes.iter().any(|cape| cape.active);
        let none = button("cape-none", if none_equipped { "✓ No cape" } else { "No cape" },
            if none_equipped { Kind::Primary } else { Kind::Secondary }, &t);
        capes = capes.child(if self.skin_cape_busy { disabled(none) } else {
            none.on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.skin_equip_cape(None, cx)))
        });
        for cape in self.skin_capes.clone() {
            let id = cape.id.clone();
            let label = if cape.active { format!("✓ {}", cape.name) } else { cape.name };
            let capebtn = button(SharedString::from(format!("cape-{}", cape.id)), label,
                if cape.active { Kind::Primary } else { Kind::Secondary }, &t);
            capes = capes.child(if self.skin_cape_busy { disabled(capebtn) } else {
                capebtn.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.skin_equip_cape(Some(id.clone()), cx);
                }))
            });
        }
        let refresh = button("cape-refresh", if self.skin_cape_busy { "Loading…" } else { "Refresh capes" },
            Kind::Ghost, &t);
        let refresh = if self.skin_cape_busy { disabled(refresh) } else {
            refresh.on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.skin_refresh_capes(cx)))
        };
        right = right.child(div().flex().flex_col().items_start().gap(px(14.)).p(px(22.))
            .rounded(px(22.)).bg(t.plate_soft)
            .child(shell::text(theme::night::TITLE, t.ink).child("Your Minecraft Capes"))
            .child(shell::text(theme::night::UI, t.ink_soft)
                .child("Only capes owned by your signed-in Minecraft account can be equipped."))
            .child(if self.skin_cape_busy {
                shell::text(theme::night::UI, t.ink_soft).child("Checking your capes…").into_any_element()
            } else if self.skin_capes.is_empty() {
                shell::text(theme::night::UI, t.ink_soft).child("No account capes found.").into_any_element()
            } else { div().into_any_element() })
            .child(capes).child(refresh));
        if let Some(status) = self.skin_status.clone() {
            right = right.child(div().p(px(14.)).rounded(px(12.)).bg(t.plate_soft)
                .child(shell::text(theme::night::UI, t.ink_soft).child(status)));
        }
        div().flex().flex_col().gap(px(20.)).pt(px(20.))
            .child(page_header("Skins", "Manage your saved Minecraft skins and official capes.", &t))
            .child(div().flex().flex_wrap().gap(px(20.)).items_start().child(left).child(right))
    }
}
