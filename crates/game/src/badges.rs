//! Round badges drawn pixel by pixel for the pointer: a disc in the colour of a risk, ringed in dark,
//! with a white glyph telling the kind of order.

use std::f32::consts::{FRAC_PI_2, TAU};

/// Side of a badge, in pixels.
pub const BADGE_SIZE: u32 = 32;

const SAMPLES_PER_SIDE: u32 = 4;
const DISC_RADIUS: f32 = 13.0;
const RIM_WIDTH: f32 = 2.0;
const OUTLINE_WIDTH: f32 = 1.0;
const RIM: [u8; 3] = [26, 20, 15];
const INK: [u8; 3] = [255, 255, 255];

const ARROW_RIGHT: [(f32, f32); 7] = [
    (-0.6, -0.15),
    (0.08, -0.15),
    (0.08, -0.45),
    (0.62, 0.0),
    (0.08, 0.45),
    (0.08, 0.15),
    (-0.6, 0.15),
];
const ARROW_UP: [(f32, f32); 7] = [
    (-0.15, 0.6),
    (-0.15, -0.08),
    (-0.45, -0.08),
    (0.0, -0.62),
    (0.45, -0.08),
    (0.15, -0.08),
    (0.15, 0.6),
];
const BOLT: [(f32, f32); 7] = [
    (-0.08, -0.66),
    (0.4, -0.66),
    (0.1, -0.12),
    (0.4, -0.12),
    (-0.22, 0.7),
    (-0.02, 0.12),
    (-0.34, 0.12),
];
const TRAILS: [[(f32, f32); 4]; 2] = [
    [(-0.66, -0.02), (-0.08, -0.02), (-0.08, 0.12), (-0.66, 0.12)],
    [(-0.5, 0.28), (-0.12, 0.28), (-0.12, 0.42), (-0.5, 0.42)],
];
const BALL_CENTRE: (f32, f32) = (0.22, -0.18);
const BALL_RADIUS: f32 = 0.34;

/// The glyph of a badge, one per kind of order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Glyph {
    /// A walk without dice: an arrow.
    Walk,
    /// A walk with a dodge on the way: a lightning bolt.
    Dodge,
    /// A walk onto the loose ball: an arrow pointing up.
    Pick,
    /// A walk into the enemy den with the ball: a star.
    Score,
    /// An assault: a burst.
    Assault,
    /// A throw: a flying ball.
    Throw,
}

impl Glyph {
    /// Every glyph.
    pub const ALL: [Glyph; 6] = [
        Glyph::Walk,
        Glyph::Dodge,
        Glyph::Pick,
        Glyph::Score,
        Glyph::Assault,
        Glyph::Throw,
    ];
}

/// RGBA pixels of a badge, row by row from the top left: a disc filled with `fill`, a dark rim and
/// `glyph` in white, smoothed by sampling each pixel several times.
pub fn badge_pixels(glyph: Glyph, fill: [u8; 3]) -> Vec<u8> {
    (0..BADGE_SIZE)
        .flat_map(|row| (0..BADGE_SIZE).map(move |column| (column, row)))
        .flat_map(|(column, row)| pixel(glyph, fill, column, row))
        .collect()
}

fn pixel(glyph: Glyph, fill: [u8; 3], column: u32, row: u32) -> [u8; 4] {
    let centre = BADGE_SIZE as f32 / 2.0;
    let step = 1.0 / SAMPLES_PER_SIDE as f32;
    let covered: Vec<[u8; 3]> = (0..SAMPLES_PER_SIDE)
        .flat_map(|down| (0..SAMPLES_PER_SIDE).map(move |across| (across, down)))
        .filter_map(|(across, down)| {
            let x = column as f32 + (across as f32 + 0.5) * step - centre;
            let y = row as f32 + (down as f32 + 0.5) * step - centre;
            colour_at(glyph, fill, x, y)
        })
        .collect();
    if covered.is_empty() {
        return [0, 0, 0, 0];
    }
    let mean = |channel: usize| {
        let sum: u32 = covered
            .iter()
            .map(|colour| u32::from(colour[channel]))
            .sum();
        u8::try_from(sum / covered.len() as u32).unwrap_or(u8::MAX)
    };
    let coverage = covered.len() as u32 * 255 / (SAMPLES_PER_SIDE * SAMPLES_PER_SIDE);
    [
        mean(0),
        mean(1),
        mean(2),
        u8::try_from(coverage).unwrap_or(u8::MAX),
    ]
}

fn colour_at(glyph: Glyph, fill: [u8; 3], x: f32, y: f32) -> Option<[u8; 3]> {
    let distance = x.hypot(y);
    if distance > DISC_RADIUS + RIM_WIDTH {
        None
    } else if distance > DISC_RADIUS {
        Some(RIM)
    } else if glyph_covers(glyph, x / DISC_RADIUS, y / DISC_RADIUS) {
        Some(INK)
    } else if outlines(glyph, x, y) {
        Some(RIM)
    } else {
        Some(fill)
    }
}

fn outlines(glyph: Glyph, x: f32, y: f32) -> bool {
    [
        (-1.0, -1.0),
        (0.0, -1.0),
        (1.0, -1.0),
        (-1.0, 0.0),
        (1.0, 0.0),
        (-1.0, 1.0),
        (0.0, 1.0),
        (1.0, 1.0),
    ]
    .into_iter()
    .any(|(across, down)| {
        glyph_covers(
            glyph,
            (x + across * OUTLINE_WIDTH) / DISC_RADIUS,
            (y + down * OUTLINE_WIDTH) / DISC_RADIUS,
        )
    })
}

fn glyph_covers(glyph: Glyph, x: f32, y: f32) -> bool {
    match glyph {
        Glyph::Walk => inside(&ARROW_RIGHT, x, y),
        Glyph::Dodge => inside(&BOLT, x, y),
        Glyph::Pick => inside(&ARROW_UP, x, y),
        Glyph::Score => inside(&star(5, 0.66, 0.28), x, y),
        Glyph::Assault => inside(&star(8, 0.68, 0.32), x, y),
        Glyph::Throw => {
            (x - BALL_CENTRE.0).hypot(y - BALL_CENTRE.1) <= BALL_RADIUS
                || TRAILS.iter().any(|trail| inside(trail, x, y))
        }
    }
}

fn star(points: u8, outer: f32, inner: f32) -> Vec<(f32, f32)> {
    (0..u16::from(points) * 2)
        .map(|corner| {
            let radius = if corner % 2 == 0 { outer } else { inner };
            let angle = -FRAC_PI_2 + f32::from(corner) * TAU / f32::from(u16::from(points) * 2);
            (radius * angle.cos(), radius * angle.sin())
        })
        .collect()
}

fn inside(polygon: &[(f32, f32)], x: f32, y: f32) -> bool {
    let Some(&last) = polygon.last() else {
        return false;
    };
    let mut crossings = false;
    let mut previous = last;
    for &current in polygon {
        let ((x1, y1), (x2, y2)) = (previous, current);
        if (y1 > y) != (y2 > y) && x < (x2 - x1) * (y - y1) / (y2 - y1) + x1 {
            crossings = !crossings;
        }
        previous = current;
    }
    crossings
}

#[cfg(test)]
mod tests {
    use super::*;

    const GREEN: [u8; 3] = [77, 217, 102];

    fn rgba_at(pixels: &[u8], column: u32, row: u32) -> [u8; 4] {
        let start = usize::try_from((row * BADGE_SIZE + column) * 4).unwrap();
        [
            pixels[start],
            pixels[start + 1],
            pixels[start + 2],
            pixels[start + 3],
        ]
    }

    #[test]
    fn given_any_glyph_when_drawn_then_one_rgba_pixel_per_square_of_the_badge() {
        assert_eq!(badge_pixels(Glyph::Throw, GREEN).len(), 32 * 32 * 4);
    }

    #[test]
    fn given_a_walk_badge_when_reading_a_corner_then_it_is_transparent() {
        assert_eq!(
            rgba_at(&badge_pixels(Glyph::Walk, GREEN), 0, 0),
            [0, 0, 0, 0]
        );
    }

    #[test]
    fn given_a_walk_badge_when_reading_its_middle_then_the_white_arrow() {
        assert_eq!(
            rgba_at(&badge_pixels(Glyph::Walk, GREEN), 16, 16),
            [255, 255, 255, 255]
        );
    }

    #[test]
    fn given_a_walk_badge_when_reading_below_the_arrow_then_the_risk_colour() {
        assert_eq!(
            rgba_at(&badge_pixels(Glyph::Walk, GREEN), 16, 25),
            [77, 217, 102, 255]
        );
    }

    #[test]
    fn given_a_walk_badge_when_reading_its_edge_then_the_dark_rim() {
        assert_eq!(
            rgba_at(&badge_pixels(Glyph::Walk, GREEN), 30, 16),
            [26, 20, 15, 255]
        );
    }

    #[test]
    fn given_two_glyphs_when_drawn_on_the_same_colour_then_they_differ() {
        assert_ne!(
            badge_pixels(Glyph::Assault, GREEN),
            badge_pixels(Glyph::Score, GREEN)
        );
    }
}
