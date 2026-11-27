use crate::commands::{self, DISCORD_URL, SUBSCRIPTIONS_URL};
use crate::runner;
use crate::types::{
    AppState, CheckStatus, EnvCheckItem, EnvReport, EventEmitter, OfferItem, SolariEvent, StepId,
    UiMessage,
};
use eframe::egui::{
    self, Align, Color32, CursorIcon, FontData, FontDefinitions, FontFamily, FontId, Frame, Layout,
    Margin, Pos2, Rect, ResizeDirection, RichText, Rounding, Sense, Stroke, TextureHandle,
    TextureOptions, Vec2, ViewportCommand,
};
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::mpsc;

pub const LOGO_PNG_BYTES: &[u8] = include_bytes!("../../src/assets/aureon-logo.png");

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    let a16 = a as u16;
    Color32::from_rgba_premultiplied(
        ((r as u16 * a16 + 127) / 255) as u8,
        ((g as u16 * a16 + 127) / 255) as u8,
        ((b as u16 * a16 + 127) / 255) as u8,
        a,
    )
}

// Materiais da janela, navegação e controles.
const GLASS_WINDOW_BG: Color32 = rgba(10, 11, 13, 245);
const GLASS_SIDEBAR_BG: Color32 = rgba(33, 35, 40, 216);
const GLASS_HEADER_BG: Color32 = rgba(25, 26, 30, 235);
const GLASS_CARD_IDLE: Color32 = rgba(255, 255, 255, 12);
const GLASS_LOG_BG: Color32 = rgba(26, 28, 32, 240);
const BORDER_OUTER_WINDOW: Color32 = rgba(255, 255, 255, 30);
const BORDER_GLASS: Color32 = rgba(240, 242, 248, 32);
const BORDER_SUBTLE: Color32 = rgba(240, 242, 248, 20);

const FG_PRIMARY: Color32 = Color32::from_rgb(247, 247, 249);
const FG_SECONDARY: Color32 = Color32::from_rgb(211, 212, 218);
const FG_MUTED: Color32 = Color32::from_rgb(165, 168, 177);

const ACCENT_IRIS: Color32 = Color32::from_rgb(165, 164, 255);
const ACTION_FILL: Color32 = Color32::from_rgb(236, 237, 242);
const ACTION_HOVER: Color32 = Color32::WHITE;
const ACCENT_FG_DARK: Color32 = Color32::from_rgb(21, 22, 27);

const EMERALD_FG: Color32 = Color32::from_rgb(74, 222, 168);
const EMERALD_BG: Color32 = rgba(16, 185, 129, 28);
const EMERALD_BORDER: Color32 = rgba(52, 211, 153, 84);

const AMBER_FG: Color32 = Color32::from_rgb(252, 202, 70);
const AMBER_BG: Color32 = rgba(245, 158, 11, 28);
const AMBER_BORDER: Color32 = rgba(251, 191, 36, 86);

const RED_FG: Color32 = Color32::from_rgb(252, 129, 142);
const RED_BG: Color32 = rgba(239, 68, 84, 30);
const RED_BORDER: Color32 = rgba(248, 113, 128, 88);

const SKY_FG: Color32 = Color32::from_rgb(125, 211, 252);
const SKY_BG: Color32 = rgba(14, 165, 233, 28);
const SKY_BORDER: Color32 = rgba(56, 189, 248, 82);

#[derive(Clone, Copy, PartialEq, Eq)]
enum BtnVariant {
    Primary,
    Secondary,
    Destructive,
    Ghost,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BadgeKind {
    Default,
    Secondary,
    Outline,
    Success,
    Warning,
    Destructive,
}

struct AccumulationStepDef {
    id: StepId,
    order: u32,
    title: &'static str,
    subtitle: &'static str,
    offer_id: &'static str,
    months: u32,
    requires_token: bool,
}

const ACCUMULATION_STEPS: [AccumulationStepDef; 4] = [
    AccumulationStepDef {
        id: StepId::Pro12,
        order: 1,
        title: "Passo 1 — Ativação Inicial (12 meses)",
        subtitle: "Assinatura base de 12 meses gratuitos sem token anterior",
        offer_id: "12months-eft",
        months: 12,
        requires_token: false,
    },
    AccumulationStepDef {
        id: StepId::Pro6,
        order: 2,
        title: "Passo 2 — Soma +6 meses (18m total)",
        subtitle: "Upgrade usando purchaseToken vigente + CHARGE_FULL_PRICE (mode 5)",
        offer_id: "6months-eft",
        months: 6,
        requires_token: true,
    },
    AccumulationStepDef {
        id: StepId::Pro6b,
        order: 3,
        title: "Passo 3 — Soma +6 meses (24m total)",
        subtitle: "Segunda extensão de 6 meses via proration mode 5",
        offer_id: "6months-eft-2",
        months: 6,
        requires_token: true,
    },
    AccumulationStepDef {
        id: StepId::Pro6c,
        order: 4,
        title: "Passo 4 — Soma Extra +6 meses (30m total)",
        subtitle: "Extensão opcional ss-2026 para atingir até 30 meses",
        offer_id: "6months-eft-ss-2026",
        months: 6,
        requires_token: true,
    },
];

fn default_env_checks() -> Vec<EnvCheckItem> {
    vec![
        EnvCheckItem {
            id: "adb".into(),
            label: "ADB (Android Debug Bridge)".into(),
            status: CheckStatus::Pending,
            detail: None,
        },
        EnvCheckItem {
            id: "emulator".into(),
            label: "Emulador Android conectado".into(),
            status: CheckStatus::Pending,
            detail: None,
        },
        EnvCheckItem {
            id: "root".into(),
            label: "Permissão Root (uid=0)".into(),
            status: CheckStatus::Pending,
            detail: None,
        },
        EnvCheckItem {
            id: "network".into(),
            label: "Rede & Proxy Limpos".into(),
            status: CheckStatus::Pending,
            detail: None,
        },
        EnvCheckItem {
            id: "play_store".into(),
            label: "Google Play Store".into(),
            status: CheckStatus::Pending,
            detail: None,
        },
        EnvCheckItem {
            id: "google_one".into(),
            label: "Google One Instalado".into(),
            status: CheckStatus::Pending,
            detail: None,
        },
        EnvCheckItem {
            id: "runner".into(),
            label: "Motor Android (Runner)".into(),
            status: CheckStatus::Pending,
            detail: None,
        },
    ]
}

struct LogEntry {
    level: String,
    message: String,
    timestamp: String,
}

pub fn load_window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(LOGO_PNG_BYTES).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn configure_fonts_and_style(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts
            .font_data
            .insert("segoe".to_owned(), FontData::from_owned(bytes).into());
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Proportional) {
            fam.insert(0, "segoe".to_owned());
        }
    }
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\seguisb.ttf") {
        fonts
            .font_data
            .insert("segoe_sb".to_owned(), FontData::from_owned(bytes).into());
        fonts.families.insert(
            FontFamily::Name("semibold".into()),
            vec!["segoe_sb".to_owned(), "segoe".to_owned()],
        );
    }
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\consola.ttf") {
        fonts
            .font_data
            .insert("consolas".to_owned(), FontData::from_owned(bytes).into());
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Monospace) {
            fam.insert(0, "consolas".to_owned());
        }
    }

    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.window_fill = rgba(36, 38, 43, 248);
    style.visuals.override_text_color = Some(FG_PRIMARY);
    style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(165, 145, 245, 95);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.scroll = egui::style::ScrollStyle::solid();
    style.spacing.scroll.bar_width = 5.0;
    ctx.set_style(style);
}

fn font_semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

fn font_regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

fn font_mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

/// Formata caminhos longos para exibição limpa sem vazar nomes legados de cache.
fn clean_display_text(raw: &str) -> String {
    if raw.contains(r"\platform-tools\adb.exe") {
        return r"...\bin\platform-tools\adb.exe".to_string();
    }
    raw.replace(r"\Solari\", r"\Aureon\")
}

// ============================================================================
// Primitivas Vetoriais Liquid Glass (zero dependência de glifos Unicode)
// ============================================================================

fn paint_liquid_glass(
    painter: &egui::Painter,
    rect: Rect,
    rounding: f32,
    fill: Color32,
    border: Color32,
    gloss: f32,
) {
    painter.rect(
        rect,
        Rounding::same(rounding),
        fill,
        Stroke::new(1.0_f32, border),
    );

    if gloss > 0.0 && rect.width() > rounding * 2.2 && rect.height() > 12.0 {
        // Reflexo curto na borda interna, com a luz se dissipando no vidro.
        let left = rect.left() + rounding;
        let right = rect.right() - rounding;
        let top = rect.top() + 0.8;
        let mut rim = egui::Mesh::default();
        for (x, color) in [
            (left, Color32::TRANSPARENT),
            (left + (right - left) * 0.32, rgba(215, 227, 255, (112.0 * gloss) as u8)),
            (right, Color32::TRANSPARENT),
        ] {
            rim.colored_vertex(Pos2::new(x, top), color);
            rim.colored_vertex(Pos2::new(x, top + 3.0), Color32::TRANSPARENT);
        }
        for i in [0, 2] {
            rim.add_triangle(i, i + 1, i + 2);
            rim.add_triangle(i + 2, i + 1, i + 3);
        }
        painter.add(egui::Shape::mesh(rim));
    }
}

fn paint_vec_check(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    let p1 = Pos2::new(center.x - radius * 0.55, center.y + radius * 0.02);
    let p2 = Pos2::new(center.x - radius * 0.12, center.y + radius * 0.45);
    let p3 = Pos2::new(center.x + radius * 0.62, center.y - radius * 0.42);
    let stroke = Stroke::new(1.8_f32, color);
    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
}

fn paint_section_icon(p: &egui::Painter, center: Pos2, step: u8, ink: Color32) {
    let at = |x, y| center + Vec2::new(x, y);
    let stroke = Stroke::new(1.5, ink);
    let line = |a, b| { p.line_segment([a, b], stroke); };
    match step {
        1 => {
            p.rect_stroke(Rect::from_min_max(at(-8.0, -6.5), at(8.0, 4.5)), Rounding::same(2.0), stroke);
            line(at(0.0, 5.0), at(0.0, 8.0));
            line(at(-4.0, 8.0), at(4.0, 8.0));
        }
        2 => {
            let corners = [at(-7.5, -6.0), at(1.0, -6.0), at(8.0, 1.0), at(1.2, 8.0), at(-7.5, 0.0)];
            p.add(egui::Shape::closed_line(corners.to_vec(), stroke));
            p.circle_stroke(at(-3.5, -2.5), 1.1, stroke);
        }
        3 => {
            p.add(egui::Shape::closed_line(vec![at(0.0,-8.0), at(8.0,-3.5), at(0.0,1.0), at(-8.0,-3.5)], stroke));
            for y in [1.0, 5.0] {
                line(at(-8.0,y), at(0.0,y+4.0));
                line(at(0.0,y+4.0), at(8.0,y));
            }
        }
        _ => {
            p.circle_stroke(center, 8.0, stroke);
            paint_vec_check(p, center, 6.0, ink);
        }
    }
}

fn paint_vec_cross(painter: &egui::Painter, center: Pos2, half: f32, color: Color32) {
    let s = Stroke::new(1.6_f32, color);
    painter.line_segment(
        [
            Pos2::new(center.x - half, center.y - half),
            Pos2::new(center.x + half, center.y + half),
        ],
        s,
    );
    painter.line_segment(
        [
            Pos2::new(center.x + half, center.y - half),
            Pos2::new(center.x - half, center.y + half),
        ],
        s,
    );
}

fn paint_vec_chevron(painter: &egui::Painter, center: Pos2, down: bool, color: Color32) {
    let s = Stroke::new(1.6_f32, color);
    if down {
        painter.line_segment(
            [
                Pos2::new(center.x - 4.5, center.y - 2.0),
                Pos2::new(center.x, center.y + 2.5),
            ],
            s,
        );
        painter.line_segment(
            [
                Pos2::new(center.x, center.y + 2.5),
                Pos2::new(center.x + 4.5, center.y - 2.0),
            ],
            s,
        );
    } else {
        painter.line_segment(
            [
                Pos2::new(center.x - 2.0, center.y - 4.5),
                Pos2::new(center.x + 2.5, center.y),
            ],
            s,
        );
        painter.line_segment(
            [
                Pos2::new(center.x + 2.5, center.y),
                Pos2::new(center.x - 2.0, center.y + 4.5),
            ],
            s,
        );
    }
}

fn paint_vec_spinner(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32, time: f64) {
    let base_angle = (time * 5.2) as f32;
    let steps = 14;
    let arc_len = std::f32::consts::PI * 1.45;
    let mut pts = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let a = base_angle + (i as f32 / steps as f32) * arc_len;
        pts.push(Pos2::new(
            center.x + a.cos() * radius,
            center.y + a.sin() * radius,
        ));
    }
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], Stroke::new(1.8_f32, color));
    }
}

fn paint_status_glyph(
    painter: &egui::Painter,
    center: Pos2,
    status: CheckStatus,
    time: f64,
) {
    match status {
        CheckStatus::Ok => {
            painter.circle_filled(
                center,
                9.0,
                Color32::from_rgba_unmultiplied(16, 185, 129, 42),
            );
            painter.circle_stroke(center, 9.0, Stroke::new(1.0_f32, EMERALD_BORDER));
            paint_vec_check(painter, center, 5.5, EMERALD_FG);
        }
        CheckStatus::Running => {
            painter.circle_filled(
                center,
                9.0,
                Color32::from_rgba_unmultiplied(158, 159, 246, 30),
            );
            paint_vec_spinner(painter, center, 6.2, ACCENT_IRIS, time);
        }
        CheckStatus::Warning => {
            painter.circle_filled(
                center,
                9.0,
                Color32::from_rgba_unmultiplied(245, 158, 11, 40),
            );
            painter.circle_stroke(center, 9.0, Stroke::new(1.0_f32, AMBER_BORDER));
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y - 4.0),
                    Pos2::new(center.x, center.y + 1.2),
                ],
                Stroke::new(1.8_f32, AMBER_FG),
            );
            painter.circle_filled(Pos2::new(center.x, center.y + 4.2), 1.1, AMBER_FG);
        }
        CheckStatus::Error => {
            painter.circle_filled(
                center,
                9.0,
                Color32::from_rgba_unmultiplied(239, 68, 84, 42),
            );
            painter.circle_stroke(center, 9.0, Stroke::new(1.0_f32, RED_BORDER));
            paint_vec_cross(painter, center, 3.6, RED_FG);
        }
        CheckStatus::Pending => {
            painter.circle_stroke(
                center,
                7.5,
                Stroke::new(1.2_f32, Color32::from_rgba_unmultiplied(180, 184, 214, 78)),
            );
            painter.circle_filled(
                center,
                2.2,
                Color32::from_rgba_unmultiplied(180, 184, 214, 95),
            );
        }
    }
}

fn badge_colors(kind: BadgeKind) -> (Color32, Color32, Color32) {
    match kind {
        BadgeKind::Default => (
            Color32::from_rgba_unmultiplied(158, 159, 246, 34),
            Color32::from_rgba_unmultiplied(158, 159, 246, 92),
            ACCENT_IRIS,
        ),
        BadgeKind::Secondary => (
            Color32::from_rgba_unmultiplied(255, 255, 255, 16),
            Color32::from_rgba_unmultiplied(255, 255, 255, 42),
            FG_SECONDARY,
        ),
        BadgeKind::Outline => (
            Color32::from_rgba_unmultiplied(255, 255, 255, 7),
            Color32::from_rgba_unmultiplied(226, 218, 255, 52),
            FG_MUTED,
        ),
        BadgeKind::Success => (EMERALD_BG, EMERALD_BORDER, EMERALD_FG),
        BadgeKind::Warning => (AMBER_BG, AMBER_BORDER, AMBER_FG),
        BadgeKind::Destructive => (RED_BG, RED_BORDER, RED_FG),
    }
}

fn draw_badge(ui: &mut egui::Ui, text: &str, kind: BadgeKind) {
    let (bg, border, fg) = badge_colors(kind);
    let font = font_semibold(11.0);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, fg);
    let size = Vec2::new(galley.size().x + 18.0, 22.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect(rect, Rounding::same(99.0), bg, Stroke::new(1.0_f32, border));
    ui.painter().galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        fg,
    );
}

fn aur_button(ui: &mut egui::Ui, label: &str, variant: BtnVariant, enabled: bool) -> bool {
    let font = font_semibold(12.8);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), FG_PRIMARY);
    let desired = Vec2::new(galley.size().x + 28.0, 35.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(desired, sense);

    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    let hovered = enabled && resp.hovered();
    let lift = ui.ctx().animate_bool_with_time(resp.id.with("hover"), hovered, 0.16);
    let (mut bg, mut border, mut fg, spec) = match variant {
        BtnVariant::Primary => (
            ACTION_FILL.lerp_to_gamma(ACTION_HOVER, lift),
            BORDER_GLASS,
            ACCENT_FG_DARK,
            true,
        ),
        BtnVariant::Secondary => (
            rgba(220, 210, 255, 22).lerp_to_gamma(rgba(232, 222, 255, 42), lift),
            Color32::from_rgba_unmultiplied(234, 226, 255, 68),
            FG_PRIMARY,
            true,
        ),
        BtnVariant::Destructive => (
            rgba(190, 68, 96, 76).lerp_to_gamma(rgba(210, 76, 104, 110), lift),
            Color32::from_rgba_unmultiplied(252, 165, 180, 96),
            Color32::from_rgb(255, 224, 230),
            true,
        ),
        BtnVariant::Ghost => (
            Color32::TRANSPARENT.lerp_to_gamma(rgba(230, 220, 255, 28), lift),
            Color32::TRANSPARENT,
            FG_MUTED.lerp_to_gamma(FG_PRIMARY, lift),
            false,
        ),
    };

    if !enabled {
        bg = Color32::from_rgb(38, 39, 44);
        border = BORDER_SUBTLE;
        fg = Color32::from_rgb(158, 161, 170);
    }

    let press = ui.ctx().animate_bool_with_time(resp.id.with("press"), enabled && resp.is_pointer_button_down_on(), 0.09);
    let face = rect.shrink(1.25 * press);
    paint_liquid_glass(
        ui.painter(),
        face,
        face.height() * 0.5,
        bg,
        border.gamma_multiply(0.76 + 0.24 * lift),
        if spec && enabled { 0.35 + 0.65 * lift } else { 0.0 },
    );

    let text_galley = ui.painter().layout_no_wrap(label.to_owned(), font, fg);
    ui.painter().galley(
        Pos2::new(
            rect.center().x - text_galley.size().x * 0.5,
            rect.center().y - text_galley.size().y * 0.5,
        ),
        text_galley,
        fg,
    );

    enabled && resp.clicked()
}

fn responsive_row(
    ui: &mut egui::Ui,
    trailing_width: f32,
    leading: impl FnOnce(&mut egui::Ui),
    trailing: impl FnOnce(&mut egui::Ui),
) {
    let width = ui.available_width();
    let gap = ui.spacing().item_spacing.x;
    let leading_width = width - trailing_width - gap;
    if leading_width < 240.0 {
        ui.vertical(leading);
        ui.horizontal_wrapped(trailing);
        return;
    }

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(leading_width, 0.0),
            Layout::top_down(Align::Min),
            leading,
        );
        ui.allocate_ui_with_layout(
            Vec2::new(trailing_width, 0.0),
            Layout::right_to_left(Align::Center),
            trailing,
        );
    });
}

fn draw_progress_bar(ui: &mut egui::Ui, fraction: f32) {
    let clamped = fraction.clamp(0.0, 1.0);
    let width = ui.available_width().max(40.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 7.0), Sense::hover());
    ui.painter().rect_filled(
        rect,
        Rounding::same(99.0),
        Color32::from_rgba_unmultiplied(255, 255, 255, 20),
    );
    if clamped > 0.0 {
        let fill_w = (rect.width() * clamped).max(7.0).min(rect.width());
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height()));
        ui.painter()
            .rect_filled(fill_rect, Rounding::same(99.0), ACCENT_IRIS);
    }
}

fn format_price(offer: &OfferItem) -> String {
    if offer.is_free || offer.micros == "0" {
        return "Gratuito (R$ 0,00)".to_string();
    }
    if let Ok(micros) = offer.micros.parse::<f64>() {
        if micros > 0.0 {
            let reais = micros / 1_000_000.0;
            return format!("R$ {:.2}", reais).replace('.', ",");
        }
    }
    offer.micros.clone()
}

pub struct AureonApp {
    rt: Handle,
    state: Arc<AppState>,
    tx: mpsc::UnboundedSender<UiMessage>,
    rx: mpsc::UnboundedReceiver<UiMessage>,
    logo_tex: Option<TextureHandle>,
    glass_applied: bool,

    current_step: u8,
    displayed_step: u8,
    transition_start: Option<f64>,
    max_unlocked_step: u8,

    // Step 1
    env_checks: Vec<EnvCheckItem>,
    env_report: Option<EnvReport>,
    is_checking_env: bool,
    step1_error: Option<String>,

    // Step 2
    offers: Vec<OfferItem>,
    has_checked_offers: bool,
    is_loading_offers: bool,
    step2_error: Option<String>,

    // Step 3
    completed_sub_steps: Vec<StepId>,
    active_step_id: Option<StepId>,
    step3_status: Option<String>,
    step3_error: Option<String>,

    // Logs
    logs: Vec<LogEntry>,
    logs_open: bool,
}

impl AureonApp {
    pub fn new(cc: &eframe::CreationContext<'_>, rt: Handle) -> Self {
        configure_fonts_and_style(&cc.egui_ctx);

        let load_art = |name: &str, bytes: &[u8]| {
            let rgba = image::load_from_memory(bytes).ok()?.into_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let pixels = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            Some(cc.egui_ctx.load_texture(name, pixels, TextureOptions::LINEAR))
        };
        let logo_tex = load_art("aureon-logo", LOGO_PNG_BYTES);

        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            rt,
            state: Arc::new(AppState::default()),
            tx,
            rx,
            logo_tex,
            glass_applied: false,
            current_step: 1,
            displayed_step: 1,
            transition_start: None,
            max_unlocked_step: 1,
            env_checks: default_env_checks(),
            env_report: None,
            is_checking_env: false,
            step1_error: None,
            offers: Vec::new(),
            has_checked_offers: false,
            is_loading_offers: false,
            step2_error: None,
            completed_sub_steps: Vec::new(),
            active_step_id: None,
            step3_status: None,
            step3_error: None,
            logs: Vec::new(),
            logs_open: false,
        }
    }

    fn emitter(&self, ctx: &egui::Context) -> EventEmitter {
        EventEmitter::new(self.tx.clone(), ctx.clone())
    }

    fn append_log(&mut self, level: &str, message: impl Into<String>) {
        self.append_log_ts(level, message.into(), runner::now_timestamp());
    }

    fn append_log_ts(&mut self, level: &str, message: String, timestamp: String) {
        if self.logs.len() >= 300 {
            self.logs.remove(0);
        }
        self.logs.push(LogEntry {
            level: level.to_string(),
            message: clean_display_text(&message),
            timestamp,
        });
    }

    fn mark_sub_step_completed(&mut self, step_id: StepId) {
        if !self.completed_sub_steps.contains(&step_id) {
            self.completed_sub_steps.push(step_id);
        }
        if self.active_step_id == Some(step_id) {
            self.active_step_id = None;
        }
    }

    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                UiMessage::Event(ev) => match ev {
                    SolariEvent::Log {
                        level,
                        message,
                        timestamp,
                    } => {
                        self.append_log_ts(&level, message, timestamp);
                    }
                    SolariEvent::EnvProgress { item } => {
                        if let Some(slot) = self.env_checks.iter_mut().find(|c| c.id == item.id) {
                            *slot = item;
                        }
                    }
                    SolariEvent::Offer { offer } => {
                        if let Some(existing) = self.offers.iter_mut().find(|o| {
                            o.product == offer.product && o.offer_id == offer.offer_id
                        }) {
                            *existing = offer;
                        } else {
                            self.offers.push(offer);
                        }
                    }
                    SolariEvent::Chosen {
                        product,
                        offer_id,
                        period,
                    } => {
                        self.append_log(
                            "info",
                            format!("Oferta selecionada: {} ({}) em {}", offer_id, period, product),
                        );
                    }
                    SolariEvent::Launched { msg, .. } => {
                        self.step3_status = Some(msg);
                        self.step3_error = None;
                    }
                    SolariEvent::PurchaseCb { step_id, code, msg } => {
                        if code == 0 {
                            self.mark_sub_step_completed(step_id);
                            self.step3_status = Some(format!(
                                "Passo '{}' confirmado (code = 0). Avançando para a próxima etapa.",
                                step_id.as_label()
                            ));
                            self.step3_error = None;
                        } else {
                            self.active_step_id = None;
                            self.step3_status = None;
                            self.step3_error = Some(format!(
                                "Retorno da Play Store no passo '{}' (código {}): {}",
                                step_id.as_label(),
                                code,
                                msg
                            ));
                        }
                    }
                    SolariEvent::StepDone {
                        step_id,
                        success,
                        message,
                    } => {
                        if success {
                            self.mark_sub_step_completed(step_id);
                            self.step3_status = Some(message);
                            self.step3_error = None;
                        } else {
                            self.active_step_id = None;
                            self.step3_status = None;
                            self.step3_error = Some(message);
                        }
                    }
                    SolariEvent::Error { message } => {
                        self.append_log("error", message.clone());
                        self.is_checking_env = false;
                        self.is_loading_offers = false;
                        self.active_step_id = None;
                        self.step3_error = Some(message);
                    }
                    SolariEvent::Cancelled => {
                        self.active_step_id = None;
                        self.is_checking_env = false;
                        self.is_loading_offers = false;
                        self.step3_status = Some("Tarefa cancelada pelo usuário.".to_string());
                    }
                },
                UiMessage::EnvFinished(res) => {
                    self.is_checking_env = false;
                    match res {
                        Ok(report) => {
                            if !report.checks.is_empty() {
                                self.env_checks = report.checks.clone();
                            }
                            if report.ready && self.max_unlocked_step < 2 {
                                self.max_unlocked_step = 2;
                            }
                            self.env_report = Some(report);
                        }
                        Err(e) => {
                            self.append_log("error", format!("Erro no diagnóstico: {}", e));
                            self.step1_error = Some(e);
                        }
                    }
                }
                UiMessage::OffersFinished(res) => {
                    self.is_loading_offers = false;
                    match res {
                        Ok(list) => {
                            self.offers = list;
                            self.has_checked_offers = true;
                            if self.max_unlocked_step < 3 {
                                self.max_unlocked_step = 3;
                            }
                        }
                        Err(e) => {
                            self.append_log("error", format!("Erro ao consultar ofertas: {}", e));
                            self.step2_error = Some(e);
                        }
                    }
                }
                UiMessage::StepFinished { step_id, result } => {
                    if let Err(e) = result {
                        if self.active_step_id == Some(step_id) {
                            self.active_step_id = None;
                        }
                        self.step3_status = None;
                        self.append_log(
                            "error",
                            format!("Erro ao executar '{}': {}", step_id.as_label(), e),
                        );
                        self.step3_error = Some(e);
                    }
                }
            }
        }
    }

    fn trigger_env_check(&mut self, ctx: &egui::Context) {
        if self.is_checking_env {
            return;
        }
        self.step1_error = None;
        self.env_report = None;
        self.env_checks = default_env_checks();
        self.is_checking_env = true;

        let emitter = self.emitter(ctx);
        let emitter_done = emitter.clone();
        let state = self.state.clone();
        self.rt.spawn(async move {
            let res = commands::check_environment(emitter, state).await;
            emitter_done.send(UiMessage::EnvFinished(res));
        });
    }

    fn trigger_check_offers(&mut self, ctx: &egui::Context) {
        if self.is_loading_offers {
            return;
        }
        self.step2_error = None;
        self.offers.clear();
        self.is_loading_offers = true;

        let emitter = self.emitter(ctx);
        let emitter_done = emitter.clone();
        let state = self.state.clone();
        self.rt.spawn(async move {
            let res = commands::list_offers(emitter, state).await;
            emitter_done.send(UiMessage::OffersFinished(res));
        });
    }

    fn trigger_start_step(&mut self, ctx: &egui::Context, step_id: StepId) {
        if self.active_step_id.is_some() {
            return;
        }
        self.step3_error = None;
        self.step3_status = Some(format!("Iniciando etapa '{}'...", step_id.as_label()));
        self.active_step_id = Some(step_id);

        let emitter = self.emitter(ctx);
        let emitter_done = emitter.clone();
        let state = self.state.clone();
        self.rt.spawn(async move {
            let result = commands::start_step(emitter, state, step_id).await;
            emitter_done.send(UiMessage::StepFinished { step_id, result });
        });
    }

    fn trigger_cancel_task(&mut self, ctx: &egui::Context) {
        self.active_step_id = None;
        self.step3_status = Some("Tarefa cancelada pelo usuário.".to_string());
        self.step3_error = None;

        let emitter = self.emitter(ctx);
        let state = self.state.clone();
        self.rt.spawn(async move {
            let _ = commands::cancel_active_task(emitter, state).await;
        });
    }

    fn reset_wizard(&mut self) {
        self.current_step = 1;
        self.max_unlocked_step = 1;
        self.env_checks = default_env_checks();
        self.env_report = None;
        self.step1_error = None;
        self.offers.clear();
        self.has_checked_offers = false;
        self.step2_error = None;
        self.completed_sub_steps.clear();
        self.active_step_id = None;
        self.step3_status = None;
        self.step3_error = None;
        self.append_log("info", "Assistente reiniciado.");
    }

    fn go_to_step(&mut self, step: u8) {
        if step > self.max_unlocked_step {
            self.max_unlocked_step = step;
        }
        self.current_step = step;
    }

    fn step_is_done(&self, step: u8) -> bool {
        match step {
            1 => self.env_report.as_ref().map(|r| r.ready).unwrap_or(false),
            2 => self.has_checked_offers && !self.offers.is_empty(),
            3 => !self.completed_sub_steps.is_empty(),
            _ => false,
        }
    }

    // ========================================================================
    // Barra de Título Nativa Frameless + Controles de Janela
    // ========================================================================

    fn render_titlebar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let bar_h = 46.0;
        let full_w = ui.available_width();
        let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(full_w, bar_h), Sense::hover());

        ui.painter().rect_filled(bar_rect, Rounding::ZERO, GLASS_HEADER_BG);
        ui.painter().line_segment(
            [
                Pos2::new(bar_rect.left(), bar_rect.bottom()),
                Pos2::new(bar_rect.right(), bar_rect.bottom()),
            ],
            Stroke::new(1.0_f32, BORDER_SUBTLE),
        );

        // Área arrastável (toda a barra menos os 3 botões da direita)
        let drag_rect = Rect::from_min_max(
            bar_rect.min,
            Pos2::new(bar_rect.right() - 138.0, bar_rect.bottom()),
        );
        let drag_resp = ui.interact(drag_rect, egui::Id::new("aur_title_drag"), Sense::click_and_drag());
        if drag_resp.drag_started() {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if drag_resp.double_clicked() {
            let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_max));
        }

        // Logo + Nome Auréon
        let logo_rect = Rect::from_min_size(
            Pos2::new(bar_rect.left() + 16.0, bar_rect.center().y - 17.0),
            Vec2::new(34.0, 34.0),
        );
        ui.painter().rect(
            logo_rect,
            Rounding::same(10.0),
            Color32::BLACK,
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(230, 222, 255, 78)),
        );
        if let Some(tex) = &self.logo_tex {
            ui.painter().image(
                tex.id(),
                logo_rect.shrink(1.0),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }

        let title_galley = ui.painter().layout_no_wrap(
            "Auréon".to_owned(),
            font_semibold(18.0),
            FG_PRIMARY,
        );
        let title_x = logo_rect.right() + 11.0;
        ui.painter().galley(
            Pos2::new(title_x, bar_rect.center().y - title_galley.size().y * 0.5 - 1.0),
            title_galley.clone(),
            FG_PRIMARY,
        );

        // Subtítulo discreto ao lado
        let sub_x = title_x + title_galley.size().x + 12.0;
        ui.painter().circle_filled(
            Pos2::new(sub_x - 5.0, bar_rect.center().y),
            1.6,
            Color32::from_rgba_unmultiplied(158, 159, 246, 120),
        );
        let sub_galley = ui.painter().layout_no_wrap(
            "Assistente Google AI Pro".to_owned(),
            font_regular(12.0),
            FG_MUTED,
        );
        ui.painter().galley(
            Pos2::new(sub_x + 2.0, bar_rect.center().y - sub_galley.size().y * 0.5),
            sub_galley,
            FG_MUTED,
        );

        // Botões nativos da janela (Minimize, Maximize, Close)
        let btn_w = 42.0;
        let btn_h = 32.0;
        let base_y = bar_rect.center().y - btn_h * 0.5;
        let close_rect = Rect::from_min_size(
            Pos2::new(bar_rect.right() - btn_w - 8.0, base_y),
            Vec2::new(btn_w, btn_h),
        );
        let max_rect = Rect::from_min_size(
            Pos2::new(close_rect.left() - btn_w - 2.0, base_y),
            Vec2::new(btn_w, btn_h),
        );
        let min_rect = Rect::from_min_size(
            Pos2::new(max_rect.left() - btn_w - 2.0, base_y),
            Vec2::new(btn_w, btn_h),
        );

        // Minimize
        let min_resp = ui.interact(min_rect, egui::Id::new("win_min"), Sense::click());
        if min_resp.hovered() {
            ui.painter().rect_filled(
                min_rect,
                Rounding::same(8.0),
                Color32::from_rgba_unmultiplied(255, 255, 255, 22),
            );
        }
        ui.painter().line_segment(
            [
                Pos2::new(min_rect.center().x - 5.0, min_rect.center().y),
                Pos2::new(min_rect.center().x + 5.0, min_rect.center().y),
            ],
            Stroke::new(1.4_f32, FG_SECONDARY),
        );
        if min_resp.clicked() {
            ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
        }

        // Maximize
        let max_resp = ui.interact(max_rect, egui::Id::new("win_max"), Sense::click());
        if max_resp.hovered() {
            ui.painter().rect_filled(
                max_rect,
                Rounding::same(8.0),
                Color32::from_rgba_unmultiplied(255, 255, 255, 22),
            );
        }
        let sq = Rect::from_center_size(max_rect.center(), Vec2::new(9.5, 9.5));
        ui.painter()
            .rect_stroke(sq, Rounding::same(1.5), Stroke::new(1.3_f32, FG_SECONDARY));
        if max_resp.clicked() {
            let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_max));
        }

        // Close
        let close_resp = ui.interact(close_rect, egui::Id::new("win_close"), Sense::click());
        if close_resp.hovered() {
            ui.painter().rect_filled(
                close_rect,
                Rounding::same(8.0),
                Color32::from_rgba_unmultiplied(225, 58, 78, 210),
            );
        }
        let x_col = if close_resp.hovered() {
            Color32::WHITE
        } else {
            FG_SECONDARY
        };
        paint_vec_cross(ui.painter(), close_rect.center(), 4.5, x_col);
        if close_resp.clicked() {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    // ========================================================================
    // Sidebar Lateral Liquid Glass
    // ========================================================================

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 5.0;
            ui.add_space(16.0);
            for (index, title) in ["Diagnóstico", "Ofertas", "Acumulação", "Finalização"]
                .iter()
                .enumerate()
            {
                let step = index as u8 + 1;
                let active = self.current_step == step;
                let available = step <= self.max_unlocked_step;
                let sense = if available {
                    Sense::click()
                } else {
                    Sense::hover()
                };
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 43.0), sense);
                let selected = ui.ctx().animate_bool_with_time(response.id.with("selection"), active, 0.18);
                let hover = ui.ctx().animate_bool_with_time(response.id.with("hover"), available && response.hovered(), 0.14);
                let tint = rgba(201, 208, 242, (25.0 * selected + 10.0 * hover) as u8);
                let rim = rgba(218, 226, 249, (38.0 * selected + 12.0 * hover) as u8);
                paint_liquid_glass(ui.painter(), rect, 12.0, tint, rim, 0.70 * selected + 0.30 * hover);
                let center = Pos2::new(rect.left() + 19.0, rect.center().y);
                let ink = if available { ACCENT_IRIS } else { FG_MUTED };
                paint_section_icon(ui.painter(), center, step, ink);
                ui.painter().text(
                    Pos2::new(rect.left() + 39.0, center.y),
                    egui::Align2::LEFT_CENTER,
                    *title,
                    font_semibold(14.0),
                    if active { FG_PRIMARY } else { FG_SECONDARY },
                );
                if self.step_is_done(step) {
                    paint_vec_check(
                        ui.painter(),
                        Pos2::new(rect.right() - 13.0, center.y),
                        6.0,
                        EMERALD_FG,
                    );
                }
                if available && response.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if available && response.clicked() {
                    self.current_step = step;
                }
            }

            // Empurra rodapé da sidebar para baixo
            let rem = (ui.available_height() - 68.0).max(8.0);
            ui.add_space(rem);

            let card_rect_w = ui.available_width();
            let (cred_rect, cred_resp) =
                ui.allocate_exact_size(Vec2::new(card_rect_w, 54.0), Sense::click());
            if cred_resp.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            let cred_bg = if cred_resp.hovered() {
                Color32::from_rgba_unmultiplied(214, 198, 255, 26)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 10)
            };
            if cred_resp.hovered() {
                ui.painter()
                    .rect_filled(cred_rect, Rounding::same(12.0), cred_bg);
            }

            let l1 = ui.painter().layout_no_wrap(
                "Feito Por Ryan".to_owned(),
                font_semibold(11.5),
                FG_SECONDARY,
            );
            let l2 = ui.painter().layout_no_wrap(
                "Comunidade no Discord ↗".to_owned(),
                font_regular(11.0),
                ACCENT_IRIS,
            );
            ui.painter().galley(
                Pos2::new(cred_rect.left() + 12.0, cred_rect.top() + 9.0),
                l1,
                FG_SECONDARY,
            );
            ui.painter().galley(
                Pos2::new(cred_rect.left() + 12.0, cred_rect.top() + 28.0),
                l2,
                ACCENT_IRIS,
            );
            if cred_resp.clicked() {
                let _ = commands::open_external_url(DISCORD_URL);
            }
        });
    }

    // ========================================================================
    // Grupos de verificações
    // ========================================================================

    fn diagnostic_collection(&self, ui: &mut egui::Ui, index: usize, time: f64) {
        let title = ["Conexão", "Dispositivo", "Serviços"][index];
        ui.label(RichText::new(title).font(font_semibold(19.0)).color(FG_PRIMARY));

        for item in &self.env_checks {
            let group = match item.id.as_str() {
                "network" | "adb" => 0,
                "emulator" | "root" | "runner" => 1,
                _ => 2,
            };
            if group != index {
                continue;
            }
            let status = match item.status {
                CheckStatus::Ok => "Pronto",
                CheckStatus::Warning => "Atenção",
                CheckStatus::Error => "Falha",
                CheckStatus::Running => "Verificando…",
                CheckStatus::Pending => "Aguardando",
            };
            // O nome completo e o resultado continuam disponíveis mesmo numa coluna estreita.
            let label = match item.id.as_str() {
                "adb" => "Android Debug Bridge",
                "emulator" => "Emulador Android",
                "root" => "Permissão root",
                "network" => "Rede e proxy",
                "google_one" => "Google One",
                "runner" => "Motor Android",
                _ => item.label.as_str(),
            };
            let (row, response) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), Sense::hover());
            let left = row.left() + 26.0;
            let text_rect = Rect::from_min_max(Pos2::new(left, row.top()), row.right_bottom());
            let p = ui
                .painter()
                .with_clip_rect(text_rect.intersect(ui.clip_rect()));
            paint_status_glyph(
                ui.painter(),
                Pos2::new(row.left() + 10.0, row.center().y),
                item.status,
                time,
            );
            p.text(
                Pos2::new(left, row.top() + 3.0),
                egui::Align2::LEFT_TOP,
                label,
                font_semibold(13.0),
                FG_PRIMARY,
            );
            let detail = item.detail.as_deref().map(clean_display_text);
            let secondary = detail
                .as_deref()
                .filter(|value| !value.is_empty())
                .unwrap_or(status);
            p.text(
                Pos2::new(left, row.top() + 23.0),
                egui::Align2::LEFT_TOP,
                secondary,
                font_regular(12.5),
                FG_MUTED,
            );
            ui.painter()
                .hline(row.x_range(), row.bottom(), Stroke::new(1.0, BORDER_SUBTLE));
            response.on_hover_ui(|ui| {
                ui.label(&item.label);
                ui.label(status);
                if let Some(detail) = &detail {
                    ui.label(detail);
                }
            });
        }
    }

    // ========================================================================
    // Etapas 1 a 4
    // ========================================================================

    fn render_step1(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, time: f64) {
        let is_ready = self.env_report.as_ref().map(|r| r.ready).unwrap_or(false);
        let completed_count = self
            .env_checks
            .iter()
            .filter(|c| c.status == CheckStatus::Ok)
            .count();

        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 12.0;
            responsive_row(
                ui,
                166.0,
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        RichText::new("Diagnóstico")
                            .font(font_semibold(31.0))
                            .color(FG_PRIMARY),
                    );
                    ui.label(
                        RichText::new("Ambiente do emulador Android")
                            .size(14.0)
                            .color(FG_MUTED),
                    );
                },
                |ui| {
                    let label = if self.is_checking_env {
                        "Verificando…"
                    } else if self.env_report.is_some() {
                        "Verificar novamente"
                    } else {
                        "Executar diagnóstico"
                    };
                    let variant = if is_ready {
                        BtnVariant::Secondary
                    } else {
                        BtnVariant::Primary
                    };
                    if aur_button(ui, label, variant, !self.is_checking_env) {
                        self.trigger_env_check(ctx);
                    }
                },
            );

            ui.add_space(2.0);
            let width = ui.available_width();
            let count = if width >= 510.0 {
                3
            } else if width >= 350.0 {
                2
            } else {
                1
            };
            for start in (0..3).step_by(count) {
                ui.columns(count, |columns| {
                    for (offset, column) in columns.iter_mut().take(3 - start).enumerate() {
                        column.spacing_mut().item_spacing.y = 3.0;
                        self.diagnostic_collection(column, start + offset, time);
                    }
                });
            }

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} de {} verificações concluídas com sucesso",
                        completed_count,
                        self.env_checks.len()
                    ))
                    .size(12.5)
                    .color(FG_SECONDARY),
                );
                if self.env_report.is_some() {
                    if is_ready {
                        draw_badge(ui, "Ambiente pronto", BadgeKind::Success);
                    } else {
                        draw_badge(ui, "Ambiente incompleto", BadgeKind::Warning);
                    }
                }
            });
            draw_progress_bar(
                ui,
                completed_count as f32 / self.env_checks.len().max(1) as f32,
            );

            if let Some(rep) = &self.env_report {
                if rep.serial.is_some() || rep.abi.is_some() {
                    Frame::none()
                        .fill(Color32::from_rgba_unmultiplied(158, 159, 246, 13))
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .rounding(Rounding::same(11.0))
                        .inner_margin(Margin::symmetric(12.0, 7.0))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "Dispositivo: {}   •   Arquitetura: {}   •   ADB: {}",
                                        rep.serial.as_deref().unwrap_or("—"),
                                        rep.abi.as_deref().unwrap_or("—"),
                                        rep.adb_path
                                            .as_deref()
                                            .map(clean_display_text)
                                            .unwrap_or_else(|| "—".to_string()),
                                    ))
                                    .font(font_mono(11.2))
                                    .color(FG_SECONDARY),
                                );
                            });
                        });
                }
            }

            if let Some(err) = &self.step1_error {
                Frame::none()
                    .fill(RED_BG)
                    .stroke(Stroke::new(1.0_f32, RED_BORDER))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(Margin::symmetric(13.0, 8.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(err).size(12.0).color(RED_FG));
                    });
            }

            ui.separator();

            responsive_row(
                ui,
                190.0,
                |ui| {
                    let hint = if is_ready {
                        "Tudo pronto! Avance para consultar as ofertas elegíveis da conta."
                    } else {
                        "Execute o diagnóstico acima para liberar a consulta de ofertas."
                    };
                    ui.label(RichText::new(hint).size(13.0).color(FG_MUTED));
                },
                |ui| {
                    if aur_button(ui, "Avançar para Ofertas  →", BtnVariant::Primary, is_ready) {
                        self.go_to_step(2);
                    }
                },
            );
        });
    }

    fn render_step2(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let raw_free_months: u32 = self
            .offers
            .iter()
            .filter(|o| o.is_free || o.micros == "0")
            .map(|o| o.months)
            .sum();
        let accumulable = raw_free_months.min(30);
        let pct = (accumulable as f32) / 30.0;

        ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 12.0;
                ui.label(RichText::new("Ofertas da conta").font(font_semibold(27.0)).color(FG_PRIMARY));

                responsive_row(                     ui,                     150.0,                     |ui| {                         ui.spacing_mut().item_spacing.y = 3.0;

                        ui.label(
                            RichText::new("Leitura segura na Google Play Store para listar promoções elegíveis sem iniciar cobrança.")
                                .size(12.0)
                                .color(FG_MUTED),
                        );                     },                     |ui| {                         let lbl = if self.is_loading_offers {
                            "Consultando..."
                        } else {
                            "Consultar ofertas"
                        };
                        if aur_button(ui, lbl, BtnVariant::Primary, !self.is_loading_offers) {
                            self.trigger_check_offers(ctx);
                        }                     },                 );

                Frame::none()
                    .fill(Color32::from_rgba_unmultiplied(158, 159, 246, 18))
                    .stroke(Stroke::new(
                        1.0_f32,
                        Color32::from_rgba_unmultiplied(158, 159, 246, 56),
                    ))
                    .rounding(Rounding::same(14.0))
                    .inner_margin(Margin::same(14.0))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 7.0;
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Total acumulável estimado (limite de 30 meses)")
                                    .font(font_semibold(13.0))
                                    .color(FG_PRIMARY),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                draw_badge(ui, "máx. 30m", BadgeKind::Outline);
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}",
                                        accumulable,
                                        if accumulable == 1 { "mês" } else { "meses" }
                                    ))
                                    .font(font_semibold(16.5))
                                    .color(ACCENT_IRIS),
                                );
                            });
                        });
                        draw_progress_bar(ui, pct);
                        ui.label(
                            RichText::new("Esta consulta apenas lê os dados de elegibilidade da conta logada no emulador.")
                                .size(11.5)
                                .color(FG_MUTED),
                        );
                    });

                if let Some(err) = &self.step2_error {
                    Frame::none()
                        .fill(RED_BG)
                        .stroke(Stroke::new(1.0_f32, RED_BORDER))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(Margin::symmetric(13.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new(err).size(12.0).color(RED_FG));
                        });
                }

                if self.offers.is_empty() {
                    Frame::none()
                        .fill(GLASS_CARD_IDLE)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .rounding(Rounding::same(15.0))
                        .inner_margin(Margin::symmetric(20.0, 28.0))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                let title = if self.is_loading_offers {
                                    "Buscando ofertas disponíveis na Google Play..."
                                } else if self.has_checked_offers {
                                    "Nenhuma oferta gratuita encontrada para esta conta."
                                } else {
                                    "Nenhuma oferta consultada ainda."
                                };
                                ui.label(
                                    RichText::new(title)
                                        .font(font_semibold(13.5))
                                        .color(FG_PRIMARY),
                                );
                                ui.label(
                                    RichText::new("As ofertas desta conta aparecem aqui após a consulta.")
                                        .size(12.0)
                                        .color(FG_MUTED),
                                );
                            });
                        });
                } else {
                    let avail_w = ui.available_width();
                    let gap = 10.0;
                    let columns = if avail_w >= 540.0 { 2 } else { 1 };
                    let col_w = (avail_w - gap * (columns - 1) as f32) / columns as f32;
                    for pair in self.offers.chunks(columns) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for offer in pair {
                                let is_free = offer.is_free || offer.micros == "0";
                                let (bg, border) = if is_free {
                                    (EMERALD_BG, EMERALD_BORDER)
                                } else {
                                    (GLASS_CARD_IDLE, BORDER_SUBTLE)
                                };
                                Frame::none()
                                    .fill(bg)
                                    .stroke(Stroke::new(1.0_f32, border))
                                    .rounding(Rounding::same(14.0))
                                    .inner_margin(Margin::same(13.0))
                                    .show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            ui.set_width((col_w - 26.0).max(0.0));
                                            ui.spacing_mut().item_spacing.y = 6.0;
                                            let title = if !offer.title.is_empty() {
                                                &offer.title
                                            } else if !offer.name.is_empty() {
                                                &offer.name
                                            } else {
                                                &offer.offer_id
                                            };
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(title)
                                                        .font(font_semibold(13.0))
                                                        .color(FG_PRIMARY),
                                                )
                                                .truncate(),
                                            );
                                            ui.horizontal_wrapped(|ui| {
                                                if is_free {
                                                    draw_badge(ui, "Gratuito", BadgeKind::Success);
                                                } else {
                                                    draw_badge(ui, &format_price(offer), BadgeKind::Secondary);
                                                }
                                                draw_badge(ui, &offer.offer_id, BadgeKind::Outline);
                                                draw_badge(
                                                    ui,
                                                    &format!(
                                                        "{} {}",
                                                        offer.months,
                                                        if offer.months == 1 { "mês" } else { "meses" }
                                                    ),
                                                    BadgeKind::Secondary,
                                                );
                                            });
                                            ui.label(
                                                RichText::new(format!(
                                                    "Valor: {}  •  Período: {}",
                                                    format_price(offer),
                                                    if offer.period.is_empty() {
                                                        format!("{}M", offer.months)
                                                    } else {
                                                        offer.period.clone()
                                                    }
                                                ))
                                                .size(11.2)
                                                .color(FG_MUTED),
                                            );
                                        });
                                    });
                            }
                        });
                    }
                }

                ui.separator();

                let can_next = !self.offers.is_empty() || self.has_checked_offers;
                ui.horizontal(|ui| {
                    if aur_button(ui, "←  Voltar", BtnVariant::Secondary, true) {
                        self.current_step = 1;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if aur_button(
                            ui,
                            "Iniciar Acumulação  →",
                            BtnVariant::Primary,
                            can_next,
                        ) {
                            self.go_to_step(3);
                        }
                    });
                });
            });
    }

    fn render_step3(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let accumulated_months: u32 = ACCUMULATION_STEPS
            .iter()
            .filter(|s| self.completed_sub_steps.contains(&s.id))
            .map(|s| s.months)
            .sum();
        let next_pending_id = ACCUMULATION_STEPS
            .iter()
            .find(|s| !self.completed_sub_steps.contains(&s.id))
            .map(|s| s.id);
        let all_completed = self.completed_sub_steps.len() == ACCUMULATION_STEPS.len();
        let can_finish = !self.completed_sub_steps.is_empty() && self.active_step_id.is_none();
        let pct = (accumulated_months as f32) / 30.0;

        ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 10.0;
                ui.label(RichText::new("Acumulação").font(font_semibold(27.0)).color(FG_PRIMARY));

                responsive_row(                     ui,                     300.0,                     |ui| {                         ui.spacing_mut().item_spacing.y = 3.0;

                        ui.label(
                            RichText::new("Execute em ordem sequencial. Confira R$ 0,00 no checkout do emulador antes de confirmar.")
                                .size(12.0)
                                .color(FG_MUTED),
                        );                     },                     |ui| {                         if self.active_step_id.is_some()
                            && aur_button(ui, "Cancelar Tarefa", BtnVariant::Destructive, true)
                        {
                            self.trigger_cancel_task(ctx);
                        }
                        draw_badge(
                            ui,
                            &format!("Acumulado: {} / 30 meses", accumulated_months),
                            BadgeKind::Default,
                        );                     },                 );

                Frame::none()
                    .fill(AMBER_BG)
                    .stroke(Stroke::new(1.0_f32, AMBER_BORDER))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(Margin::symmetric(13.0, 9.0))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("Atenção: sempre confira se o checkout na Play Store exibe 'R$ 0,00 hoje' antes de tocar em Assinar.")
                                .font(font_semibold(12.0))
                                .color(AMBER_FG),
                        );
                    });

                draw_progress_bar(ui, pct);

                if let Some(status) = &self.step3_status {
                    Frame::none()
                        .fill(SKY_BG)
                        .stroke(Stroke::new(1.0_f32, SKY_BORDER))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(Margin::symmetric(13.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new(status).size(12.0).color(SKY_FG));
                        });
                }

                if let Some(err) = &self.step3_error {
                    Frame::none()
                        .fill(RED_BG)
                        .stroke(Stroke::new(1.0_f32, RED_BORDER))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(Margin::symmetric(13.0, 8.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new(err).size(12.0).color(RED_FG));
                        });
                }

                let mut step_to_start: Option<StepId> = None;
                for step_def in &ACCUMULATION_STEPS {
                    let is_completed = self.completed_sub_steps.contains(&step_def.id);
                    let is_running = self.active_step_id == Some(step_def.id);
                    let is_next = next_pending_id == Some(step_def.id);
                    let can_start = !is_completed && self.active_step_id.is_none();

                    let (bg, border) = if is_completed {
                        (EMERALD_BG, EMERALD_BORDER)
                    } else if is_running {
                        (
                            Color32::from_rgba_unmultiplied(158, 159, 246, 30),
                            Color32::from_rgba_unmultiplied(158, 159, 246, 130),
                        )
                    } else if is_next {
                        (
                            Color32::from_rgba_unmultiplied(255, 255, 255, 16),
                            Color32::from_rgba_unmultiplied(158, 159, 246, 76),
                        )
                    } else {
                        (GLASS_CARD_IDLE, BORDER_SUBTLE)
                    };

                    Frame::none()
                        .fill(bg)
                        .stroke(Stroke::new(1.0_f32, border))
                        .rounding(Rounding::same(14.0))
                        .inner_margin(Margin::symmetric(14.0, 10.0))
                        .show(ui, |ui| {
                            responsive_row(                                 ui,                                 166.0,                                 |ui| {                                     ui.spacing_mut().item_spacing.y = 3.0;
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            RichText::new(step_def.title)
                                                .font(font_semibold(13.0))
                                                .color(FG_PRIMARY),
                                        );
                                        draw_badge(ui, step_def.offer_id, BadgeKind::Outline);
                                        if step_def.requires_token {
                                            draw_badge(
                                                ui,
                                                "Requer token anterior",
                                                BadgeKind::Secondary,
                                            );
                                        }
                                    });
                                    ui.label(
                                        RichText::new(step_def.subtitle)
                                            .size(11.5)
                                            .color(FG_MUTED),
                                    );                                 },                                 |ui| {                                     if is_completed {
                                        draw_badge(
                                            ui,
                                            &format!("Concluído (+{}m)", step_def.months),
                                            BadgeKind::Success,
                                        );
                                    } else if is_running {
                                        draw_badge(ui, "Abrindo checkout...", BadgeKind::Default);
                                    } else {
                                        let variant = if is_next {
                                            BtnVariant::Primary
                                        } else {
                                            BtnVariant::Secondary
                                        };
                                        if aur_button(
                                            ui,
                                            &format!("Executar Passo {}", step_def.order),
                                            variant,
                                            can_start,
                                        ) {
                                            step_to_start = Some(step_def.id);
                                        }
                                    }                                 },                             );
                        });
                }

                if let Some(sid) = step_to_start {
                    self.trigger_start_step(ctx, sid);
                }

                ui.separator();

                ui.horizontal(|ui| {
                    if aur_button(
                        ui,
                        "←  Voltar",
                        BtnVariant::Secondary,
                        self.active_step_id.is_none(),
                    ) {
                        self.current_step = 2;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let next_lbl = if all_completed {
                            "Ir para Finalização Segura  →"
                        } else {
                            "Finalizar com Período Atual  →"
                        };
                        let variant = if all_completed {
                            BtnVariant::Primary
                        } else {
                            BtnVariant::Secondary
                        };
                        if aur_button(ui, next_lbl, variant, can_finish) {
                            self.go_to_step(4);
                        }
                    });
                });
            });
    }

    fn render_step4(&mut self, ui: &mut egui::Ui) {
        let completed_defs: Vec<&AccumulationStepDef> = ACCUMULATION_STEPS
            .iter()
            .filter(|s| self.completed_sub_steps.contains(&s.id))
            .collect();
        let total_months: u32 = completed_defs.iter().map(|s| s.months).sum();

        ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 12.0;
                ui.label(RichText::new("Finalização").font(font_semibold(27.0)).color(FG_PRIMARY));

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;

                    ui.label(
                        RichText::new("Confira o resumo do período acumulado e desative a renovação automática na Google Play.")
                            .size(12.0)
                            .color(FG_MUTED),
                    );
                });

                Frame::none()
                    .fill(EMERALD_BG)
                    .stroke(Stroke::new(1.0_f32, EMERALD_BORDER))
                    .rounding(Rounding::same(15.0))
                    .inner_margin(Margin::same(16.0))
                    .show(ui, |ui| {
                        responsive_row(                             ui,                             180.0,                             |ui| {                                 ui.spacing_mut().item_spacing.y = 3.0;
                                ui.label(
                                    RichText::new("PERÍODO TOTAL ACUMULADO")
                                        .font(font_semibold(10.5))
                                        .color(EMERALD_FG),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} {} de Google AI Pro",
                                        total_months,
                                        if total_months == 1 { "mês" } else { "meses" }
                                    ))
                                    .font(font_semibold(21.0))
                                    .color(FG_PRIMARY),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} de {} etapas de acumulação concluídas",
                                        completed_defs.len(),
                                        ACCUMULATION_STEPS.len()
                                    ))
                                    .size(11.8)
                                    .color(FG_MUTED),
                                );                             },                             |ui| {                                 draw_badge(ui, "Pronto para proteger conta", BadgeKind::Success);                             },                         );
                    });

                Frame::none()
                    .fill(Color32::from_rgba_unmultiplied(158, 159, 246, 18))
                    .stroke(Stroke::new(
                        1.0_f32,
                        Color32::from_rgba_unmultiplied(158, 159, 246, 64),
                    ))
                    .rounding(Rounding::same(15.0))
                    .inner_margin(Margin::same(15.0))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        ui.label(
                            RichText::new("Passo obrigatório: Cancelar renovação automática")
                                .font(font_semibold(13.5))
                                .color(FG_PRIMARY),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Ao cancelar a assinatura na página da Google Play, você mantém todo o período acumulado ({} meses) ativo até a data de expiração, garantindo cobrança zero no futuro.",
                                total_months
                            ))
                            .size(12.0)
                            .color(FG_MUTED),
                        );
                        ui.horizontal_wrapped(|ui| {
                            if aur_button(
                                ui,
                                "Abrir Assinaturas da Google Play  ↗",
                                BtnVariant::Primary,
                                true,
                            ) {
                                match commands::open_external_url(SUBSCRIPTIONS_URL) {
                                    Ok(()) => self.append_log(
                                        "ok",
                                        "Página de assinaturas da Google Play aberta.",
                                    ),
                                    Err(e) => self.append_log("error", e),
                                }
                            }
                            ui.label(
                                RichText::new("play.google.com/store/account/subscriptions")
                                    .size(11.2)
                                    .color(FG_MUTED),
                            );
                        });
                    });

                ui.separator();

                ui.horizontal(|ui| {
                    if aur_button(ui, "←  Voltar para Acumulação", BtnVariant::Secondary, true) {
                        self.current_step = 3;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if aur_button(ui, "Reiniciar Assistente", BtnVariant::Ghost, true) {
                            self.reset_wizard();
                        }
                    });
                });
            });
    }

    // ========================================================================
    // Console de Atividade Liquid Glass (com chevron vetorial)
    // ========================================================================

    fn render_log_console(&mut self, ui: &mut egui::Ui) {
        let error_count = self.logs.iter().filter(|l| l.level == "error").count();

        Frame::none()
            .fill(GLASS_LOG_BG)
            .stroke(Stroke::new(1.0_f32, BORDER_GLASS))
            .rounding(Rounding::same(14.0))
            .inner_margin(Margin::symmetric(14.0, 9.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (chev_rect, chev_resp) =
                        ui.allocate_exact_size(Vec2::new(84.0, 22.0), Sense::click());
                    if chev_resp.hovered() {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    paint_vec_chevron(
                        ui.painter(),
                        Pos2::new(chev_rect.left() + 6.0, chev_rect.center().y),
                        self.logs_open,
                        ACCENT_IRIS,
                    );
                    let ag = ui.painter().layout_no_wrap(
                        "Atividade".to_owned(),
                        font_semibold(12.5),
                        FG_PRIMARY,
                    );
                    ui.painter().galley(
                        Pos2::new(
                            chev_rect.left() + 18.0,
                            chev_rect.center().y - ag.size().y * 0.5,
                        ),
                        ag,
                        FG_PRIMARY,
                    );
                    if chev_resp.clicked() {
                        self.logs_open = !self.logs_open;
                    }

                    draw_badge(
                        ui,
                        &format!(
                            "{} {}",
                            self.logs.len(),
                            if self.logs.len() == 1 {
                                "registro"
                            } else {
                                "registros"
                            }
                        ),
                        BadgeKind::Outline,
                    );
                    if error_count > 0 {
                        draw_badge(
                            ui,
                            &format!(
                                "{} {}",
                                error_count,
                                if error_count == 1 { "erro" } else { "erros" }
                            ),
                            BadgeKind::Destructive,
                        );
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !self.logs.is_empty()
                            && aur_button(ui, "Limpar", BtnVariant::Ghost, true)
                        {
                            self.logs.clear();
                        }
                    });
                });

                if self.logs_open {
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt("aur_log_scroll")
                        .max_height(125.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            if self.logs.is_empty() {
                                ui.label(
                                    RichText::new("Nenhum evento registrado ainda.")
                                        .size(11.8)
                                        .color(FG_MUTED),
                                );
                            } else {
                                for entry in &self.logs {
                                    let (lbl, col) = match entry.level.as_str() {
                                        "ok" => ("OK", EMERALD_FG),
                                        "warn" => ("AVISO", AMBER_FG),
                                        "error" => ("ERRO", RED_FG),
                                        _ => ("INFO", SKY_FG),
                                    };
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            RichText::new(&entry.timestamp)
                                                .font(font_mono(11.0))
                                                .color(FG_MUTED),
                                        );
                                        ui.label(
                                            RichText::new(format!("[{}]", lbl))
                                                .font(font_mono(11.0))
                                                .color(col),
                                        );
                                        ui.label(
                                            RichText::new(&entry.message)
                                                .font(font_mono(11.2))
                                                .color(FG_SECONDARY),
                                        );
                                    });
                                }
                            }
                        });
                }
            });
    }

    // ========================================================================
    // Bordas de Redimensionamento Invisíveis para Janela Frameless
    // ========================================================================

    fn handle_resize_grips(&self, ui: &mut egui::Ui, ctx: &egui::Context, win_rect: Rect) {
        if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
            return;
        }
        let g = 5.0;
        let grips: [(Rect, ResizeDirection, CursorIcon, &str); 8] = [
            (
                Rect::from_min_max(win_rect.min, Pos2::new(win_rect.left() + g * 2.0, win_rect.top() + g * 2.0)),
                ResizeDirection::NorthWest,
                CursorIcon::ResizeNwSe,
                "rz_nw",
            ),
            (
                Rect::from_min_max(Pos2::new(win_rect.right() - g * 2.0, win_rect.top()), Pos2::new(win_rect.right(), win_rect.top() + g * 2.0)),
                ResizeDirection::NorthEast,
                CursorIcon::ResizeNeSw,
                "rz_ne",
            ),
            (
                Rect::from_min_max(Pos2::new(win_rect.left(), win_rect.bottom() - g * 2.0), Pos2::new(win_rect.left() + g * 2.0, win_rect.bottom())),
                ResizeDirection::SouthWest,
                CursorIcon::ResizeNeSw,
                "rz_sw",
            ),
            (
                Rect::from_min_max(Pos2::new(win_rect.right() - g * 2.0, win_rect.bottom() - g * 2.0), win_rect.max),
                ResizeDirection::SouthEast,
                CursorIcon::ResizeNwSe,
                "rz_se",
            ),
            (
                Rect::from_min_max(Pos2::new(win_rect.left() + g * 2.0, win_rect.bottom() - g), Pos2::new(win_rect.right() - g * 2.0, win_rect.bottom())),
                ResizeDirection::South,
                CursorIcon::ResizeVertical,
                "rz_s",
            ),
            (
                Rect::from_min_max(Pos2::new(win_rect.left(), win_rect.top() + g * 2.0), Pos2::new(win_rect.left() + g, win_rect.bottom() - g * 2.0)),
                ResizeDirection::West,
                CursorIcon::ResizeHorizontal,
                "rz_w",
            ),
            (
                Rect::from_min_max(Pos2::new(win_rect.right() - g, win_rect.top() + g * 2.0), Pos2::new(win_rect.right(), win_rect.bottom() - g * 2.0)),
                ResizeDirection::East,
                CursorIcon::ResizeHorizontal,
                "rz_e",
            ),
            (
                Rect::from_min_max(Pos2::new(win_rect.left() + g * 2.0, win_rect.top()), Pos2::new(win_rect.right() - g * 2.0, win_rect.top() + g)),
                ResizeDirection::North,
                CursorIcon::ResizeVertical,
                "rz_n",
            ),
        ];

        for (r, dir, cur, id) in grips {
            let resp = ui.interact(r, egui::Id::new(id), Sense::drag());
            if resp.hovered() {
                ctx.set_cursor_icon(cur);
            }
            if resp.drag_started() {
                ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
            }
        }
    }
}

impl eframe::App for AureonApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(windows)]
        if !self.glass_applied {
            self.glass_applied = crate::win_glass::apply_liquid_glass_to_process_window();
        }

        self.drain_messages();

        let any_spinning =
            self.is_checking_env || self.is_loading_offers || self.active_step_id.is_some();
        if any_spinning {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        let time = ctx.input(|i| i.time);

        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                let win_rect = ui.max_rect();

                ui.spacing_mut().item_spacing = Vec2::ZERO;

                // 1. Barra de título integrada frameless
                self.render_titlebar(ui, ctx);

                // 2. Corpo Edge-to-Edge: Sidebar Liquid Glass + Área de Trabalho
                let body_h = ui.available_height();
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;

                    // Navegação lateral
                    Frame::none()
                        .fill(GLASS_SIDEBAR_BG)
                        .inner_margin(Margin::symmetric(14.0, 14.0))
                        .show(ui, |ui| {
                            ui.set_width(154.0);
                            ui.set_min_height((body_h - 28.0).max(100.0));
                            self.render_sidebar(ui);
                        });

                    // Divisor vertical sutil
                    let sep_x = ui.cursor().min.x;
                    let workspace = Rect::from_min_max(Pos2::new(sep_x, win_rect.top() + 46.0), win_rect.right_bottom());
                    ui.painter().rect_filled(workspace, Rounding::ZERO, GLASS_WINDOW_BG);
                    ui.painter().line_segment(
                        [
                            Pos2::new(sep_x, win_rect.top() + 46.0),
                            Pos2::new(sep_x, win_rect.bottom()),
                        ],
                        Stroke::new(1.0_f32, BORDER_SUBTLE),
                    );

                    // Área principal direita com scroll suave
                    Frame::none()
                        .inner_margin(Margin::symmetric(22.0, 18.0))
                        .show(ui, |ui| {
                            let content_width = ui.available_width();
                            ui.set_width(content_width);
                            ui.set_min_height((body_h - 36.0).max(100.0));

                            // The horizontal shell only positions the sidebar and workspace.
                            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt(("aur_main_scroll", self.current_step))
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.spacing_mut().item_spacing = Vec2::new(9.0, 12.0);

                                        if self.displayed_step != self.current_step {
                                            self.displayed_step = self.current_step;
                                            self.transition_start = Some(time);
                                        }
                                        if let Some(start) = self.transition_start {
                                            let t = ((time - start) / 0.18).clamp(0.0, 1.0) as f32;
                                            ui.set_opacity(1.0 - 0.6 * (1.0 - t).powi(3));
                                            if t < 1.0 {
                                                ctx.request_repaint();
                                            } else {
                                                self.transition_start = None;
                                            }
                                        }

                                        match self.current_step {
                                            1 => self.render_step1(ui, ctx, time),
                                            2 => self.render_step2(ui, ctx),
                                            3 => self.render_step3(ui, ctx),
                                            _ => self.render_step4(ui),
                                        }

                                        self.render_log_console(ui);
                                    });
                            });
                        });
                });

                // Moldura externa cristalina 1px ao redor da janela inteira
                ui.painter().rect_stroke(
                    win_rect.shrink(0.5),
                    Rounding::ZERO,
                    Stroke::new(1.0_f32, BORDER_OUTER_WINDOW),
                );

                // Grips de redimensionamento
                self.handle_resize_grips(ui, ctx, win_rect);
            });
    }
}
