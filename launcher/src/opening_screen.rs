//! Custom lightweight Valtic startup screen, drawn using GPUI shapes and text.
use super::*;

pub(super) fn render(held: f32, visible: f32) -> Div {
    let elapsed = if motion::still() { 1.8 } else { held };
    let progress = (elapsed / OPENING_HOLD.as_secs_f32()).clamp(0., 1.);
    let pulse = 0.55 + 0.45 * (elapsed * 5.).sin().abs();
    let red = gpui::rgb(0xee3644);
    let pale = gpui::rgb(0xfff5f5);

    // A slowly turning ring of red markers, with brighter accents at each quarter.
    let orbit = (0..24).map(|i| {
        let angle = i as f32 * std::f32::consts::TAU / 24.
            + if motion::still() { 0. } else { elapsed * 0.42 };
        div()
            .absolute()
            .left(px(106. + 94. * angle.cos()))
            .top(px(106. + 94. * angle.sin()))
            .size(px(if i % 6 == 0 { 8. } else { 3. }))
            .rounded_full()
            .bg(if i % 6 == 0 { red } else { gpui::rgb(0x71232a) })
            .opacity(if i % 6 == 0 { pulse } else { 0.5 })
    });
    // Fixed positions make this animation deterministic and cheap to render.
    let embers = (0..18).map(|i| {
        let i = i as f32;
        let x = (i * 87.3).rem_euclid(610.);
        let y = (i * 47.7).rem_euclid(280.);
        let drift = if motion::still() { 0. } else { (elapsed + i).sin() * 7. };
        div()
            .absolute()
            .left(px(x))
            .top(px(y + drift))
            .size(px(2.))
            .rounded_full()
            .bg(if i as i32 % 3 == 0 { red } else { pale })
            .opacity(0.12 + 0.25 * ((elapsed + i).sin().abs()))
    });
    let emblem = div()
        .relative()
        .size(px(216.))
        .flex_none()
        .child(
            div()
                .absolute()
                .left(px(13.))
                .top(px(13.))
                .size(px(190.))
                .rounded_full()
                .border_1()
                .border_color(gpui::rgb(0x702630)),
        )
        .child(
            div()
                .absolute()
                .left(px(31.))
                .top(px(31.))
                .size(px(154.))
                .rounded_full()
                .border_1()
                .border_color(gpui::rgb(0x401b22))
                .bg(gpui::rgb(0x170b0f)),
        )
        .children(orbit)
        .child(
            div()
                .absolute()
                .left(px(48.))
                .top(px(46.))
                .size(px(120.))
                .flex()
                .items_center()
                .justify_center()
                .font_family(horizon_theme::glass::DISPLAY_FONT)
                .font_weight(FontWeight::BOLD)
                .text_size(px(103.))
                .text_color(red)
                .child("V"),
        )
        .child(
            div()
                .absolute()
                .left(px(53.))
                .bottom(px(26.))
                .w(px(110.))
                .h(px(2.))
                .rounded_full()
                .bg(red)
                .opacity(pulse),
        );
    let letters = "VALTIC".chars().enumerate().map(|(i, ch)| {
        let p = ease_out_cubic((elapsed - 0.14 - i as f32 * 0.07) / 0.48);
        div()
            .relative()
            .top(px(20. * (1. - p)))
            .opacity(p)
            .child(ch.to_string())
    });
    div()
        .relative()
        .flex()
        .flex_col()
        .items_center()
        .w(px(660.))
        .max_w_full()
        .px(px(24.))
        .opacity(visible)
        .child(
            div()
                .relative()
                .w_full()
                .h(px(290.))
                .flex()
                .items_center()
                .justify_center()
                .children(embers)
                .child(emblem),
        )
        .child(
            div()
                .flex()
                .gap(px(7.))
                .font_family(horizon_theme::glass::DISPLAY_FONT)
                .font_weight(FontWeight::BOLD)
                .text_size(px(64.))
                .line_height(px(76.))
                .text_color(pale)
                .children(letters),
        )
        .child(
            div()
                .mt(px(8.))
                .font_family(horizon_theme::glass::FONT)
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(12.))
                .text_color(gpui::rgb(0xe69aa0))
                .child("YOUR WORLD. YOUR WAY."),
        )
        .child(
            div()
                .mt(px(34.))
                .w(px(280.))
                .h(px(4.))
                .rounded_full()
                .overflow_hidden()
                .bg(gpui::rgb(0x362025))
                .child(div().h_full().w(relative(progress)).rounded_full().bg(red)),
        )
        .child(
            div()
                .mt(px(13.))
                .font_family(horizon_theme::glass::FONT)
                .font_weight(FontWeight::MEDIUM)
                .text_size(px(11.))
                .text_color(gpui::rgb(0xa18a8e))
                .child(if progress >= 1. { "WELCOME TO VALTIC" } else { "INITIALISING VALTIC" }),
        )
}
