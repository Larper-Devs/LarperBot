use ab_glyph::{FontArc, PxScale};
use font8x8::{UnicodeFonts, BASIC_FONTS, LATIN_FONTS};
use futures::future::join_all;
use image::{
    imageops::{resize, FilterType},
    DynamicImage, ImageBuffer, ImageFormat, Rgba, RgbaImage,
};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_line_segment_mut, draw_polygon_mut, draw_text_mut, text_size,
};
use imageproc::point::Point;
use reqwest::Client;
use std::{error::Error, f32::consts::PI, io::Cursor, sync::OnceLock};
use tracing::warn;

const WIDTH: u32 = 1_000;
const HEIGHT: u32 = 430;

pub const PROFILE_THEMES: &[(&str, &str)] = &[
    ("midnight", "Midnight"),
    ("aurora", "Aurora"),
    ("sunset", "Sunset"),
    ("cyber", "Cyber"),
    ("ocean", "Ocean"),
    ("forest", "Forest"),
    ("rose", "Rose"),
    ("gold", "Gold"),
    ("matrix", "Matrix"),
];

pub fn is_profile_theme(theme: &str) -> bool {
    PROFILE_THEMES.iter().any(|(value, _)| *value == theme)
}

#[derive(Clone, Copy)]
struct Palette {
    background: Rgba<u8>,
    background_light: Rgba<u8>,
    card: Rgba<u8>,
    purple: Rgba<u8>,
    purple_light: Rgba<u8>,
    white: Rgba<u8>,
    muted: Rgba<u8>,
    track: Rgba<u8>,
    border: Rgba<u8>,
    milestone: Rgba<u8>,
    icon: Rgba<u8>,
    grid: Rgba<u8>,
}

fn palette(background: &str) -> Palette {
    let standard = Palette {
        background: Rgba([22, 10, 43, 255]),
        background_light: Rgba([43, 20, 84, 255]),
        card: Rgba([41, 19, 77, 240]),
        purple: Rgba([139, 92, 246, 255]),
        purple_light: Rgba([196, 181, 253, 255]),
        white: Rgba([255, 255, 255, 255]),
        muted: Rgba([216, 204, 245, 255]),
        track: Rgba([39, 52, 77, 255]),
        border: Rgba([88, 62, 88, 180]),
        milestone: Rgba([39, 49, 73, 255]),
        icon: Rgba([166, 178, 202, 255]),
        grid: Rgba([72, 66, 100, 80]),
    };

    match background {
        "aurora" => Palette {
            background: Rgba([7, 31, 44, 255]),
            background_light: Rgba([27, 25, 79, 255]),
            card: Rgba([18, 34, 69, 240]),
            border: Rgba([52, 86, 106, 180]),
            ..standard
        },
        "sunset" => Palette {
            background: Rgba([48, 13, 35, 255]),
            background_light: Rgba([92, 33, 54, 255]),
            card: Rgba([67, 24, 59, 240]),
            purple: Rgba([244, 114, 182, 255]),
            purple_light: Rgba([251, 207, 232, 255]),
            track: Rgba([78, 45, 78, 255]),
            border: Rgba([111, 59, 83, 180]),
            ..standard
        },
        "cyber" => Palette {
            background: Rgba([5, 18, 29, 255]),
            background_light: Rgba([12, 53, 68, 255]),
            card: Rgba([10, 28, 43, 240]),
            purple: Rgba([34, 211, 238, 255]),
            purple_light: Rgba([165, 243, 252, 255]),
            track: Rgba([22, 78, 99, 255]),
            border: Rgba([34, 102, 117, 180]),
            ..standard
        },
        "ocean" => Palette {
            background: Rgba([5, 19, 34, 255]),
            background_light: Rgba([10, 67, 93, 255]),
            card: Rgba([14, 37, 58, 240]),
            purple: Rgba([56, 189, 248, 255]),
            purple_light: Rgba([186, 230, 253, 255]),
            track: Rgba([28, 67, 91, 255]),
            border: Rgba([47, 103, 127, 180]),
            ..standard
        },
        "forest" => Palette {
            background: Rgba([7, 24, 20, 255]),
            background_light: Rgba([22, 70, 45, 255]),
            card: Rgba([17, 43, 34, 240]),
            purple: Rgba([74, 222, 128, 255]),
            purple_light: Rgba([187, 247, 208, 255]),
            track: Rgba([30, 68, 52, 255]),
            border: Rgba([52, 107, 75, 180]),
            ..standard
        },
        "rose" => Palette {
            background: Rgba([38, 10, 29, 255]),
            background_light: Rgba([91, 28, 59, 255]),
            card: Rgba([59, 23, 49, 240]),
            purple: Rgba([244, 114, 182, 255]),
            purple_light: Rgba([251, 207, 232, 255]),
            track: Rgba([88, 42, 79, 255]),
            border: Rgba([119, 57, 92, 180]),
            ..standard
        },
        "gold" => Palette {
            background: Rgba([34, 22, 7, 255]),
            background_light: Rgba([91, 58, 12, 255]),
            card: Rgba([56, 38, 18, 240]),
            purple: Rgba([245, 158, 11, 255]),
            purple_light: Rgba([253, 230, 138, 255]),
            track: Rgba([86, 62, 25, 255]),
            border: Rgba([124, 91, 37, 180]),
            ..standard
        },
        "matrix" => Palette {
            background: Rgba([3, 18, 12, 255]),
            background_light: Rgba([7, 67, 37, 255]),
            card: Rgba([7, 38, 26, 240]),
            purple: Rgba([34, 197, 94, 255]),
            purple_light: Rgba([187, 247, 208, 255]),
            track: Rgba([19, 68, 43, 255]),
            border: Rgba([34, 105, 63, 180]),
            ..standard
        },
        _ => standard,
    }
}

fn rounded_rect_contains(
    px: i32,
    py: i32,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    radius: i32,
) -> bool {
    let max_x = x + width as i32 - 1;
    let max_y = y + height as i32 - 1;
    let radius = radius
        .max(0)
        .min(width.min(height).saturating_sub(1) as i32 / 2);
    let nearest_x = px.clamp(x + radius, max_x - radius);
    let nearest_y = py.clamp(y + radius, max_y - radius);
    let dx = px - nearest_x;
    let dy = py - nearest_y;
    radius == 0 || dx * dx + dy * dy <= radius * radius
}

pub struct ProfileCardInput {
    pub username: String,
    pub avatar_url: String,
    pub level: i64,
    pub total_xp: i64,
    pub rank: i64,
    pub background: String,
}

pub struct RankingEntry {
    pub username: String,
    pub avatar_url: String,
    pub xp: i64,
}

pub struct RankingCardInput {
    pub period: String,
    pub entries: Vec<RankingEntry>,
    pub background: String,
}

struct FontSet {
    regular: FontArc,
    bold: FontArc,
}

static FONTS: OnceLock<Option<FontSet>> = OnceLock::new();

fn load_font(paths: &[&str]) -> Option<FontArc> {
    paths.iter().find_map(|path| {
        std::fs::read(path)
            .ok()
            .and_then(|data| FontArc::try_from_vec(data).ok())
    })
}

fn fonts() -> Option<&'static FontSet> {
    FONTS
        .get_or_init(|| {
            let regular = load_font(&[
                r"C:\Windows\Fonts\segoeui.ttf",
                r"C:\Windows\Fonts\arial.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
            ])?;
            let bold = load_font(&[
                r"C:\Windows\Fonts\segoeuib.ttf",
                r"C:\Windows\Fonts\arialbd.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
                "/usr/share/fonts/truetype/liberation2/LiberationSans-Bold.ttf",
            ])
            .unwrap_or_else(|| regular.clone());
            Some(FontSet { regular, bold })
        })
        .as_ref()
}

fn blend_pixel(destination: &mut Rgba<u8>, source: Rgba<u8>) {
    let source_alpha = source[3] as f32 / 255.0;
    if source_alpha >= 1.0 {
        *destination = source;
        return;
    }
    if source_alpha <= 0.0 {
        return;
    }

    let destination_alpha = destination[3] as f32 / 255.0;
    let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    for channel in 0..3 {
        let value = (source[channel] as f32 * source_alpha
            + destination[channel] as f32 * destination_alpha * (1.0 - source_alpha))
            / output_alpha;
        destination[channel] = value.round().clamp(0.0, 255.0) as u8;
    }
    destination[3] = (output_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
}

fn draw_gradient(canvas: &mut RgbaImage, colors: Palette) {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let amount = (x as f32 + y as f32) / (WIDTH + HEIGHT) as f32;
            let mut pixel = Rgba([0, 0, 0, 255]);
            for channel in 0..3 {
                pixel[channel] = (colors.background[channel] as f32
                    + (colors.background_light[channel] as f32 - colors.background[channel] as f32)
                        * amount)
                    .round() as u8;
            }
            canvas.put_pixel(x, y, pixel);
        }
    }
}

fn draw_background_grid(canvas: &mut RgbaImage, color: Rgba<u8>) {
    for y in (248..HEIGHT).step_by(28) {
        draw_line_segment_mut(canvas, (0.0, y as f32), (WIDTH as f32, y as f32), color);
    }
    for x in (0..WIDTH).step_by(36) {
        draw_line_segment_mut(canvas, (x as f32, 248.0), (x as f32, HEIGHT as f32), color);
    }
}

fn blend_circle_clipped(
    canvas: &mut RgbaImage,
    center: (i32, i32),
    radius: i32,
    color: Rgba<u8>,
    clip: (i32, i32, u32, u32, i32),
) {
    let radius_squared = radius * radius;
    let left = (center.0 - radius).max(clip.0).max(0);
    let right = (center.0 + radius).min(clip.0 + clip.2 as i32 - 1);
    let top = (center.1 - radius).max(clip.1).max(0);
    let bottom = (center.1 + radius).min(clip.1 + clip.3 as i32 - 1);
    for y in top..=bottom {
        for x in left..=right {
            let dx = x - center.0;
            let dy = y - center.1;
            if dx * dx + dy * dy <= radius_squared
                && rounded_rect_contains(x, y, clip.0, clip.1, clip.2, clip.3, clip.4)
            {
                blend_pixel(canvas.get_pixel_mut(x as u32, y as u32), color);
            }
        }
    }
}

fn draw_profile_surface(canvas: &mut RgbaImage, colors: Palette) {
    let card = (24, 24, WIDTH - 48, HEIGHT - 48, 26);
    rounded_rect(
        canvas,
        card.0,
        card.1,
        card.2,
        card.3,
        card.4,
        colors.border,
    );
    rounded_rect(
        canvas,
        card.0 + 1,
        card.1 + 1,
        card.2 - 2,
        card.3 - 2,
        card.4 - 1,
        colors.card,
    );

    blend_circle_clipped(
        canvas,
        (790, 38),
        215,
        Rgba([174, 74, 80, 34]),
        (card.0 + 1, card.1 + 1, card.2 - 2, card.3 - 2, card.4 - 1),
    );
    blend_circle_clipped(
        canvas,
        (140, 114),
        160,
        Rgba([139, 92, 246, 25]),
        (card.0 + 1, card.1 + 1, card.2 - 2, card.3 - 2, card.4 - 1),
    );
}

fn blend_circle(canvas: &mut RgbaImage, center: (i32, i32), radius: i32, color: Rgba<u8>) {
    let radius_squared = radius * radius;
    let left = (center.0 - radius).max(0);
    let right = (center.0 + radius).min(canvas.width() as i32 - 1);
    let top = (center.1 - radius).max(0);
    let bottom = (center.1 + radius).min(canvas.height() as i32 - 1);
    for y in top..=bottom {
        for x in left..=right {
            let dx = x - center.0;
            let dy = y - center.1;
            if dx * dx + dy * dy <= radius_squared {
                blend_pixel(canvas.get_pixel_mut(x as u32, y as u32), color);
            }
        }
    }
}

fn rounded_rect(
    canvas: &mut RgbaImage,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    radius: i32,
    color: Rgba<u8>,
) {
    if width == 0 || height == 0 {
        return;
    }
    let max_x = x + width as i32 - 1;
    let max_y = y + height as i32 - 1;
    let radius = radius
        .max(0)
        .min(width.min(height).saturating_sub(1) as i32 / 2);
    let left = x.max(0);
    let right = max_x.min(canvas.width() as i32 - 1);
    let top = y.max(0);
    let bottom = max_y.min(canvas.height() as i32 - 1);
    if left > right || top > bottom {
        return;
    }

    for py in top..=bottom {
        for px in left..=right {
            let nearest_x = px.clamp(x + radius, max_x - radius);
            let nearest_y = py.clamp(y + radius, max_y - radius);
            let dx = px - nearest_x;
            let dy = py - nearest_y;
            if radius == 0 || dx * dx + dy * dy <= radius * radius {
                blend_pixel(canvas.get_pixel_mut(px as u32, py as u32), color);
            }
        }
    }
}

fn circle_ring(
    canvas: &mut RgbaImage,
    center: (i32, i32),
    outer_radius: i32,
    inner_radius: i32,
    color: Rgba<u8>,
) {
    let outer_squared = outer_radius * outer_radius;
    let inner_squared = inner_radius.max(0) * inner_radius.max(0);
    let left = (center.0 - outer_radius).max(0);
    let right = (center.0 + outer_radius).min(canvas.width() as i32 - 1);
    let top = (center.1 - outer_radius).max(0);
    let bottom = (center.1 + outer_radius).min(canvas.height() as i32 - 1);
    for y in top..=bottom {
        for x in left..=right {
            let dx = x - center.0;
            let dy = y - center.1;
            let distance = dx * dx + dy * dy;
            if distance <= outer_squared && distance >= inner_squared {
                blend_pixel(canvas.get_pixel_mut(x as u32, y as u32), color);
            }
        }
    }
}

fn placeholder_avatar(size: u32, colors: Palette) -> RgbaImage {
    let mut avatar = ImageBuffer::from_pixel(size, size, colors.card);
    draw_filled_circle_mut(
        &mut avatar,
        (size as i32 / 2, size as i32 / 2),
        size as i32 / 2,
        colors.purple,
    );
    avatar
}

async fn load_avatar(client: &Client, url: &str, size: u32, colors: Palette) -> RgbaImage {
    let result = async {
        let response = client.get(url).send().await?.error_for_status()?;
        let bytes = response.bytes().await?;
        Ok::<DynamicImage, Box<dyn Error + Send + Sync>>(image::load_from_memory(&bytes)?)
    }
    .await;

    match result {
        Ok(image) => resize(&image.to_rgba8(), size, size, FilterType::Lanczos3),
        Err(error) => {
            warn!("Não foi possível carregar o avatar do perfil: {error}");
            placeholder_avatar(size, colors)
        }
    }
}

fn fit_text(font: &FontArc, scale: PxScale, text: &str, max_width: u32) -> String {
    if text_size(scale, font, text).0 <= max_width {
        return text.to_string();
    }
    let mut result = text.to_string();
    while result.chars().count() > 3 {
        let candidate = format!("{result}...");
        if text_size(scale, font, &candidate).0 <= max_width {
            return candidate;
        }
        result.pop();
    }
    "...".into()
}

fn draw_bitmap_text(
    canvas: &mut RgbaImage,
    text: &str,
    x: i32,
    y: i32,
    size: f32,
    color: Rgba<u8>,
    max_width: u32,
) {
    let pixel_size = (size / 8.0).round().max(1.0) as u32;
    let advance = pixel_size * 9;
    let max_chars = (max_width / advance).max(1) as usize;
    let mut chars: Vec<char> = text.chars().collect();
    if chars.len() > max_chars {
        if max_chars > 3 {
            chars.truncate(max_chars - 3);
            chars.extend(['.', '.', '.']);
        } else {
            chars.truncate(max_chars);
        }
    }

    for (index, character) in chars.into_iter().enumerate() {
        let glyph = LATIN_FONTS
            .get(character)
            .or_else(|| BASIC_FONTS.get(character))
            .unwrap_or_else(|| BASIC_FONTS.get('?').expect("bitmap fallback has ? glyph"));
        let origin_x = x + index as i32 * advance as i32;
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..8 {
                if bits & (1 << column) == 0 {
                    continue;
                }
                for dy in 0..pixel_size {
                    for dx in 0..pixel_size {
                        let px = origin_x + column * pixel_size as i32 + dx as i32;
                        let py = y + row as i32 * pixel_size as i32 + dy as i32;
                        if px >= 0
                            && py >= 0
                            && px < canvas.width() as i32
                            && py < canvas.height() as i32
                        {
                            blend_pixel(canvas.get_pixel_mut(px as u32, py as u32), color);
                        }
                    }
                }
            }
        }
    }
}

fn draw_text_fit(
    canvas: &mut RgbaImage,
    text: &str,
    x: i32,
    y: i32,
    size: f32,
    color: Rgba<u8>,
    max_width: u32,
    bold: bool,
) {
    if let Some(fonts) = fonts() {
        let scale = PxScale::from(size);
        let font = if bold { &fonts.bold } else { &fonts.regular };
        let text = fit_text(font, scale, text, max_width);
        draw_text_mut(canvas, color, x, y, scale, font, &text);
    } else {
        draw_bitmap_text(canvas, text, x, y, size, color, max_width);
    }
}

fn draw_text_right_fit(
    canvas: &mut RgbaImage,
    text: &str,
    right: i32,
    y: i32,
    size: f32,
    color: Rgba<u8>,
    max_width: u32,
    bold: bool,
) {
    if let Some(fonts) = fonts() {
        let scale = PxScale::from(size);
        let font = if bold { &fonts.bold } else { &fonts.regular };
        let text = fit_text(font, scale, text, max_width);
        let width = text_size(scale, font, &text).0 as i32;
        draw_text_mut(canvas, color, right - width, y, scale, font, &text);
    } else {
        let pixel_size = (size / 8.0).round().max(1.0) as i32;
        let width = text.chars().count() as i32 * pixel_size * 9;
        draw_bitmap_text(canvas, text, right - width, y, size, color, max_width);
    }
}

fn icon_line(canvas: &mut RgbaImage, start: (f32, f32), end: (f32, f32), color: Rgba<u8>) {
    for (offset_x, offset_y) in [(-1.0, 0.0), (0.0, -1.0), (0.0, 0.0), (1.0, 0.0), (0.0, 1.0)] {
        draw_line_segment_mut(
            canvas,
            (start.0 + offset_x, start.1 + offset_y),
            (end.0 + offset_x, end.1 + offset_y),
            color,
        );
    }
    draw_filled_circle_mut(canvas, (start.0 as i32, start.1 as i32), 2, color);
    draw_filled_circle_mut(canvas, (end.0 as i32, end.1 as i32), 2, color);
}

#[allow(dead_code)]
fn draw_people_icon(canvas: &mut RgbaImage, center: (i32, i32), colors: Palette) {
    let (x, y) = center;
    let purple = colors.purple;
    let cyan = Rgba([74, 222, 255, 255]);
    let green = Rgba([62, 211, 147, 255]);
    draw_filled_circle_mut(canvas, (x - 5, y - 6), 4, purple);
    draw_filled_circle_mut(canvas, (x + 7, y - 5), 4, cyan);
    rounded_rect(canvas, x - 12, y - 1, 13, 8, 4, purple);
    rounded_rect(canvas, x + 1, y, 13, 8, 4, green);
    icon_line(
        canvas,
        (x as f32 - 1.0, y as f32 - 2.0),
        (x as f32 + 2.0, y as f32 - 2.0),
        colors.white,
    );
}

#[allow(dead_code)]
fn draw_crown_icon(canvas: &mut RgbaImage, center: (i32, i32), color: Rgba<u8>) {
    let (x, y) = center;
    icon_line(
        canvas,
        ((x - 11) as f32, (y - 6) as f32),
        ((x - 7) as f32, (y + 3) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x - 7) as f32, (y + 3) as f32),
        ((x + 7) as f32, (y + 3) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x + 7) as f32, (y + 3) as f32),
        ((x + 11) as f32, (y - 6) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x - 11) as f32, (y - 6) as f32),
        (x as f32, (y - 1) as f32),
        color,
    );
    icon_line(
        canvas,
        (x as f32, (y - 1) as f32),
        ((x + 11) as f32, (y - 6) as f32),
        color,
    );
    draw_filled_circle_mut(canvas, (x - 11, y - 6), 2, color);
    draw_filled_circle_mut(canvas, (x, y - 1), 2, color);
    draw_filled_circle_mut(canvas, (x + 11, y - 6), 2, color);
    rounded_rect(canvas, x - 8, y + 5, 16, 3, 1, color);
}

#[allow(dead_code)]
fn draw_building_icon(canvas: &mut RgbaImage, center: (i32, i32), color: Rgba<u8>) {
    let (x, y) = center;
    icon_line(
        canvas,
        ((x - 10) as f32, (y + 10) as f32),
        ((x - 10) as f32, (y - 4) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x - 10) as f32, (y - 4) as f32),
        ((x - 4) as f32, (y - 4) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x - 4) as f32, (y - 4) as f32),
        ((x - 4) as f32, (y - 9) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x - 4) as f32, (y - 9) as f32),
        ((x + 4) as f32, (y - 9) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x + 4) as f32, (y - 9) as f32),
        ((x + 4) as f32, (y - 3) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x + 4) as f32, (y - 3) as f32),
        ((x + 10) as f32, (y - 3) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x + 10) as f32, (y - 3) as f32),
        ((x + 10) as f32, (y + 10) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x - 13) as f32, (y + 10) as f32),
        ((x + 13) as f32, (y + 10) as f32),
        color,
    );
    rounded_rect(canvas, x - 5, y + 2, 4, 5, 1, color);
    rounded_rect(canvas, x + 2, y + 2, 4, 5, 1, color);
}

#[allow(dead_code)]
fn draw_scroll_icon(canvas: &mut RgbaImage, center: (i32, i32), color: Rgba<u8>) {
    let (x, y) = center;
    icon_line(
        canvas,
        ((x - 8) as f32, (y - 10) as f32),
        ((x + 6) as f32, (y - 10) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x + 6) as f32, (y - 10) as f32),
        ((x + 8) as f32, (y - 8) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x + 8) as f32, (y - 8) as f32),
        ((x + 8) as f32, (y + 8) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x + 8) as f32, (y + 8) as f32),
        ((x + 6) as f32, (y + 10) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x + 6) as f32, (y + 10) as f32),
        ((x - 8) as f32, (y + 10) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x - 8) as f32, (y + 10) as f32),
        ((x - 8) as f32, (y - 10) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x - 3) as f32, (y - 4) as f32),
        ((x + 4) as f32, (y - 4) as f32),
        color,
    );
    icon_line(
        canvas,
        ((x - 3) as f32, (y) as f32),
        ((x + 3) as f32, y as f32),
        color,
    );
    icon_line(
        canvas,
        ((x - 3) as f32, (y + 4) as f32),
        ((x + 2) as f32, (y + 4) as f32),
        color,
    );
}

#[allow(dead_code)]
fn draw_sun_icon(canvas: &mut RgbaImage, center: (i32, i32), color: Rgba<u8>) {
    circle_ring(canvas, center, 8, 5, color);
    for index in 0..8 {
        let angle = index as f32 * PI / 4.0;
        let start = (
            center.0 as f32 + angle.cos() * 11.0,
            center.1 as f32 + angle.sin() * 11.0,
        );
        let end = (
            center.0 as f32 + angle.cos() * 15.0,
            center.1 as f32 + angle.sin() * 15.0,
        );
        icon_line(canvas, start, end, color);
    }
}

fn draw_star_icon(canvas: &mut RgbaImage, center: (i32, i32), color: Rgba<u8>) {
    let points = (0..10)
        .map(|index| {
            let angle = -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::PI / 5.0;
            let radius = if index % 2 == 0 { 13.0 } else { 6.0 };
            Point::new(
                (center.0 as f32 + angle.cos() * radius).round() as i32,
                (center.1 as f32 + angle.sin() * radius).round() as i32,
            )
        })
        .collect::<Vec<_>>();
    draw_polygon_mut(canvas, &points, color);
}

fn draw_milestone(canvas: &mut RgbaImage, center: (i32, i32), active: bool, colors: Palette) {
    let outer = if active { colors.white } else { colors.purple };
    draw_filled_circle_mut(canvas, center, 27, outer);
    draw_filled_circle_mut(canvas, center, 24, colors.milestone);
    let icon_color = if active { colors.white } else { colors.icon };
    draw_star_icon(canvas, center, icon_color);
}

fn draw_avatar_circle(
    canvas: &mut RgbaImage,
    avatar: &RgbaImage,
    position: (i32, i32),
    size: u32,
    colors: Palette,
) {
    let radius = size as i32 / 2;
    let radius_squared = radius * radius;
    for y in 0..size {
        for x in 0..size {
            let dx = x as i32 - radius;
            let dy = y as i32 - radius;
            if dx * dx + dy * dy <= radius_squared {
                let target_x = position.0 + x as i32;
                let target_y = position.1 + y as i32;
                if target_x >= 0
                    && target_y >= 0
                    && target_x < canvas.width() as i32
                    && target_y < canvas.height() as i32
                {
                    blend_pixel(
                        canvas.get_pixel_mut(target_x as u32, target_y as u32),
                        *avatar.get_pixel(x, y),
                    );
                }
            }
        }
    }
    let center = (position.0 + radius, position.1 + radius);
    circle_ring(
        canvas,
        center,
        radius + 3,
        radius,
        Rgba([colors.purple[0], colors.purple[1], colors.purple[2], 170]),
    );
}

fn progress_values(level: i64, xp: i64) -> (i64, i64, f32) {
    let level = level.max(0);
    let current_floor = level.saturating_mul(level).saturating_mul(100);
    let next_level = level.saturating_add(1);
    let next_floor = next_level.saturating_mul(next_level).saturating_mul(100);
    let required = (next_floor - current_floor).max(1);
    let current = (xp - current_floor).max(0);
    let progress = (current as f32 / required as f32).clamp(0.0, 1.0);
    (current, required, progress)
}

pub async fn render_ranking_card(
    client: &Client,
    input: RankingCardInput,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let colors = palette(&input.background);
    let mut canvas = ImageBuffer::from_pixel(WIDTH, HEIGHT, colors.background);
    draw_gradient(&mut canvas, colors);
    draw_background_grid(&mut canvas, colors.grid);
    blend_circle(
        &mut canvas,
        (WIDTH as i32 - 40, 30),
        210,
        Rgba([colors.purple[0], colors.purple[1], colors.purple[2], 46]),
    );
    blend_circle(
        &mut canvas,
        (80, HEIGHT as i32 + 30),
        180,
        Rgba([colors.purple[0], colors.purple[1], colors.purple[2], 31]),
    );
    draw_profile_surface(&mut canvas, colors);

    draw_text_fit(
        &mut canvas,
        "RANKING",
        54,
        44,
        30.0,
        colors.white,
        250,
        true,
    );
    draw_text_right_fit(
        &mut canvas,
        &format!("TOP 5 - {}", input.period),
        945,
        52,
        18.0,
        colors.muted,
        300,
        true,
    );
    rounded_rect(
        &mut canvas,
        54,
        82,
        892,
        1,
        1,
        Rgba([colors.purple[0], colors.purple[1], colors.purple[2], 105]),
    );

    let visible_entries = input.entries.into_iter().take(5).collect::<Vec<_>>();
    if visible_entries.is_empty() {
        draw_star_icon(&mut canvas, (500, 215), colors.purple_light);
        draw_text_fit(
            &mut canvas,
            "Ainda nao ha dados neste ranking.",
            330,
            250,
            22.0,
            colors.muted,
            340,
            false,
        );
    } else {
        let avatars = join_all(
            visible_entries
                .iter()
                .map(|entry| load_avatar(client, &entry.avatar_url, 38, colors)),
        )
        .await;

        for (index, (entry, avatar)) in visible_entries.iter().zip(avatars.iter()).enumerate() {
            let position = index + 1;
            let y = 98 + index as i32 * 51;
            let row_color = Rgba([colors.track[0], colors.track[1], colors.track[2], 145]);
            rounded_rect(&mut canvas, 54, y, 892, 44, 12, row_color);

            draw_text_fit(
                &mut canvas,
                &format!("#{position}"),
                68,
                y + 12,
                18.0,
                colors.white,
                48,
                true,
            );
            draw_avatar_circle(&mut canvas, avatar, (125, y + 3), 38, colors);
            draw_text_fit(
                &mut canvas,
                &entry.username,
                183,
                y + 10,
                20.0,
                colors.white,
                500,
                true,
            );
            draw_text_right_fit(
                &mut canvas,
                &format!("{} XP", entry.xp.max(0)),
                925,
                y + 11,
                19.0,
                colors.white,
                180,
                true,
            );
        }
    }

    let mut output = Cursor::new(Vec::new());
    canvas.write_to(&mut output, ImageFormat::Png)?;
    Ok(output.into_inner())
}

pub async fn render_profile_card(
    client: &Client,
    input: ProfileCardInput,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let colors = palette(&input.background);
    let mut canvas = ImageBuffer::from_pixel(WIDTH, HEIGHT, colors.background);
    draw_gradient(&mut canvas, colors);
    draw_background_grid(&mut canvas, colors.grid);
    blend_circle(
        &mut canvas,
        (WIDTH as i32 - 40, 30),
        210,
        Rgba([colors.purple[0], colors.purple[1], colors.purple[2], 46]),
    );
    blend_circle(
        &mut canvas,
        (80, HEIGHT as i32 + 30),
        180,
        Rgba([colors.purple[0], colors.purple[1], colors.purple[2], 31]),
    );

    draw_profile_surface(&mut canvas, colors);

    let avatar_size = 180;
    let avatar = load_avatar(client, &input.avatar_url, avatar_size, colors).await;
    let avatar_x = 58;
    let avatar_y = 68;
    let center = (
        avatar_x + avatar_size as i32 / 2,
        avatar_y + avatar_size as i32 / 2,
    );
    let radius = avatar_size as i32 / 2;
    let radius_squared = radius * radius;
    for y in 0..avatar_size {
        for x in 0..avatar_size {
            let dx = x as i32 - radius;
            let dy = y as i32 - radius;
            if dx * dx + dy * dy <= radius_squared {
                blend_pixel(
                    canvas.get_pixel_mut((avatar_x as u32) + x, (avatar_y as u32) + y),
                    *avatar.get_pixel(x, y),
                );
            }
        }
    }
    circle_ring(
        &mut canvas,
        center,
        radius + 6,
        radius + 1,
        Rgba([colors.purple[0], colors.purple[1], colors.purple[2], 55]),
    );
    circle_ring(&mut canvas, center, radius, radius - 6, colors.purple_light);

    draw_text_fit(
        &mut canvas,
        &input.username,
        285,
        86,
        38.0,
        colors.white,
        540,
        true,
    );
    draw_text_right_fit(
        &mut canvas,
        &format!("#{}", input.rank),
        925,
        49,
        25.0,
        colors.muted,
        70,
        true,
    );

    let (current_xp, required_xp, progress) = progress_values(input.level, input.total_xp);
    rounded_rect(&mut canvas, 285, 175, 600, 24, 12, colors.track);
    rounded_rect(
        &mut canvas,
        285,
        175,
        ((600.0 * progress) as u32).max(24).min(600),
        24,
        12,
        colors.purple,
    );
    draw_text_fit(
        &mut canvas,
        &format!("NÍVEL {}", input.level),
        285,
        211,
        20.0,
        colors.white,
        180,
        false,
    );
    draw_text_right_fit(
        &mut canvas,
        &format!("{current_xp}/{required_xp} XP"),
        885,
        211,
        20.0,
        colors.white,
        220,
        false,
    );

    let milestone_centers = [86, 293, 500, 707, 914];
    rounded_rect(&mut canvas, 86, 330, 828, 10, 5, colors.track);
    for (index, x) in milestone_centers.into_iter().enumerate() {
        draw_milestone(&mut canvas, (x, 335), index == 0, colors);
    }

    let mut output = Cursor::new(Vec::new());
    canvas.write_to(&mut output, ImageFormat::Png)?;
    Ok(output.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn renders_a_png_card_with_avatar_fallback() {
        let client = Client::new();
        let bytes = render_profile_card(
            &client,
            ProfileCardInput {
                username: "Gard".into(),
                avatar_url: "invalid://avatar".into(),
                level: 3,
                total_xp: 950,
                rank: 1,
                background: "midnight".into(),
            },
        )
        .await
        .unwrap();

        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!(image.width(), WIDTH);
        assert_eq!(image.height(), HEIGHT);
    }

    #[tokio::test]
    async fn renders_a_png_ranking_with_avatar_fallback() {
        let client = Client::new();
        let bytes = render_ranking_card(
            &client,
            RankingCardInput {
                period: "Total".into(),
                entries: vec![RankingEntry {
                    username: "Gard".into(),
                    avatar_url: "invalid://avatar".into(),
                    xp: 950,
                }],
                background: "midnight".into(),
            },
        )
        .await
        .unwrap();

        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!(image.width(), WIDTH);
        assert_eq!(image.height(), HEIGHT);
    }
}
