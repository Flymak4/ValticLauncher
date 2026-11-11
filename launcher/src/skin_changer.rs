//! Genuine Minecraft Java skin upload directly to Minecraft Services, not a third party.
use super::*;

const SKIN_URL: &str = "https://api.minecraftservices.com/minecraft/profile/skins";
const MAX_SKIN_BYTES: u64 = 2 * 1024 * 1024;

fn upload_skin(path: &std::path::Path, slim: bool) -> Result<(), String> {
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
    // The account flow refreshes a Minecraft Services token after verifying ownership.
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
            Err("Minecraft account authorization failed. Sign in again after Mojang approves the launcher.".into()),
        Err(ureq::Error::Status(status, _)) =>
            Err(format!("Minecraft rejected the skin (HTTP {status}).")),
        Err(ureq::Error::Transport(e)) =>
            Err(format!("Cannot reach Minecraft Services: {e}")),
    }
}

impl Launcher {
    pub(super) fn choose_minecraft_skin(&mut self, cx: &mut Context<Self>) {
        if self.skin_uploading { return; }
        let slim = self.skin_slim;
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true, directories: false, multiple: false,
            prompt: Some("Choose Minecraft skin PNG".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else { return; };
            let Some(path) = paths.into_iter().next() else { return; };
            let _ = this.update(cx, |this, cx| {
                this.skin_uploading = true;
                this.skin_status = Some("Uploading directly to Minecraft Services…".into());
                cx.notify();
            });
            let result = cx.background_executor().spawn(async move {
                upload_skin(&path, slim)
            }).await;
            let _ = this.update(cx, |this, cx| {
                this.skin_uploading = false;
                match result {
                    Ok(()) => {
                        this.skin_status = Some("Skin updated! Rejoin Minecraft to see it.".into());
                        this.load_auth_statuses(true);
                    }
                    Err(e) => this.skin_status = Some(e),
                }
                cx.notify();
            });
        }).detach();
    }

    pub(super) fn skins_page(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let player = self.accounts.selected()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "No Microsoft account selected".into());
        let upload = button(
            "skin-upload",
            if self.skin_uploading { "Uploading…" } else { "Choose skin PNG" },
            Kind::Primary, &t,
        );
        let upload = if self.skin_uploading { disabled(upload) } else {
            upload.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.choose_minecraft_skin(cx);
                cx.notify();
            }))
        };
        let classic = button("skin-classic", "Classic arms",
            if self.skin_slim { Kind::Ghost } else { Kind::Secondary }, &t)
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.skin_slim = false; cx.notify();
            }));
        let slim = button("skin-slim", "Slim arms",
            if self.skin_slim { Kind::Secondary } else { Kind::Ghost }, &t)
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.skin_slim = true; cx.notify();
            }));
        div().flex().flex_col().gap(px(24.)).pt(px(24.))
            .child(page_header("Minecraft Skins",
                "Change your real Minecraft: Java Edition skin. Your image goes directly to Minecraft Services.", &t))
            .child(div().flex().flex_col().items_start().gap(px(16.))
                .p(px(28.)).rounded(px(18.)).bg(t.plate_soft)
                .child(shell::text(theme::night::TITLE, t.ink).child(player))
                .child(shell::text(theme::night::UI, t.ink_soft)
                    .child("Pick classic or slim arms, then upload a 64x64 PNG. Minecraft ownership and Microsoft sign-in are required."))
                .child(div().flex().flex_wrap().gap(px(10.)).child(classic).child(slim))
                .child(upload)
                .children(self.skin_status.clone().map(|msg| shell::text(theme::night::UI, t.ink_soft).child(msg)))
                .child(button("skin-official-website", "Manage on Minecraft.net", Kind::Secondary, &t)
                    .on_click(|_, _, cx| cx.open_url("https://www.minecraft.net/en-us/msaprofile"))))
    }
}
