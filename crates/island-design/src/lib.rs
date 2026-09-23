//! Shared logical-point tokens and geometry; no toolkit types.
pub const COMPACT_WIDTH: u32 = 245;
pub const COMPACT_HEIGHT: u32 = 24;
pub const MAX_CONTENT_WIDTH: u32 = 512;
pub const MAX_CONTENT_HEIGHT: u32 = 256;
pub const COLLAPSED_WIDTH: f32 = 185.0;
pub const COLLAPSED_HEIGHT: f32 = 32.0;
pub const NOTCH_EAR: f32 = 44.0;
pub const CONTENT_INSET: f32 = 16.0;
pub const EXPAND_MS: u64 = 420;
pub const PEEK_MS: u64 = 4_000;
pub const ACTIVITY_EAR: f32 = 38.0;
pub const PILL_BG: u32 = 0x0a0a0a;
pub const TEXT_PRIMARY: u32 = 0xf5f5f7;
pub const TEXT_SECONDARY: u32 = 0x98989e;
/// Logical points per discrete mouse-wheel step across native shells.
pub const SCROLL_LINE_HEIGHT: f32 = 16.0;

/// Agent Pulse surface, shared by every shell through the plugin framebuffer.
pub mod agent_pulse {
    pub const WIDTH: u32 = 340;
    pub const HEIGHT: u32 = 88;
    pub const INSET: f32 = 14.0;
    pub const HEADER: f32 = 28.0;
    pub const ROW: f32 = 52.0;
    pub const BOTTOM: f32 = 8.0;
    pub const LIST_VIEWPORT_HEIGHT: f32 = 156.0;
    pub const SCROLLBAR_WIDTH: f32 = 3.0;
    pub const SCROLLBAR_HIT_WIDTH: f32 = 12.0;
    pub const SCROLLBAR_MIN_THUMB: f32 = 20.0;
    pub const SCROLLBAR: u32 = 0x48484c;
    pub const SETTINGS_HEIGHT: u32 = 248;
    pub const CONNECTION_ROW: f32 = 46.0;
    pub const CONNECTION_HINT_Y: f32 = 174.0;
    pub const CONNECTION_ACTION_Y: f32 = 207.0;
    pub const MENU_HEIGHT: u32 = 176;
    pub const ICON: f32 = 28.0;
    pub const TITLE: f32 = 14.0;
    pub const LABEL: f32 = 11.0;
    pub const SURFACE: u32 = 0x1b1b1d;
    pub const HOVER: u32 = 0x252527;
    pub const PRESSED: u32 = 0x303034;
    pub const WORKING: u32 = 0x80b4ff;
    pub const WAITING: u32 = 0xe8bf75;
    pub const SUCCESS: u32 = 0x92d6ac;
    pub const ERROR: u32 = 0xff9b91;
}

/// The same polygon is painted and used for native hit testing. Quadratic
/// attached corners and circular detached corners use a shared fixed sampling resolution.
pub fn outline(w: f32, h: f32, attached: bool, expand: f32) -> Vec<(f32, f32)> {
    if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
        return vec![];
    }
    let mut points = Vec::new();
    let mut curve = |a: (f32, f32), c: (f32, f32), b: (f32, f32)| {
        for i in 0..=16 {
            let t = i as f32 / 16.0;
            let u = 1.0 - t;
            points.push((
                u * u * a.0 + 2.0 * u * t * c.0 + t * t * b.0,
                u * u * a.1 + 2.0 * u * t * c.1 + t * t * b.1,
            ));
        }
    };
    if attached {
        let tr = (6.0 + 4.0 * expand).min(h * 0.45).min(w * 0.18);
        let br = (14.0 + 6.0 * expand)
            .min(h * 0.45)
            .min((w - 2.0 * tr) * 0.5);
        curve((0.0, 0.0), (tr, 0.0), (tr, tr));
        curve((tr, h - br), (tr, h), (tr + br, h));
        curve((w - tr - br, h), (w - tr, h), (w - tr, h - br));
        curve((w - tr, tr), (w - tr, 0.0), (w, 0.0));
    } else {
        let r = (h * 0.5).min(w * 0.5);
        for (cx, cy, start) in [
            (r, r, std::f32::consts::PI),
            (w - r, r, 1.5 * std::f32::consts::PI),
            (w - r, h - r, 0.0),
            (r, h - r, std::f32::consts::FRAC_PI_2),
        ] {
            for i in 0..=16 {
                let angle = start + i as f32 / 16.0 * std::f32::consts::FRAC_PI_2;
                points.push((cx + r * angle.cos(), cy + r * angle.sin()));
            }
        }
    }
    points
}
pub fn contains(points: &[(f32, f32)], x: f32, y: f32) -> bool {
    if points.is_empty() || !x.is_finite() || !y.is_finite() {
        return false;
    }
    let mut inside = false;
    let mut prev = points[points.len() - 1];
    for &p in points {
        if (p.1 > y) != (prev.1 > y) && x < (prev.0 - p.0) * (y - p.1) / (prev.1 - p.1) + p.0 {
            inside = !inside;
        }
        prev = p;
    }
    inside
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shape_excludes_transparent_corners_and_outside_points() {
        let detached = outline(300.0, 80.0, false, 1.0);
        assert!(!contains(&detached, 10.0, 10.0));
        assert!(contains(&detached, 12.0, 12.0));
        for attached in [true, false] {
            let p = outline(300.0, 80.0, attached, 1.0);
            assert!(contains(&p, 150.0, 40.0));
            assert!(!contains(&p, 0.0, 79.0));
            assert!(!contains(&p, -1.0, 40.0));
            assert!(!contains(&p, f32::NAN, 40.0));
        }
    }
}

/// Shared companion-window tokens used by GPUI and GTK Market.
pub mod market {
    pub const BG: u32 = 0x151515;
    pub const SURFACE: u32 = 0x222222;
    pub const HOVER_BG: u32 = 0x2c2c2c;
    pub const SELECTED_BG: u32 = 0x363636;
    pub const SELECTED_BORDER: u32 = 0x686868;
    pub const BORDER: u32 = 0x414141;
    pub const DIVIDER: u32 = 0x303030;
    pub const ICON_BG: u32 = 0x191919;
    pub const PREVIEW_BG: u32 = 0x303030;
    pub const FEATURE_BG: u32 = 0x252525;
    pub const ACCENT: u32 = 0x80b4ff;
    pub const FOCUS: u32 = 0xa8cbff;
    pub const ACTION_BG: u32 = 0x245cc5;
    pub const ACTION_HOVER: u32 = 0x2a65d4;
    pub const SELECTION_BG: u32 = 0x284976;
    pub const MUTED: u32 = 0xa6a6ad;
    pub const ERROR: u32 = 0xff9b91;
    pub const WARNING: u32 = 0xe8bf75;
    pub const WINDOW_WIDTH: f32 = 1000.0;
    pub const WINDOW_HEIGHT: f32 = 740.0;
    pub const MIN_WIDTH: f32 = 720.0;
    pub const MIN_HEIGHT: f32 = 520.0;
    pub const CARD_WIDTH: f32 = 286.0;
    pub const PAGE_INSET: f32 = 32.0;
    pub const PREVIEW_HEIGHT: f32 = 240.0;
    pub const TITLE_SIZE: f32 = 32.0;
    pub const WORDMARK_SIZE: f32 = 20.0;
    pub const FEATURE_TITLE_SIZE: f32 = 36.0;
    pub const CARD_TITLE_SIZE: f32 = 22.0;
    pub const BODY_SIZE: f32 = 14.0;
    pub const LABEL_SIZE: f32 = 12.0;
    pub const ICON_SIZE: f32 = 72.0;
    pub const FEATURE_ICON_SIZE: f32 = 104.0;
    pub const CARD_ICON_SIZE: f32 = 48.0;
    pub const ICON_TYPE_SIZE: f32 = 32.0;
    pub const FEATURE_ICON_TYPE_SIZE: f32 = 44.0;
    pub const CARD_RADIUS: f32 = 16.0;
    pub const CONTROL_HEIGHT: f32 = 38.0;
}
