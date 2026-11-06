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

/// Specular reflections, refractive inner caustic bevel, and fluid surface lens curve
/// for both the expanded panel and the collapsed Dynamic Island pill.
pub fn liquid_sheen(
    painter: &Painter,
    shape: &Squircle,
    theme: &Theme,
    opacity: f32,
    progress: f32,
    time: f64,
) {
    if opacity <= 0.01 {
        return;
    }
    let rect = shape.rect;
    let r = shape.radius;
    let pill_stage = (1.0 - progress / 0.35).clamp(0.0, 1.0);

    // 1. In the collapsed Dynamic Island pill:
    // Polished liquid obsidian meniscus, caustic refraction rim, and prismatic specular reflex.
    if pill_stage > 0.01 {
        let p_alpha = pill_stage * opacity;
        let h = (rect.height() * 0.52).max(4.0);
        let highlight_top = Color32::from_rgba_unmultiplied(255, 255, 255, (95.0 * p_alpha) as u8);
        let highlight_bot = Color32::from_rgba_unmultiplied(255, 255, 255, 0);

        // Meniscus convex specular dome
        let sheen_rect = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + h));
        let meniscus = Squircle::new(sheen_rect, (r - 1.0).max(2.0)).with_flat_top(shape.flat_top);
        fill_gradient(painter, &meniscus.outline(), highlight_top, highlight_bot);

        // Micro caustic refraction rim (inset 1.0px) around the whole pill
        let inner_rect = rect.shrink(1.0);
        if inner_rect.width() > 0.0 && inner_rect.height() > 0.0 {
            let inner_pill = Squircle::new(inner_rect, (r - 1.0).max(2.0)).with_flat_top(shape.flat_top);
            let caustic_c = Color32::from_rgba_unmultiplied(255, 255, 255, (45.0 * p_alpha) as u8);
            painter.add(PathShape::closed_line(
                inner_pill.outline(),
                PathStroke::new(0.9, caustic_c),
            ));
        }

        // Sharp specular reflex line along the top curve
        let x0 = rect.min.x + r * 0.55;
        let x1 = rect.max.x - r * 0.55;
        if x1 > x0 + 10.0 {
            let mut mesh = Mesh::default();
            let y = rect.min.y + 1.2;
            let mid_x = 0.5 * (x0 + x1);
            let c_center = Color32::from_rgba_unmultiplied(255, 255, 255, (190.0 * p_alpha) as u8);
            let c_edge = Color32::TRANSPARENT;
            for (x, c) in [(x0, c_edge), (mid_x, c_center), (x1, c_edge)] {
                mesh.colored_vertex(pos2(x, y), c);
                mesh.colored_vertex(pos2(x, y + 1.0), c);
            }
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(1, 2, 3);
            mesh.add_triangle(2, 3, 4);
            mesh.add_triangle(3, 4, 5);
            painter.add(Shape::mesh(mesh));

            // Chromatic / accent dispersion fringe directly underneath specular line
            let mut prism_mesh = Mesh::default();
            let py = y + 1.0;
            let prism_base = mix(Color32::from_rgb(180, 220, 255), theme.accent, 0.22);
            let [pr, pg, pb, _] = prism_base.to_srgba_unmultiplied();
            let prism_c = Color32::from_rgba_unmultiplied(pr, pg, pb, (50.0 * p_alpha) as u8);
            for (x, c) in [(x0, c_edge), (mid_x, prism_c), (x1, c_edge)] {
                prism_mesh.colored_vertex(pos2(x, py), c);
                prism_mesh.colored_vertex(pos2(x, py + 0.8), c);
            }
            prism_mesh.add_triangle(0, 1, 2);
            prism_mesh.add_triangle(1, 2, 3);
            prism_mesh.add_triangle(2, 3, 4);
            prism_mesh.add_triangle(3, 4, 5);
            painter.add(Shape::mesh(prism_mesh));
        }

        // Dynamic fluid shimmer glint across the pill
        let pill_shimmer_t = (time * 0.7).sin() as f32 * 0.5 + 0.5;
        if x1 > x0 + 30.0 {
            let span = x1 - x0;
            let cx = x0 + span * pill_shimmer_t;
            let w = (span * 0.25).max(20.0);
            let s_min = (cx - w * 0.5).max(x0);
            let s_max = (cx + w * 0.5).min(x1);
            if s_max > s_min + 4.0 && cx > s_min && cx < s_max {
                let glint_c = Color32::from_rgba_unmultiplied(255, 255, 255, (60.0 * p_alpha) as u8);
                let y = rect.min.y + 0.8;
                let mut glint_mesh = Mesh::default();
                for (x, c) in [(s_min, Color32::TRANSPARENT), (cx, glint_c), (s_max, Color32::TRANSPARENT)] {
                    glint_mesh.colored_vertex(pos2(x, y), c);
                    glint_mesh.colored_vertex(pos2(x, y + 1.2), c);
                }
                glint_mesh.add_triangle(0, 1, 2);
                glint_mesh.add_triangle(1, 2, 3);
                glint_mesh.add_triangle(2, 3, 4);
                glint_mesh.add_triangle(3, 4, 5);
                painter.add(Shape::mesh(glint_mesh));
            }
        }

        // Subtle bottom bounce reflection (liquid light gathering)
        let bounce_rect = Rect::from_min_max(
            pos2(rect.min.x + r, rect.max.y - 2.8),
            pos2(rect.max.x - r, rect.max.y - 1.0),
        );
        if bounce_rect.width() > 0.0 && bounce_rect.height() > 0.0 {
            painter.rect_filled(
                bounce_rect,
                1.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, (30.0 * p_alpha) as u8),
            );
        }
    }

    // 2. In the expanding/expanded panel state:
    // Fluid surface lens curve, caustic inner edge, and gentle specular wave.
    let panel_stage = (progress / 0.35).clamp(0.0, 1.0);
    if panel_stage > 0.01 {
        let p_alpha = panel_stage * opacity;

        // Subtle liquid sheen over the upper 38% of the panel
        let sheen_h = (rect.height() * 0.38).min(180.0);
        let top_c = if theme.dark {
            Color32::from_rgba_unmultiplied(255, 255, 255, (34.0 * p_alpha) as u8)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, (75.0 * p_alpha) as u8)
        };
        let bot_c = Color32::from_rgba_unmultiplied(255, 255, 255, 0);

        let sheen_rect = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + sheen_h));
        let sheen_shape = Squircle::new(sheen_rect, r).with_flat_top(shape.flat_top);
        fill_gradient(painter, &sheen_shape.outline(), top_c, bot_c);

        // Inner optical caustic refraction rim (inset 1.2px) with subtle accent integration
        let inner_rect = rect.shrink(1.2);
        if inner_rect.width() > 0.0 && inner_rect.height() > 0.0 {
            let inner_shape = Squircle::new(inner_rect, (r - 1.2).max(4.0))
                .with_exponent(shape.exponent)
                .with_flat_top(shape.flat_top);
            let caustic_base = mix(Color32::WHITE, theme.accent, 0.16);
            let [cr, cg, cb, _] = caustic_base.to_srgba_unmultiplied();
            let caustic_a = (if theme.dark { 28.0 } else { 60.0 } * p_alpha) as u8;
            let caustic_c = Color32::from_rgba_unmultiplied(cr, cg, cb, caustic_a);
            painter.add(PathShape::closed_line(
                inner_shape.outline(),
                PathStroke::new(1.0, caustic_c),
            ));
        }

        // Fluid shimmering light wave across the top highlight
        let shimmer_x_t = (time * 0.5).sin() as f32 * 0.5 + 0.5;
        let x0 = rect.min.x + r;
        let x1 = rect.max.x - r;
        if x1 > x0 + 40.0 {
            let span = x1 - x0;
            let center_x = x0 + span * shimmer_x_t;
            let width = (span * 0.32).max(35.0);
            let s_min = (center_x - width * 0.5).max(x0);
            let s_max = (center_x + width * 0.5).min(x1);
            if s_max > s_min + 4.0 && center_x > s_min && center_x < s_max {
                let wave_c = Color32::from_rgba_unmultiplied(255, 255, 255, (55.0 * p_alpha) as u8);
                let y = rect.min.y + 0.8;
                let mut mesh = Mesh::default();
                for (x, c) in [
                    (s_min, Color32::TRANSPARENT),
                    (center_x, wave_c),
                    (s_max, Color32::TRANSPARENT),
                ] {
                    mesh.colored_vertex(pos2(x, y), c);
                    mesh.colored_vertex(pos2(x, y + 1.2), c);
                }
                mesh.add_triangle(0, 1, 2);
                mesh.add_triangle(1, 2, 3);
                mesh.add_triangle(2, 3, 4);
                mesh.add_triangle(3, 4, 5);
                painter.add(Shape::mesh(mesh));
            }
        }

        // Bottom desk bounce light (optical depth)
        let bounce_rect = Rect::from_min_max(
            pos2(rect.min.x + r * 1.5, rect.max.y - 2.5),
            pos2(rect.max.x - r * 1.5, rect.max.y - 0.8),
        );
        if bounce_rect.width() > 0.0 && bounce_rect.height() > 0.0 {
            painter.rect_filled(
                bounce_rect,
                1.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, (22.0 * p_alpha) as u8),
            );
        }
    }
}

/// Curved liquid glass specular sheen and refractive optical bevel for bento tiles.
pub fn tile_liquid_sheen(painter: &Painter, rect: Rect, theme: &Theme, lift: f32) {
    let r = TILE_RADIUS;
    let h = (rect.height() * 0.46).min(48.0);
    let sheen_rect = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + h));
    let s = Squircle::new(sheen_rect, (r - 0.5).max(4.0));

    let alpha_boost = 1.0 + lift * 0.6;
    let top_a = if theme.dark {
        (42.0 * alpha_boost).min(90.0) as u8
    } else {
        (95.0 * alpha_boost).min(170.0) as u8
    };

    // Top convex specular meniscus
    fill_gradient(
        painter,
        &s.outline(),
        Color32::from_rgba_unmultiplied(255, 255, 255, top_a),
        Color32::from_rgba_unmultiplied(255, 255, 255, 0),
    );

    // Sharp specular hairline along the upper curve
    let x0 = rect.min.x + r * 0.7;
    let x1 = rect.max.x - r * 0.7;
    if x1 > x0 + 10.0 {
        let mut mesh = Mesh::default();
        let y = rect.min.y + 0.8;
        let mid_x = 0.5 * (x0 + x1);
        let crest_a = if theme.dark {
            (110.0 + lift * 80.0).min(220.0) as u8
        } else {
            (180.0 + lift * 60.0).min(255.0) as u8
        };
        let c_mid = Color32::from_rgba_unmultiplied(255, 255, 255, crest_a);
        let c_edge = Color32::TRANSPARENT;
        for (x, c) in [(x0, c_edge), (mid_x, c_mid), (x1, c_edge)] {
            mesh.colored_vertex(pos2(x, y), c);
            mesh.colored_vertex(pos2(x, y + 0.9), c);
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(1, 2, 3);
        mesh.add_triangle(2, 3, 4);
        mesh.add_triangle(3, 4, 5);
        painter.add(Shape::mesh(mesh));
    }

    // Inner optical refractive chamfer stroke (1.0px inset)
    let inner = rect.shrink(1.0);
    if inner.width() > 0.0 && inner.height() > 0.0 {
        let inner_shape = Squircle::new(inner, (r - 1.0).max(2.0));
        let bevel_base = mix(Color32::WHITE, theme.accent, lift * 0.22);
        let [br, bg, bb, _] = bevel_base.to_srgba_unmultiplied();
        let bevel_a = if theme.dark {
            (24.0 + lift * 32.0).min(80.0) as u8
        } else {
            (65.0 + lift * 35.0).min(140.0) as u8
        };
        let bevel_c = Color32::from_rgba_unmultiplied(br, bg, bb, bevel_a);
        painter.add(PathShape::closed_line(
            inner_shape.outline(),
            PathStroke::new(0.9, bevel_c),
        ));
    }

    // Subtle bottom bounce highlight on the card
    let bottom_hairline = Rect::from_min_max(
        pos2(rect.min.x + r * 1.2, rect.max.y - 1.8),
        pos2(rect.max.x - r * 1.2, rect.max.y - 0.8),
    );
    if bottom_hairline.width() > 0.0 && bottom_hairline.height() > 0.0 {
        let bounce_a = (14.0 + lift * 18.0) as u8;
        painter.rect_filled(
            bottom_hairline,
            0.5,
            Color32::from_rgba_unmultiplied(255, 255, 255, bounce_a),
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
