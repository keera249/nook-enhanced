//! Low-level painting: squircle fills with vertical gradients, the glass
//! surface (fill + grain + rim + top highlight), glows, icons and rings.

use crate::geometry::Squircle;
use crate::theme::{FAMILY_ICON_FILL, FAMILY_ICON_REGULAR, TILE_RADIUS, Theme, mix, with_alpha};
use egui::epaint::{Mesh, PathShape, PathStroke, Vertex};
use egui::{
    Align2, Color32, ColorImage, Context, FontFamily, FontId, Painter, Pos2, Rect, Shape, Stroke,
    TextureHandle, TextureId, TextureOptions, TextureWrapMode, pos2, vec2,
};

const NOISE_SIZE: usize = 128;

/// Vertical-gradient fill of a convex outline (fan triangulation from the
/// centroid; colors are linear in y, so the interpolation is exact).
pub fn fill_gradient(painter: &Painter, outline: &[Pos2], top: Color32, bottom: Color32) {
    if outline.len() < 3 {
        return;
    }
    let (min_y, max_y) = outline.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
        (lo.min(p.y), hi.max(p.y))
    });
    let span = (max_y - min_y).max(1.0);
    let color_at = |y: f32| lerp_color(top, bottom, (y - min_y) / span);
    let mut mesh = Mesh::default();
    let center = centroid(outline);
    mesh.colored_vertex(center, color_at(center.y));
    for p in outline {
        mesh.colored_vertex(*p, color_at(p.y));
    }
    let n = outline.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(mesh));
}

/// Film-grain overlay clipped to the outline, using a tiling noise texture.
///
/// The texture is mapped in physical pixels (one texel covers `grain_px`
/// screen pixels), so the grain stays fine at any display scaling instead
/// of growing into blotches at 150-200%.
pub fn fill_noise(
    painter: &Painter,
    outline: &[Pos2],
    texture: TextureId,
    alpha: u8,
    grain_px: f32,
) {
    if outline.len() < 3 || alpha == 0 {
        return;
    }
    let tint = Color32::from_white_alpha(alpha);
    let scale = painter.ctx().pixels_per_point() / (NOISE_SIZE as f32 * grain_px.max(1.0));
    let uv = |p: Pos2| pos2(p.x * scale, p.y * scale);
    let mut mesh = Mesh::with_texture(texture);
    let center = centroid(outline);
    mesh.vertices.push(Vertex {
        pos: center,
        uv: uv(center),
        color: tint,
    });
    for p in outline {
        mesh.vertices.push(Vertex {
            pos: *p,
            uv: uv(*p),
            color: tint,
        });
    }
    let n = outline.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(mesh));
}

/// The shell's glass surface at shell `progress` (0 = pill, 1 = panel).
/// With `fill = false` the compositor draws the tint, rim and highlight (see
/// `nook_platform::backdrop`) and only the grain is painted here.
pub fn glass(
    painter: &Painter,
    shape: &Squircle,
    outline: &[Pos2],
    theme: &Theme,
    opacity: f32,
    progress: f32,
    fill: bool,
) {
    let o = |c: Color32| with_alpha(c, opacity);
    if fill {
        let (top, bottom) = theme.glass_at(progress);
        fill_gradient(painter, outline, o(top), o(bottom));
    }
    let noise = noise_texture(painter.ctx());
    fill_noise(
        painter,
        outline,
        noise.id(),
        (theme.noise_alpha as f32 * opacity) as u8,
        theme.grain_px,
    );
    if !fill {
        return;
    }
    painter.add(PathShape::closed_line(
        outline.to_vec(),
        PathStroke::new(1.0, o(theme.rim_at(progress))),
    ));
    top_highlight(painter, shape, o(theme.highlight_at(progress)));
}

/// 1 px highlight along the straight part of the top edge, fading out
/// before the corners begin (so it never pokes outside the curve).
pub fn top_highlight(painter: &Painter, shape: &Squircle, color: Color32) {
    let r = shape.radius;
    let y = shape.rect.min.y + 0.75;
    let x0 = shape.rect.min.x + r;
    let x1 = shape.rect.max.x - r;
    if x1 <= x0 {
        return;
    }
    let mid = 0.5 * (x0 + x1);
    let fade = Color32::TRANSPARENT;
    let mut mesh = Mesh::default();
    let h = 1.0;
    for (x, c) in [(x0, fade), (mid, color), (x1, fade)] {
        mesh.colored_vertex(pos2(x, y), c);
        mesh.colored_vertex(pos2(x, y + h), c);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    mesh.add_triangle(2, 3, 4);
    mesh.add_triangle(3, 4, 5);
    painter.add(Shape::mesh(mesh));
}

/// Exact horizontal `[x_left, x_right]` span of `shape` at vertical coordinate `y`.
/// Evaluating against the full `Squircle` avoids creating a truncated squircle with
/// unwanted bottom-corner rounding in the middle of a tile or panel.
fn squircle_x_span(shape: &Squircle, y: f32) -> (f32, f32) {
    let rect = shape.rect;
    let r = shape.radius;
    let e = shape.exponent.max(2.0);
    let inset = if !shape.flat_top && r > 0.0 && y < rect.min.y + r {
        let qy = ((rect.min.y + r - y) / r).clamp(0.0, 1.0);
        let qx = (1.0 - qy.powf(e)).max(0.0).powf(1.0 / e);
        r * (1.0 - qx)
    } else if r > 0.0 && y > rect.max.y - r {
        let qy = ((y - (rect.max.y - r)) / r).clamp(0.0, 1.0);
        let qx = (1.0 - qy.powf(e)).max(0.0).powf(1.0 / e);
        r * (1.0 - qx)
    } else {
        0.0
    };
    (rect.min.x + inset, rect.max.x - inset)
}

/// Paints a 2D convex optical lens sheen over the upper `reach_frac` (0..1) of `shape`,
/// staying 100% flush with `shape`'s true top corners and vertical walls.
pub fn fill_convex_sheen(
    painter: &Painter,
    shape: &Squircle,
    reach_frac: f32,
    top_center: Color32,
    top_edge: Color32,
) {
    let rect = shape.rect;
    let span_y = (rect.height() * reach_frac.clamp(0.05, 1.0)).max(2.0);
    if rect.width() <= 2.0 || span_y <= 2.0 {
        return;
    }
    const SLICES: usize = 16;
    let mut mesh = Mesh::default();
    let cx = rect.center().x;

    for i in 0..=SLICES {
        // Cosine spacing clusters slices near the top corner curve for sub-pixel accuracy.
        let u = i as f32 / SLICES as f32;
        let t = 1.0 - (u * std::f32::consts::FRAC_PI_2).cos();
        let y = rect.min.y + span_y * t;
        let (xl, xr) = squircle_x_span(shape, y);
        // Smooth quadratic optical attenuation from top (1.0) to reach_frac (0.0).
        let falloff = (1.0 - t) * (1.0 - t);
        let c_mid = with_alpha(top_center, falloff);
        let c_side = with_alpha(top_edge, falloff);
        mesh.colored_vertex(pos2(xl, y), c_side);
        mesh.colored_vertex(pos2(cx, y), c_mid);
        mesh.colored_vertex(pos2(xr, y), c_side);
    }

    for i in 0..(SLICES as u32) {
        let row0 = i * 3;
        let row1 = (i + 1) * 3;
        // Left quad (xl..cx)
        mesh.add_triangle(row0, row0 + 1, row1);
        mesh.add_triangle(row0 + 1, row1 + 1, row1);
        // Right quad (cx..xr)
        mesh.add_triangle(row0 + 1, row0 + 2, row1 + 1);
        mesh.add_triangle(row0 + 2, row1 + 2, row1 + 1);
    }
    painter.add(Shape::mesh(mesh));
}

/// Paints a directional 2D Fresnel specular & caustic rim around `outline` using
/// egui's hardware-feathered `PathStroke::new_uv`.
pub fn fresnel_rim_stroke(
    painter: &Painter,
    outline: Vec<Pos2>,
    top_col: Color32,
    side_col: Color32,
    bot_col: Color32,
    width: f32,
) {
    if outline.len() < 3 {
        return;
    }
    let stroke = PathStroke::new_uv(width, move |bbox, p| {
        let ny = ((p.y - bbox.min.y) / bbox.height().max(1.0)).clamp(0.0, 1.0);
        let nx = ((p.x - bbox.min.x) / bbox.width().max(1.0)).clamp(0.0, 1.0);
        if ny < 0.52 {
            let k = (1.0 - ny / 0.52).clamp(0.0, 1.0);
            // Slight top-left directional specular bias
            let spec_boost = (1.0 - nx * 0.22).clamp(0.78, 1.0);
            lerp_color(side_col, top_col, (k * k * spec_boost).clamp(0.0, 1.0))
        } else {
            let k = ((ny - 0.52) / 0.48).clamp(0.0, 1.0);
            // Slight bottom-right caustic refraction gathering
            let caustic_boost = (0.82 + nx * 0.18).clamp(0.82, 1.0);
            lerp_color(side_col, bot_col, (k * k * caustic_boost).clamp(0.0, 1.0))
        }
    });
    painter.add(PathShape::closed_line(outline, stroke));
}

/// Interior optical lens sheen for the shell.
/// When `composited` is true, `nook_platform::backdrop::Backdrop` already renders the
/// hardware-synchronized 4-stop Liquid Glass gradient, 3-stop Fresnel rim, and highlight
/// inside the DWM `GeometricClip`, so we only add the settled interior dome well inside
/// the clip to guarantee zero swapchain edge ghosting during spring animations.
pub fn liquid_sheen(
    painter: &Painter,
    shape: &Squircle,
    theme: &Theme,
    opacity: f32,
    progress: f32,
    composited: bool,
) {
    if opacity <= 0.01 {
        return;
    }

    if !composited {
        let [top_r, side_r, bot_r] = theme.rim_stops_at(progress, shape.flat_top);
        fresnel_rim_stroke(
            painter,
            shape.outline(),
            with_alpha(top_r, opacity),
            with_alpha(side_r, opacity),
            with_alpha(bot_r, opacity),
            1.2,
        );
    }

    // Settled expanded panel interior optical dome (inset 2.0px so it never touches the outer edge)
    let settled = ((progress - 0.72) / 0.26).clamp(0.0, 1.0) * opacity;
    if settled > 0.01 {
        let inner_rect = shape.rect.shrink(2.0);
        if inner_rect.width() > 8.0 && inner_rect.height() > 8.0 {
            let inner_shape = Squircle::new(inner_rect, (shape.radius - 2.0).max(4.0))
                .with_exponent(shape.exponent)
                .with_flat_top(shape.flat_top);
            let center_a = if theme.dark {
                (26.0 * settled) as u8
            } else {
                (64.0 * settled) as u8
            };
            let edge_a = if theme.dark {
                (10.0 * settled) as u8
            } else {
                (28.0 * settled) as u8
            };
            let top_center = if shape.flat_top {
                Color32::from_rgba_unmultiplied(255, 255, 255, center_a / 2)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, center_a)
            };
            let top_edge = Color32::from_rgba_unmultiplied(255, 255, 255, edge_a);
            fill_convex_sheen(painter, &inner_shape, 0.36, top_center, top_edge);
        }
    }
}

/// Curved liquid glass specular sheen, 2D Fresnel rim, and inner refractive thickness
/// bevel for bento tiles and page cards.
pub fn tile_liquid_sheen(painter: &Painter, rect: Rect, theme: &Theme, active: bool, lift: f32) {
    let r = TILE_RADIUS;
    let shape = Squircle::new(rect, r);

    // 1. 2D Convex Specular Lens Dome over the upper 44% of the true tile Squircle
    // (flush with the straight vertical walls; zero mid-tile rounded-bottom-corner artifact).
    let boost = 1.0 + lift * 0.55;
    let (center_a, edge_a) = if theme.dark {
        (
            ((if active { 44.0 } else { 32.0 }) * boost).min(78.0) as u8,
            ((if active { 20.0 } else { 14.0 }) * boost).min(42.0) as u8,
        )
    } else {
        (
            ((if active { 96.0 } else { 78.0 }) * boost).min(155.0) as u8,
            ((if active { 48.0 } else { 36.0 }) * boost).min(95.0) as u8,
        )
    };
    let dome_tint = if active {
        mix(Color32::WHITE, theme.accent, 0.18)
    } else {
        Color32::WHITE
    };
    let [dr, dg, db, _] = dome_tint.to_srgba_unmultiplied();
    fill_convex_sheen(
        painter,
        &shape,
        0.44,
        Color32::from_rgba_unmultiplied(dr, dg, db, center_a),
        Color32::from_rgba_unmultiplied(dr, dg, db, edge_a),
    );

    // 2. Outer 2D Fresnel Specular & Caustic Rim (1.15px)
    let caustic_rgb = mix(Color32::WHITE, theme.accent, if active { 0.48 } else { 0.30 });
    let [cr, cg, cb, _] = caustic_rgb.to_srgba_unmultiplied();
    let (top_rim_a, side_rim_a, bot_rim_a) = if theme.dark {
        (
            (88.0 + lift * 52.0 + if active { 24.0 } else { 0.0 }).min(185.0) as u8,
            (22.0 + lift * 16.0 + if active { 14.0 } else { 0.0 }).min(70.0) as u8,
            (46.0 + lift * 28.0 + if active { 26.0 } else { 0.0 }).min(115.0) as u8,
        )
    } else {
        (
            (185.0 + lift * 45.0).min(245.0) as u8,
            (45.0 + lift * 25.0).min(95.0) as u8,
            (85.0 + lift * 35.0).min(150.0) as u8,
        )
    };
    fresnel_rim_stroke(
        painter,
        shape.outline(),
        Color32::from_rgba_unmultiplied(255, 255, 255, top_rim_a),
        Color32::from_rgba_unmultiplied(dr, dg, db, side_rim_a),
        Color32::from_rgba_unmultiplied(cr, cg, cb, bot_rim_a),
        1.15,
    );

    // 3. Inner Optical Refractive Thickness Bevel (1.0px inset)
    let inner = rect.shrink(1.0);
    if inner.width() > 4.0 && inner.height() > 4.0 {
        let inner_shape = Squircle::new(inner, (r - 1.0).max(2.0));
        let (in_top_a, in_bot_a) = if theme.dark {
            (
                (22.0 + lift * 20.0).min(55.0) as u8,
                (16.0 + lift * 14.0).min(42.0) as u8,
            )
        } else {
            (
                (65.0 + lift * 30.0).min(120.0) as u8,
                (28.0 + lift * 18.0).min(65.0) as u8,
            )
        };
        fresnel_rim_stroke(
            painter,
            inner_shape.outline(),
            Color32::from_rgba_unmultiplied(cr, cg, cb, in_top_a),
            Color32::TRANSPARENT,
            Color32::from_rgba_unmultiplied(255, 255, 255, in_bot_a),
            0.85,
        );
    }
}

/// Soft glow: a few expanding, fading squircles under a shape.
pub fn glow(painter: &Painter, shape: &Squircle, color: Color32, strength: f32) {
    if strength <= 0.01 {
        return;
    }
    for (grow, a) in [(6.0, 0.10), (3.5, 0.16), (1.5, 0.24)] {
        let s = Squircle::new(shape.rect.expand(grow), shape.radius + grow)
            .with_exponent(shape.exponent);
        painter.add(Shape::convex_polygon(
            s.outline(),
            with_alpha(color, a * strength),
            Stroke::NONE,
        ));
    }
}

/// A Phosphor glyph. `fill` selects the filled variant (active state).
pub fn icon(painter: &Painter, center: Pos2, glyph: &str, size: f32, color: Color32, fill: bool) {
    let family = if fill {
        FAMILY_ICON_FILL
    } else {
        FAMILY_ICON_REGULAR
    };
    painter.text(
        center,
        Align2::CENTER_CENTER,
        glyph,
        FontId::new(size, FontFamily::Name(family.into())),
        color,
    );
}

/// Progress ring; `progress` in 0..1 runs clockwise from 12 o'clock.
pub fn ring(
    painter: &Painter,
    center: Pos2,
    radius: f32,
    width: f32,
    progress: f32,
    track: Color32,
    color: Color32,
) {
    painter.circle_stroke(center, radius, Stroke::new(width, track));
    let progress = progress.clamp(0.0, 1.0);
    if progress <= 0.0 {
        return;
    }
    let segments = ((64.0 * progress).ceil() as usize).max(2);
    let start = -std::f32::consts::FRAC_PI_2;
    let points: Vec<Pos2> = (0..=segments)
        .map(|i| {
            let a = start + std::f32::consts::TAU * progress * i as f32 / segments as f32;
            center + vec2(a.cos(), a.sin()) * radius
        })
        .collect();
    painter.add(PathShape::line(points, PathStroke::new(width, color)));
}

fn centroid(points: &[Pos2]) -> Pos2 {
    let sum = points
        .iter()
        .fold(vec2(0.0, 0.0), |acc, p| acc + p.to_vec2());
    (sum / points.len() as f32).to_pos2()
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    // Premultiplied components interpolate correctly for alpha blending.
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

/// Lazily created tiling grain texture (deterministic, generated once).
///
/// Each texel is a light or a dark speck (premultiplied white or black)
/// whose alpha follows a triangular distribution around zero, so the grain
/// adds texture without washing the glass lighter.
pub fn noise_texture(ctx: &Context) -> TextureHandle {
    let key = egui::Id::NULL.with("nook-noise");
    if let Some(tex) = ctx.data(|d| d.get_temp::<TextureHandle>(key)) {
        return tex;
    }
    let mut seed: u32 = 0x9E37_79B9;
    let mut next = || {
        // xorshift32
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed >> 24) as i32
    };
    let mut pixels = Vec::with_capacity(NOISE_SIZE * NOISE_SIZE);
    for _ in 0..NOISE_SIZE * NOISE_SIZE {
        // Difference of two uniforms: -255..=255, mostly near zero.
        let v = next() - next();
        let a = v.unsigned_abs().min(255) as u8;
        pixels.push(if v >= 0 {
            Color32::from_rgba_premultiplied(a, a, a, a)
        } else {
            Color32::from_rgba_premultiplied(0, 0, 0, a)
        });
    }
    let image = ColorImage::new([NOISE_SIZE, NOISE_SIZE], pixels);
    let tex = ctx.load_texture(
        "nook-noise",
        image,
        TextureOptions {
            wrap_mode: TextureWrapMode::Repeat,
            ..TextureOptions::NEAREST
        },
    );
    ctx.data_mut(|d| d.insert_temp(key, tex.clone()));
    tex
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Look, Theme};
    use egui::pos2;

    #[test]
    fn squircle_x_span_is_flush_with_vertical_walls_at_mid_height() {
        let rect = Rect::from_min_max(pos2(10.0, 20.0), pos2(110.0, 120.0));
        let shape = Squircle::new(rect, 14.0);
        // At 44% height (y = 64.0), a mid-tile sheen slice must span the entire [10.0, 110.0] width
        // with zero inward corner rounding.
        let (x_l, x_r) = squircle_x_span(&shape, 64.0);
        assert!((x_l - 10.0).abs() < 1e-4);
        assert!((x_r - 110.0).abs() < 1e-4);

        // Near the top corner (y = 22.0, inside top radius 14.0), the span narrows smoothly.
        let (top_l, top_r) = squircle_x_span(&shape, 22.0);
        assert!(top_l > 10.0 && top_r < 110.0);
        assert!(((top_l - 10.0) - (110.0 - top_r)).abs() < 1e-4);
    }

    #[test]
    fn squircle_x_span_respects_flat_top_notch() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(200.0, 36.0));
        let shape = Squircle::new(rect, 18.0).with_flat_top(true);
        let (x_l, x_r) = squircle_x_span(&shape, 1.0);
        assert!((x_l - 0.0).abs() < 1e-4);
        assert!((x_r - 200.0).abs() < 1e-4);
    }

    #[test]
    fn liquid_glass_optical_stops_have_clear_mid_lens_and_fresnel_rim() {
        let look = Look {
            liquid_glass: true,
            ..Look::default()
        };
        let theme = Theme::styled(true, Color32::from_rgb(56, 189, 248), true, &look);
        assert!(theme.liquid_glass);
        let [top, upper_mid, lower_mid, bottom] = theme.glass_stops_at(1.0, false);
        // In Liquid Glass mode, the upper-mid lens window is clearer (lower alpha) than the bottom caustic/grounding.
        assert!(upper_mid.a() < bottom.a());
        assert!(top.r() > lower_mid.r());

        let [rim_top, rim_mid, rim_bot] = theme.rim_stops_at(1.0, false);
        // Top and bottom Fresnel curves have stronger specular reflection than the vertical side walls.
        assert!(rim_top.a() > rim_mid.a());
        assert!(rim_bot.a() > rim_mid.a());

        // In hardware Notch mode (flat_top = true) when collapsed (progress = 0.0),
        // the top bezel stop is deep dark ink so it blends seamlessly with the screen bezel.
        let [pill_top, _, _, _] = theme.glass_stops_at(0.0, true);
        assert!(pill_top.r() <= 20 && pill_top.a() >= 215);
    }
}
