//! Valtic's native Cherry Grove opening, with the same artwork as Home.
//! Respects reduced motion and never needs loose files beside the Windows .exe.
use super::*;
use std::sync::Arc;

pub(super) fn render(
    held: f32,
    visible: f32,
    emblem: Option<Arc<gpui::RenderImage>>,
    scenery: Option<Arc<gpui::RenderImage>>,
) -> Div {
    let elapsed = if motion::still() { 2. } else { held };
    let progress = (elapsed / OPENING_HOLD.as_secs_f32()).clamp(0., 1.);
    let soft_pink = gpui::rgb(0xffbbda);
    let pale = gpui::rgb(0xfff2f7);

    // Translucent blossom petals glide down the background, not an aggressive
    // rotating red ring. Positions are deterministic for flicker-free startup.
    let mut petals = div().absolute().top_0().left_0().size_full();
    for (i, (x, y, size)) in [
        (0.07, 24., 10.), (0.16, 170., 7.), (0.29, 82., 9.),
        (0.43, 242., 8.), (0.59, 53., 11.), (0.75, 189., 7.),
        (0.87, 49., 9.), (0.94, 274., 12.), (0.38, 350., 7.),
        (0.64, 390., 10.), (0.12, 420., 8.), (0.81, 450., 9.),
    ].into_iter().enumerate() {
        let drift = if motion::still() { 0. } else { elapsed * (12. + i as f32 * 1.5) };
        let sway = if motion::still() { 0. } else {
            (elapsed * 1.2 + i as f32 * 0.8).sin() * 9.
        };
        petals = petals.child(
            div()
                .absolute()
                .left(gpui::relative(x))
                .top(px((y + drift).rem_euclid(650.) - 30. + sway))
                .w(px(size * 1.35))
                .h(px(size * 0.76))
                .rounded(px(size * 0.38))
                .bg(if i % 3 == 0 { gpui::rgb(0xffe8f3) } else { soft_pink })
                .opacity(0.38 + ((i % 4) as f32 * 0.12)),
        );
    }

    let scenery_layer = div()
        .absolute().top_0().left_0().size_full()
        .bg(gpui::rgb(0x25152d))
        .children(scenery.map(|image| {
            gpui::img(image)
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .object_fit(gpui::ObjectFit::Cover)
        }))
        .child(
            div()
                .absolute().top_0().left_0().size_full()
                .bg(gpui::linear_gradient(
                    90.,
                    gpui::linear_color_stop(gpui::rgb(0x221127).opacity(0.50), 0.),
                    gpui::linear_color_stop(gpui::rgb(0x160e22).opacity(0.82), 1.),
                )),
        );

    let logo = div()
        .size(px(175.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(38.))
        .bg(gpui::rgb(0x2b192f).opacity(0.50))
        .border_1()
        .border_color(soft_pink.opacity(0.25))
        .children(emblem.map(|image| {
            gpui::img(image)
                .size(px(164.))
                .object_fit(gpui::ObjectFit::Contain)
        }));

    let panel = div()
        .relative()
        .flex().flex_col()
        .items_center()
        .gap(px(10.))
        .px(px(34.))
        .py(px(30.))
        .rounded(px(26.))
        .bg(gpui::rgb(0x26192f).opacity(0.77))
        .border_1()
        .border_color(soft_pink.opacity(0.19))
        .child(logo)
        .child(
            div()
                .mt(px(9.))
                .font_family(horizon_theme::glass::DISPLAY_FONT)
                .font_weight(FontWeight::BOLD)
                .text_size(px(56.))
                .text_color(pale)
                .child("VALTIC"),
        )
        .child(
            div()
                .font_family(horizon_theme::glass::FONT)
                .font_weight(FontWeight::MEDIUM)
                .text_size(px(16.))
                .text_color(soft_pink)
                .child("CHERRY GROVE"),
        )
        .child(
            div()
                .mt(px(15.))
                .w(px(285.))
                .h(px(5.))
                .rounded_full()
                .overflow_hidden()
                .bg(pale.opacity(0.19))
                .child(
                    div()
                        .h_full()
                        .w(relative(progress))
                        .rounded_full()
                        .bg(gpui::rgb(0xeb7eb4)),
                ),
        )
        .child(
            div()
                .mt(px(6.))
                .font_family(horizon_theme::glass::FONT)
                .font_weight(FontWeight::MEDIUM)
                .text_size(px(12.))
                .text_color(pale.opacity(0.87))
                .child(if progress >= 1. {
                    "READY FOR YOUR NEXT ADVENTURE"
                } else {
                    "GROWING YOUR WORLD..."
                }),
        );

    div()
        .relative()
        .size_full()
        .overflow_hidden()
        .opacity(visible)
        .child(scenery_layer)
        .child(petals)
        .child(
            div()
                .relative()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(panel),
        )
}
