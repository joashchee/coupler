//! Coupler's painter: a picture of the room with no model, drawn in
//! ANSI from what the game says about it. Free, instant and the same every visit, so it shows at
//! once and stands in whenever a model isn't there, is busy or is off.
//! Pure and unit-tested.
//!
//! **How it draws.** Each character cell is two square pixels, one above
//! the other (`▀`, its top in the foreground color and its bottom in the
//! background), in the 16 VGA colors as palette indexes, so the view's
//! palette colors it as it colors the game. Characters from the IBM VGA
//! font go over the pixels for what a pixel can't show: stars, rain,
//! snow, waves, leaves, runes. The result is the same `Art` an ANSI file
//! becomes (`ansi_art.rs`), shown by `AnsiPicture.tsx`.
//!
//! **What it reads** (`paint::Scene`): the terrain (CoffeeMUD's 24, and
//! whether it's indoors, `ambient::room_type`), the time of day and the
//! weather. The sky follows the time (a sun by day, a moon and stars at
//! night, each crossing the sky left to right as the hours pass: low at
//! its rising on the left, highest halfway, low again at its setting on
//! the right; `Scene::arc`, from `daytime.rs`) and the weather (clouds, rain,
//! snow, hail, fog, dust, a storm's lightning), and only where there's
//! a sky (`ambient::has_sky`). The land follows the terrain: plains,
//! woods, hills, mountains, a city's roofs, the sea; indoors a room in
//! one-point perspective in its material (stone, wood, metal, magic)
//! with a torch, or a cave. Indoors and underwater show nothing of the
//! sky: no sun, moon, stars or weather (a wooden wall's window has its
//! shutters closed). The room's ID seeds the rest (`paint::seed`):
//! where the hills rise and the trees stand. The sun and the moon go by
//! the clock alone, so every room's sky agrees.
//!
//! **The two styles.** Fantasy is the plain landscape. High fantasy adds
//! the motifs: a castle's towers on the horizon, a second moon and an
//! aurora at night, banners and braziers indoors.
//!
//! It can't read the description, so a forest looks like a forest, not
//! *this* forest: that's what a model is for.

use crate::ambient;
use crate::ansi::{Color, Line, Span, Style as Sgr};
use crate::ansi_art::Art;
use crate::paint::{seed, Scene, Style};

/// The picture's size in cells is the panel's; within the screen.
const MOST_COLUMNS: usize = 160;
const MOST_ROWS: usize = 45;
const LEAST_COLUMNS: usize = 8;
const LEAST_ROWS: usize = 4;

// The 16 VGA colors, by their ANSI index.
pub(crate) const BLACK: u8 = 0;
pub(crate) const RED: u8 = 1;
pub(crate) const GREEN: u8 = 2;
pub(crate) const BROWN: u8 = 3;
pub(crate) const BLUE: u8 = 4;
pub(crate) const MAGENTA: u8 = 5;
pub(crate) const CYAN: u8 = 6;
pub(crate) const LIGHTGRAY: u8 = 7;
pub(crate) const DARKGRAY: u8 = 8;
pub(crate) const LIGHTRED: u8 = 9;
pub(crate) const LIGHTGREEN: u8 = 10;
pub(crate) const YELLOW: u8 = 11;
pub(crate) const LIGHTBLUE: u8 = 12;
pub(crate) const LIGHTMAGENTA: u8 = 13;
pub(crate) const LIGHTCYAN: u8 = 14;
pub(crate) const WHITE: u8 = 15;

/// A 4 by 4 ordered dither: two colors mixed in sixteenths without noise.
const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Paints a room at a size in cells.
pub fn paint(scene: &Scene, style: Style, columns: usize, rows: usize, again: u32) -> Art {
    let columns = columns.clamp(LEAST_COLUMNS, MOST_COLUMNS);
    let rows = rows.clamp(LEAST_ROWS, MOST_ROWS);
    draw(scene, style, columns, rows, again).art()
}

/// The time of day, as the light sees it; day when unknown.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Light {
    Dawn,
    Day,
    Dusk,
    Night,
}

impl Light {
    fn of(time: Option<&str>) -> Light {
        match time {
            Some("dawn") => Light::Dawn,
            Some("dusk") => Light::Dusk,
            Some("night") => Light::Night,
            _ => Light::Day,
        }
    }
}

/// What the weather does to a picture.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sky {
    Clear,
    Cloudy,
    Windy,
    Rain,
    Storm,
    Snow,
    Blizzard,
    Hail,
    Sleet,
    Fog,
    Dust,
    Hot,
    Cold,
}

impl Sky {
    /// From `coupler.weather`'s kinds (`ambient.rs`); clear when unknown.
    fn of(weather: Option<&str>) -> Sky {
        match weather {
            Some("cloudy") => Sky::Cloudy,
            Some("windy") => Sky::Windy,
            Some("rain") => Sky::Rain,
            Some("thunderstorm") => Sky::Storm,
            Some("snow") => Sky::Snow,
            Some("blizzard") => Sky::Blizzard,
            Some("hail") => Sky::Hail,
            Some("sleet") => Sky::Sleet,
            Some("fog") => Sky::Fog,
            Some("dust") => Sky::Dust,
            Some("heat" | "drought") => Sky::Hot,
            Some("cold") => Sky::Cold,
            _ => Sky::Clear,
        }
    }

    /// A grey sky hides the sun, the moon and the stars.
    fn overcast(self) -> bool {
        matches!(self, Sky::Rain | Sky::Storm | Sky::Snow | Sky::Blizzard | Sky::Hail | Sky::Sleet | Sky::Fog | Sky::Dust)
    }
}

/// xorshift64*: small, fast and the same everywhere.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    pub(crate) fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// 0 up to `n` (not including it); 0 when `n` is 0 or less.
    pub(crate) fn below(&mut self, n: i32) -> i32 {
        if n <= 0 {
            return 0;
        }
        (self.next() >> 33) as i32 % n
    }

    /// `a` to `b`, both included.
    pub(crate) fn between(&mut self, a: i32, b: i32) -> i32 {
        a + self.below(b - a + 1)
    }

    /// True `percent` times in a hundred.
    pub(crate) fn chance(&mut self, percent: i32) -> bool {
        self.below(100) < percent
    }
}

/// Pixels and the characters laid over them.
pub(crate) struct Canvas {
    /// In pixels: the columns, and twice the rows.
    pub(crate) w: i32,
    pub(crate) h: i32,
    pixels: Vec<u8>,
    /// Per cell: a character and its color.
    glyphs: Vec<Option<(char, u8)>>,
    /// Pixels that give their own light (windows, fires, the moon): night leaves them be.
    lit: Vec<bool>,
}

impl Canvas {
    pub(crate) fn new(columns: usize, rows: usize) -> Canvas {
        let (w, h) = (columns as i32, rows as i32 * 2);
        Canvas { w, h, pixels: vec![BLACK; (w * h) as usize], glyphs: vec![None; columns * rows], lit: vec![false; (w * h) as usize] }
    }

    pub(crate) fn inside(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }

    pub(crate) fn get(&self, x: i32, y: i32) -> u8 {
        if self.inside(x, y) {
            self.pixels[(y * self.w + x) as usize]
        } else {
            BLACK
        }
    }

    pub(crate) fn set(&mut self, x: i32, y: i32, c: u8) {
        if self.inside(x, y) {
            let i = (y * self.w + x) as usize;
            self.pixels[i] = c;
            self.lit[i] = false;
        }
    }

    /// A pixel that keeps its color at night.
    pub(crate) fn light(&mut self, x: i32, y: i32, c: u8) {
        if self.inside(x, y) {
            let i = (y * self.w + x) as usize;
            self.pixels[i] = c;
            self.lit[i] = true;
        }
    }

    /// A character over the cell holding pixel (x, y).
    pub(crate) fn glyph(&mut self, x: i32, y: i32, c: char, fg: u8) {
        if self.inside(x, y) {
            self.glyphs[((y / 2) * self.w + x) as usize] = Some((c, fg));
        }
    }

    fn has_glyph(&self, x: i32, y: i32) -> bool {
        self.inside(x, y) && self.glyphs[((y / 2) * self.w + x) as usize].is_some()
    }

    pub(crate) fn rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u8) {
        for y in y0.max(0)..y1.min(self.h) {
            for x in x0.max(0)..x1.min(self.w) {
                self.set(x, y, c);
            }
        }
    }

    /// An ellipse, filled.
    pub(crate) fn oval(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, c: u8) {
        let (rx, ry) = (rx.max(1), ry.max(1));
        for y in cy - ry..=cy + ry {
            for x in cx - rx..=cx + rx {
                let (dx, dy) = (x - cx, y - cy);
                if dx * dx * ry * ry + dy * dy * rx * rx <= rx * rx * ry * ry {
                    self.set(x, y, c);
                }
            }
        }
    }

    /// A filled triangle standing on its base: its peak at (x, top), its
    /// base `half` pixels each side at `bottom`.
    fn peak(&mut self, x: i32, top: i32, bottom: i32, half: i32, c: u8) {
        let tall = (bottom - top).max(1);
        for y in top..=bottom {
            let reach = half * (y - top) / tall;
            for dx in -reach..=reach {
                self.set(x + dx, y, c);
            }
        }
    }

    /// Two colors mixed: `b` for `t` of every 16 pixels.
    pub(crate) fn mix(x: i32, y: i32, a: u8, b: u8, t: i32) -> u8 {
        if i32::from(BAYER[(y & 3) as usize][(x & 3) as usize]) < t {
            b
        } else {
            a
        }
    }

    /// Bands of color from `y0` down to `y1`: solid, dithered only
    /// where one meets the next, as ANSI art draws a sky.
    pub(crate) fn gradient(&mut self, y0: i32, y1: i32, stops: &[u8]) {
        let span = (y1 - y0).max(1);
        let steps = (stops.len() as i32 - 1).max(1);
        for y in y0.max(0)..y1.min(self.h) {
            let at = (y - y0) * steps * 16 / span;
            let (i, t) = ((at / 16) as usize, ((at % 16 - 5) * 16 / 6).clamp(0, 16));
            let a = stops[i.min(stops.len() - 1)];
            let b = stops[(i + 1).min(stops.len() - 1)];
            for x in 0..self.w {
                self.set(x, y, Canvas::mix(x, y, a, b, t));
            }
        }
    }

    /// Every pixel so far keeps its color at night: the sky, drawn in
    /// its night colors already. What's drawn over it after doesn't.
    fn keep(&mut self) {
        self.lit.fill(true);
    }

    /// Night: what doesn't give light goes dark, and so do the
    /// characters over it.
    fn darken(&mut self) {
        for y in 0..self.h {
            for x in 0..self.w {
                let i = (y * self.w + x) as usize;
                if !self.lit[i] {
                    self.pixels[i] = dim(self.pixels[i], x, y);
                }
            }
        }
        for (i, glyph) in self.glyphs.iter_mut().enumerate() {
            if let Some((_, fg)) = glyph {
                let (x, y) = (i as i32 % self.w, (i as i32 / self.w) * 2);
                if !self.lit[(y * self.w + x) as usize] && *fg >= 8 && *fg != DARKGRAY && *fg != YELLOW {
                    *fg -= 8;
                }
            }
        }
    }

    /// The cells as lines of styled characters.
    pub(crate) fn art(&self) -> Art {
        let rows = (self.h / 2) as usize;
        let mut lines: Vec<Line> = Vec::with_capacity(rows);
        for row in 0..rows as i32 {
            let mut line: Line = Vec::new();
            for x in 0..self.w {
                let (top, bottom) = (self.get(x, row * 2), self.get(x, row * 2 + 1));
                let (c, fg, bg) = match self.glyphs[(row * self.w + x) as usize] {
                    Some((c, fg)) => (c, fg, top),
                    None if top == bottom => (' ', top, top),
                    None => ('▀', top, bottom),
                };
                let style = Sgr { fg: Some(Color::Index { index: fg }), bg: Some(Color::Index { index: bg }), ..Sgr::default() };
                // A space's foreground doesn't show: let it join the run before it.
                match line.last_mut() {
                    Some(span) if span.style == style || (c == ' ' && span.style.bg == style.bg) => span.text.push(c),
                    _ => line.push(Span { text: c.to_string(), style }),
                }
            }
            lines.push(line);
        }
        Art { columns: self.w as usize, rows, lines }
    }
}

/// A color at night: a bright one goes to its dark self, a dark one
/// halfway to black.
fn dim(c: u8, x: i32, y: i32) -> u8 {
    match c {
        BLACK => BLACK,
        DARKGRAY => Canvas::mix(x, y, DARKGRAY, BLACK, 8),
        c if c >= 8 => c - 8,
        c => Canvas::mix(x, y, c, BLACK, 8),
    }
}

/// A smooth line of heights across the picture: random points every
/// `step` pixels, eased between. Each height is 0 to `most`.
fn ridge(rng: &mut Rng, w: i32, most: i32, step: i32) -> Vec<i32> {
    let step = step.max(1);
    let points: Vec<i32> = (0..=w / step + 1).map(|_| rng.between(0, most.max(0))).collect();
    (0..w)
        .map(|x| {
            let (i, t) = ((x / step) as usize, f64::from(x % step) / f64::from(step));
            let ease = (1.0 - (t * std::f64::consts::PI).cos()) / 2.0;
            let (a, b) = (f64::from(points[i]), f64::from(points[i + 1]));
            (a + (b - a) * ease).round() as i32
        })
        .collect()
}

/// Draws a room's picture.
fn draw(scene: &Scene, style: Style, columns: usize, rows: usize, again: u32) -> Canvas {
    let mut rng = Rng::new(seed(&scene.world, &scene.room, again));
    let mut canvas = Canvas::new(columns, rows);
    let terrain = scene.terrain.to_ascii_lowercase();
    let light = Light::of(scene.time.as_deref());
    // Without the hour, the part of the day alone places it.
    let arc = scene.arc.unwrap_or(match light {
        Light::Dawn => 60,
        Light::Dusk => 940,
        _ => 500,
    });
    let sky = if ambient::has_sky(&terrain) { Sky::of(scene.weather.as_deref()) } else { Sky::Clear };
    let high = style == Style::High;
    match ambient::room_type(&terrain) {
        // Indoors and underwater, nothing of the sky: no sun, moon, stars or weather.
        Some("indoors") => indoors(&mut canvas, &mut rng, &terrain, high),
        _ if terrain == "underwater" => underwater(&mut canvas, &mut rng),
        _ => outdoors(&mut canvas, &mut rng, &terrain, light, arc, sky, high),
    }
    canvas
}

// ---- Outdoors ----

fn outdoors(canvas: &mut Canvas, rng: &mut Rng, terrain: &str, light: Light, arc: u16, sky: Sky, high: bool) {
    let (w, h) = (canvas.w, canvas.h);
    let horizon = match terrain {
        "air" => h,
        "mountains" => h * 60 / 100,
        "watersurface" | "seaport" => h * 50 / 100,
        _ => h * 45 / 100,
    };
    heavens(canvas, rng, horizon, light, arc, sky, high);
    canvas.keep();
    match terrain {
        "air" => air(canvas, rng),
        "woods" | "jungle" => woods(canvas, rng, horizon, terrain == "jungle"),
        "hills" => hills(canvas, rng, horizon),
        "mountains" => mountains(canvas, rng, horizon),
        "rocky" => rocky(canvas, rng, horizon),
        "desert" => desert(canvas, rng, horizon),
        "swamp" => swamp(canvas, rng, horizon),
        "watersurface" => sea(canvas, rng, horizon, 0),
        "seaport" => {
            sea(canvas, rng, horizon, w / 2);
            town(canvas, rng, horizon, w / 2, light);
            ship(canvas, rng, w * 3 / 4, horizon + (h - horizon) / 3);
        }
        "city" => town(canvas, rng, horizon, w, light),
        "spaceport" => spaceport(canvas, rng, horizon, light),
        _ => plains(canvas, rng, horizon),
    }
    if high && !matches!(terrain, "air" | "city" | "spaceport") {
        castle(canvas, rng, horizon, light);
    }
    if light == Light::Night {
        canvas.darken();
    }
    if sky == Sky::Fog {
        fog(canvas, horizon, light);
    }
    weather(canvas, rng, horizon, light, sky);
}

/// Where on the sky a sun or moon of radius `r` is, `arc` thousandths of
/// the way across: from half risen on the left, up to near the top
/// halfway, to half set on the right.
fn arc_at(w: i32, horizon: i32, r: i32, arc: u16) -> (i32, i32) {
    let t = f64::from(arc.min(1000)) / 1000.0;
    let x = r + (f64::from((w - 1 - 2 * r).max(0)) * t).round() as i32;
    let (low, top) = (horizon - r / 2, (r + 1).min(horizon - r / 2));
    let y = low - (f64::from(low - top) * (std::f64::consts::PI * t).sin()).round() as i32;
    (x, y)
}

/// The sky down to the horizon: its colors by the time and the weather,
/// the sun or the moon and stars, clouds, an aurora.
fn heavens(canvas: &mut Canvas, rng: &mut Rng, horizon: i32, light: Light, arc: u16, sky: Sky, high: bool) {
    let night = light == Light::Night;
    let stops: &[u8] = match (sky, light) {
        (Sky::Storm | Sky::Blizzard, Light::Night) => &[BLACK, BLACK, DARKGRAY],
        (Sky::Storm | Sky::Blizzard, _) => &[BLACK, DARKGRAY, DARKGRAY, LIGHTGRAY],
        (Sky::Rain | Sky::Hail | Sky::Sleet | Sky::Snow | Sky::Fog, Light::Dawn | Light::Dusk) => &[BLACK, DARKGRAY, LIGHTGRAY],
        (Sky::Rain | Sky::Hail | Sky::Sleet, Light::Night) => &[BLACK, DARKGRAY],
        (Sky::Rain | Sky::Hail | Sky::Sleet, _) => &[DARKGRAY, LIGHTGRAY],
        (Sky::Snow, Light::Night) => &[BLACK, DARKGRAY, LIGHTGRAY],
        (Sky::Snow, _) => &[DARKGRAY, LIGHTGRAY],
        (Sky::Fog, Light::Night) => &[BLACK, DARKGRAY],
        (Sky::Fog, _) => &[LIGHTGRAY, LIGHTGRAY, WHITE],
        (Sky::Dust, Light::Night) => &[BLACK, BROWN],
        (Sky::Dust, _) => &[BROWN, YELLOW],
        (_, Light::Night) => &[BLACK, BLACK, BLUE],
        (_, Light::Dawn) => &[BLUE, MAGENTA, LIGHTRED, YELLOW],
        (_, Light::Dusk) => &[BLACK, BLUE, MAGENTA, RED, LIGHTRED],
        (Sky::Hot, Light::Day) => &[LIGHTBLUE, LIGHTCYAN, YELLOW],
        (Sky::Cold, Light::Day) => &[LIGHTBLUE, LIGHTCYAN, WHITE],
        (_, Light::Day) => &[BLUE, LIGHTBLUE, LIGHTCYAN],
    };
    canvas.gradient(0, horizon, stops);
    let (w, sky_h) = (canvas.w, horizon.max(1));

    if night && !sky.overcast() {
        if high {
            aurora(canvas, rng, sky_h);
        }
        let stars = (w * sky_h / 18).max(1);
        for _ in 0..stars {
            let (x, y) = (rng.below(w), rng.below(sky_h - 1));
            let (c, fg) = match rng.below(10) {
                0 => ('*', WHITE),
                1 => ('+', LIGHTGRAY),
                2..=5 => ('∙', LIGHTGRAY),
                _ => ('.', WHITE),
            };
            if !canvas.has_glyph(x, y) {
                canvas.glyph(x, y, c, fg);
            }
        }
        let r = (sky_h / 5).clamp(2, 4);
        // Drawn and not used (the moon goes by the clock now), so what's
        // drawn after keeps each room's look.
        let _ = (rng.between(w / 5, w * 4 / 5), rng.between(r + 1, (sky_h / 2).max(r + 1)));
        let (mx, my) = arc_at(w, horizon, r, arc);
        orb(canvas, mx, my, r, WHITE, LIGHTGRAY);
        if high {
            // The second moon, smaller and red, a third of the sky behind or ahead.
            let _ = rng.between(1, (sky_h / 3).max(1));
            let other = if arc > 500 { arc - 333 } else { arc + 333 };
            let (sx, sy) = arc_at(w, horizon, (r / 2).max(1), other);
            orb(canvas, sx, sy, (r / 2).max(1), LIGHTRED, RED);
        }
    } else if !sky.overcast() {
        let r = (sky_h / 5).clamp(1, 5);
        if !matches!(light, Light::Dawn | Light::Dusk) {
            // As with the moon: drawn, not used, for the land's sake.
            let _ = (rng.between(w / 6, w * 5 / 6), rng.between(r, (sky_h / 3).max(r)));
        }
        let (sx, sy) = arc_at(w, horizon, r, arc);
        let hot = if sky == Sky::Hot { r + 1 } else { r };
        orb(canvas, sx, sy, hot, YELLOW, if light == Light::Day { YELLOW } else { LIGHTRED });
    }

    if matches!(sky, Sky::Cloudy | Sky::Storm | Sky::Windy) {
        let (body, shade) = match (sky, night) {
            (_, true) => (DARKGRAY, BLACK),
            (Sky::Storm, _) => (DARKGRAY, BLACK),
            _ => (WHITE, LIGHTGRAY),
        };
        let clouds = if sky == Sky::Windy { 2 } else { (w / 12).max(2) };
        for _ in 0..clouds {
            let (cx, cy) = (rng.below(w), rng.between(1, (sky_h * 2 / 3).max(1)));
            let (rx, ry) = (rng.between(3, 3 + w / 10), rng.between(1, 2));
            if sky != Sky::Storm {
                canvas.oval(cx, cy + 1, rx, ry, shade);
            }
            canvas.oval(cx, cy, rx, ry, body);
            canvas.oval(cx - rx / 2, cy - 1, rx / 2, ry, body);
        }
    }
    if sky == Sky::Windy {
        for _ in 0..(w * sky_h / 40).max(1) {
            let (x, y) = (rng.below(w), rng.below(sky_h));
            canvas.glyph(x, y, if rng.chance(50) { '~' } else { '-' }, if night { DARKGRAY } else { WHITE });
        }
    }
}

/// A sun or a moon: a disc with a shaded edge, giving its own light.
fn orb(canvas: &mut Canvas, cx: i32, cy: i32, r: i32, body: u8, edge: u8) {
    for y in cy - r..=cy + r {
        for x in cx - r..=cx + r {
            let d = (x - cx) * (x - cx) + (y - cy) * (y - cy);
            if d <= r * r {
                let c = if d > (r - 1) * (r - 1) && r > 1 { edge } else { body };
                canvas.light(x, y, c);
            }
        }
    }
}

/// High fantasy's night: green and cyan curtains in the upper sky.
fn aurora(canvas: &mut Canvas, rng: &mut Rng, sky_h: i32) {
    let wave = ridge(rng, canvas.w, (sky_h / 3).max(1), 6);
    for x in 0..canvas.w {
        let top = wave[x as usize];
        let tall = (sky_h / 4).max(2);
        if x % 4 == 3 {
            continue;
        }
        for y in top..top + tall {
            let fade = 10 - (y - top) * 10 / tall;
            let c = Canvas::mix(x, y, canvas.get(x, y), if (x / 4) % 2 == 0 { GREEN } else { CYAN }, fade);
            canvas.set(x, y, c);
        }
        if x % 4 == 1 {
            canvas.set(x, top, LIGHTGREEN);
        }
    }
}

/// Grass to the horizon, a road, a few far hills.
fn plains(canvas: &mut Canvas, rng: &mut Rng, horizon: i32) {
    let (w, h) = (canvas.w, canvas.h);
    let far = ridge(rng, w, 2, 10);
    for x in 0..w {
        for y in horizon - far[x as usize]..horizon {
            canvas.set(x, y, Canvas::mix(x, y, GREEN, CYAN, 4));
        }
    }
    canvas.gradient(horizon, h, &[GREEN, LIGHTGREEN, LIGHTGREEN]);
    road(canvas, rng, horizon, BROWN);
    for _ in 0..(w * (h - horizon) / 16).max(1) {
        let (x, y) = (rng.below(w), rng.between(horizon + 2, h - 1));
        if canvas.get(x, y) != BROWN {
            canvas.glyph(x, y, if rng.chance(50) { '"' } else { ',' }, GREEN);
        }
    }
}

/// A road from near the middle of the horizon to the bottom edge.
fn road(canvas: &mut Canvas, rng: &mut Rng, horizon: i32, c: u8) {
    let (w, h) = (canvas.w, canvas.h);
    let (top, bottom) = (w / 2 + rng.between(-w / 8, w / 8), w / 2 + rng.between(-w / 4, w / 4));
    let tall = (h - horizon).max(1);
    for y in horizon..h {
        let t = y - horizon;
        let centre = top + (bottom - top) * t / tall;
        let half = t * w / 14 / tall;
        for x in centre - half..=centre + half {
            canvas.set(x, y, c);
        }
    }
}

/// A tree line on the horizon, then trees standing in the grass.
fn woods(canvas: &mut Canvas, rng: &mut Rng, horizon: i32, jungle: bool) {
    let (w, h) = (canvas.w, canvas.h);
    let line = ridge(rng, w, (horizon / 3).max(1), 3);
    for x in 0..w {
        for y in horizon - line[x as usize] - 1..horizon {
            canvas.set(x, y, Canvas::mix(x, y, GREEN, DARKGRAY, 4));
        }
    }
    canvas.gradient(horizon, h, &[GREEN, GREEN, LIGHTGREEN]);
    // Near trees: a trunk and a crown, larger lower down.
    let trees = (w / if jungle { 5 } else { 7 }).max(2);
    for _ in 0..trees {
        let x = rng.below(w);
        let foot = rng.between(horizon + 1, h - 1);
        let size = 1 + (foot - horizon) * 3 / (h - horizon).max(1);
        let crown = foot - size * 3;
        canvas.rect(x, crown, x + 1, foot + 1, BROWN);
        canvas.oval(x, crown, size + 1, size + 1, if jungle { LIGHTGREEN } else { GREEN });
        canvas.oval(x - 1, crown - 1, size, size, LIGHTGREEN);
    }
    let leaves = if jungle { ['♠', '♣'] } else { ['♣', '♣'] };
    for _ in 0..(w * (h - horizon) / if jungle { 6 } else { 12 }).max(1) {
        let (x, y) = (rng.below(w), rng.between(horizon, h - 1));
        canvas.glyph(x, y, leaves[rng.below(2) as usize], if rng.chance(50) { LIGHTGREEN } else { GREEN });
    }
    if jungle {
        // The canopy hangs over the top of the picture.
        let hang = ridge(rng, w, (h / 6).max(1), 4);
        for x in 0..w {
            for y in 0..hang[x as usize] {
                canvas.set(x, y, Canvas::mix(x, y, GREEN, LIGHTGREEN, 4));
            }
            if rng.chance(15) {
                canvas.glyph(x, hang[x as usize] + rng.below(4), '§', GREEN);
            }
        }
    }
}

/// Rolling hills, one layer behind another.
fn hills(canvas: &mut Canvas, rng: &mut Rng, horizon: i32) {
    let (w, h) = (canvas.w, canvas.h);
    canvas.gradient(horizon, h, &[GREEN, LIGHTGREEN]);
    // Far to near: hazy, green, bright; each with a darker crest.
    let layers: [(i32, u8, u8); 3] = [(0, CYAN, CYAN), ((h - horizon) / 3, GREEN, GREEN), ((h - horizon) * 2 / 3, LIGHTGREEN, GREEN)];
    for (i, (drop, body, crest)) in layers.into_iter().enumerate() {
        let line = ridge(rng, w, (horizon / 2).max(2), (w / 4).max(4));
        for x in 0..w {
            let top = horizon + drop - line[x as usize];
            for y in top..h {
                let c = body;
                canvas.set(x, y, if y == top && i > 0 { crest } else { c });
            }
        }
    }
}

/// Jagged peaks with snow on their tops, stony ground before them.
fn mountains(canvas: &mut Canvas, rng: &mut Rng, horizon: i32) {
    let (w, h) = (canvas.w, canvas.h);
    canvas.gradient(horizon, h, &[DARKGRAY, BROWN, DARKGRAY]);
    for layer in 0..2 {
        let peaks = (w / 10).max(2);
        for _ in 0..peaks {
            let x = rng.below(w);
            let tall = rng.between(horizon / 3, horizon * 5 / 6 - layer * horizon / 4);
            let top = horizon - tall;
            let half = tall + rng.between(0, tall / 2);
            let (body, shade) = if layer == 0 { (LIGHTGRAY, DARKGRAY) } else { (DARKGRAY, BLACK) };
            canvas.peak(x, top, horizon + layer * 2, half, body);
            // The far side in shadow.
            for y in top..=horizon {
                let reach = half * (y - top) / (horizon - top).max(1);
                for dx in 1..=reach {
                    canvas.set(x + dx, y, shade);
                }
            }
            // Snow on the top quarter.
            canvas.peak(x, top, top + tall / 4, half / 4, WHITE);
        }
    }
    for _ in 0..(w / 4).max(1) {
        let (x, y) = (rng.below(w), rng.between(horizon + 2, h - 1));
        canvas.glyph(x, y, '▲', LIGHTGRAY);
    }
}

/// Boulders on stony ground.
fn rocky(canvas: &mut Canvas, rng: &mut Rng, horizon: i32) {
    let (w, h) = (canvas.w, canvas.h);
    let far = ridge(rng, w, (horizon / 4).max(1), 4);
    for x in 0..w {
        for y in horizon - far[x as usize]..horizon {
            canvas.set(x, y, DARKGRAY);
        }
    }
    canvas.gradient(horizon, h, &[DARKGRAY, BROWN]);
    for _ in 0..(w / 6).max(2) {
        let (x, y) = (rng.below(w), rng.between(horizon + 1, h - 1));
        let r = 1 + (y - horizon) * 3 / (h - horizon).max(1);
        canvas.oval(x, y + 1, r + 1, r, DARKGRAY);
        canvas.oval(x, y, r, r, LIGHTGRAY);
    }
    for _ in 0..(w * (h - horizon) / 20).max(1) {
        let (x, y) = (rng.below(w), rng.between(horizon, h - 1));
        canvas.glyph(x, y, '∙', LIGHTGRAY);
    }
}

/// Dunes, their crests shaded on the side away from the light.
fn desert(canvas: &mut Canvas, rng: &mut Rng, horizon: i32) {
    let (w, h) = (canvas.w, canvas.h);
    canvas.gradient(horizon, h, &[YELLOW, BROWN]);
    for drop in [0, (h - horizon) / 3, (h - horizon) * 2 / 3] {
        let line = ridge(rng, w, (horizon / 3).max(2), (w / 3).max(4));
        for x in 0..w {
            let top = horizon + drop - line[x as usize];
            let rising = x + 1 < w && line[(x + 1) as usize] > line[x as usize];
            for y in top..h {
                canvas.set(x, y, if !rising && y <= top + 1 { BROWN } else { YELLOW });
            }
        }
    }
}

/// Murky ground, pools, reeds and a dead tree in the mist.
fn swamp(canvas: &mut Canvas, rng: &mut Rng, horizon: i32) {
    let (w, h) = (canvas.w, canvas.h);
    canvas.gradient(horizon, h, &[DARKGRAY, GREEN, BROWN]);
    for _ in 0..(w / 8).max(2) {
        let (x, y) = (rng.below(w), rng.between(horizon + 2, h - 1));
        let rx = rng.between(2, 3 + w / 10);
        canvas.rect(x - rx + 1, y - 1, x + rx - 1, y, CYAN);
        canvas.rect(x - rx, y, x + rx, y + 1, BLUE);
    }
    let x = rng.between(w / 6, w * 5 / 6);
    canvas.rect(x, horizon - horizon / 2, x + 1, horizon + 2, DARKGRAY);
    canvas.rect(x - 2, horizon - horizon / 3, x, horizon - horizon / 3 + 1, DARKGRAY);
    canvas.rect(x + 1, horizon - horizon / 4, x + 3, horizon - horizon / 4 + 1, DARKGRAY);
    for _ in 0..(w / 2).max(2) {
        let (x, y) = (rng.below(w), rng.between(horizon, h - 1));
        canvas.glyph(x, y, if rng.chance(70) { '|' } else { '¡' }, if rng.chance(50) { BROWN } else { GREEN });
    }
    // Mist lying on the horizon.
    for y in horizon - 2..horizon + 2 {
        for x in 0..canvas.w {
            let c = Canvas::mix(x, y, canvas.get(x, y), LIGHTGRAY, 5);
            canvas.set(x, y, c);
        }
    }
}

/// Open water from `from` (a pixel column) to the right edge, waves on
/// it, a far shore, and the sun's road on the water.
fn sea(canvas: &mut Canvas, rng: &mut Rng, horizon: i32, from: i32) {
    let (w, h) = (canvas.w, canvas.h);
    let shore = ridge(rng, w, 2, 8);
    for x in from..w {
        if rng.chance(70) {
            canvas.set(x, horizon - 1 - shore[x as usize].min(1), DARKGRAY);
        }
        for y in horizon..h {
            let t = (y - horizon) * 16 / (h - horizon).max(1);
            canvas.set(x, y, Canvas::mix(x, y, LIGHTBLUE, BLUE, t));
        }
    }
    for _ in 0..((w - from) * (h - horizon) / 10).max(1) {
        let (x, y) = (rng.between(from, w - 1), rng.between(horizon, h - 1));
        canvas.glyph(x, y, if rng.chance(70) { '~' } else { '≈' }, if rng.chance(50) { LIGHTCYAN } else { CYAN });
    }
}

/// Houses along the horizon from the left edge to `to`, roofs and
/// windows, cobbles before them.
fn town(canvas: &mut Canvas, rng: &mut Rng, horizon: i32, to: i32, light: Light) {
    let h = canvas.h;
    for y in horizon..h {
        for x in 0..to {
            canvas.set(x, y, Canvas::mix(x, y, LIGHTGRAY, DARKGRAY, 3));
        }
    }
    let mut x = -rng.below(3);
    while x < to {
        let wide = rng.between(4, 9);
        let tall = rng.between(horizon / 3, horizon * 2 / 3);
        let top = horizon - tall;
        let wall = [LIGHTGRAY, BROWN, DARKGRAY, WHITE][rng.below(4) as usize];
        canvas.rect(x, top, (x + wide).min(to), horizon + 1, wall);
        let roof = [RED, BROWN, DARKGRAY][rng.below(3) as usize];
        if rng.chance(60) {
            canvas.peak(x + wide / 2, top - wide / 2, top, wide / 2, roof);
        } else {
            canvas.rect(x, top - 1, (x + wide).min(to), top, roof);
        }
        // Windows: dark by day, lit at night.
        let mut wy = top + 1;
        while wy < horizon - 1 {
            let mut wx = x + 1;
            while wx < (x + wide - 1).min(to) {
                if light == Light::Night {
                    if rng.chance(55) {
                        canvas.light(wx, wy, YELLOW);
                    }
                } else {
                    canvas.set(wx, wy, if light == Light::Day { BLUE } else { BLACK });
                }
                wx += 2;
            }
            wy += 3;
        }
        x += wide + rng.below(2);
    }
}

/// A sailing ship on the water.
fn ship(canvas: &mut Canvas, rng: &mut Rng, x: i32, y: i32) {
    let long = rng.between(4, 7);
    for i in 0..2 {
        canvas.rect(x - long + i, y + i, x + long - i, y + i + 1, BROWN);
    }
    canvas.rect(x, y - long, x + 1, y, BROWN);
    canvas.peak(x + 1, y - long, y - 1, long / 2, WHITE);
    canvas.light(x, y - long - 1, RED);
}

/// The sci-fi ports' landing field: a tower with a beacon, a ship on the pad.
fn spaceport(canvas: &mut Canvas, rng: &mut Rng, horizon: i32, light: Light) {
    let (w, h) = (canvas.w, canvas.h);
    canvas.gradient(horizon, h, &[DARKGRAY, LIGHTGRAY]);
    for x in (0..w).step_by(6) {
        for y in horizon..h {
            canvas.set(x + (y - horizon) * (x - w / 2) / (w / 2).max(1) / 3, y, WHITE);
        }
    }
    let tx = rng.between(2, w / 4);
    canvas.rect(tx, horizon / 4, tx + 3, horizon + 1, DARKGRAY);
    canvas.rect(tx - 1, horizon / 4, tx + 4, horizon / 4 + 2, LIGHTGRAY);
    canvas.light(tx + 1, horizon / 4 - 1, LIGHTRED);
    let (sx, sy) = (rng.between(w / 2, w * 3 / 4), horizon + (h - horizon) / 2);
    canvas.oval(sx, sy, (w / 8).max(3), 2, WHITE);
    canvas.oval(sx, sy + 1, (w / 8).max(3), 1, LIGHTGRAY);
    canvas.rect(sx - 1, sy - 3, sx + 2, sy - 1, LIGHTCYAN);
    if light == Light::Night {
        canvas.light(sx - w / 8, sy, LIGHTRED);
        canvas.light(sx + w / 8, sy, LIGHTGREEN);
    }
}

/// From high above: cloud tops below, birds about.
fn air(canvas: &mut Canvas, rng: &mut Rng) {
    let (w, h) = (canvas.w, canvas.h);
    let tops = ridge(rng, w, (h / 6).max(1), 5);
    for x in 0..w {
        for y in h * 3 / 4 - tops[x as usize]..h {
            canvas.set(x, y, if y > h - 3 { LIGHTGRAY } else { WHITE });
        }
    }
    for _ in 0..rng.between(1, 4) {
        let (x, y) = (rng.below(w), rng.below(h / 2));
        canvas.glyph(x, y, 'v', DARKGRAY);
    }
}

/// High fantasy: a castle on the horizon, towers and pointed roofs.
fn castle(canvas: &mut Canvas, rng: &mut Rng, horizon: i32, light: Light) {
    let w = canvas.w;
    let tall = (horizon / 2).max(3);
    let wide = (w / 6).clamp(6, 20);
    let x0 = if rng.chance(50) { rng.between(1, w / 3) } else { rng.between(w / 2, (w - wide - 1).max(w / 2)) };
    let (body, roof) = (DARKGRAY, if rng.chance(50) { BLUE } else { RED });
    let wall_top = horizon - tall / 2;
    canvas.rect(x0, wall_top, x0 + wide, horizon, body);
    for x in (x0..x0 + wide).step_by(2) {
        canvas.set(x, wall_top - 1, body);
    }
    for tower in [x0, x0 + wide / 2 - 1, x0 + wide - 2] {
        let top = horizon - tall - rng.below(3);
        canvas.rect(tower, top, tower + 2, horizon, body);
        canvas.peak(tower, top - 3, top - 1, 1, roof);
        canvas.peak(tower + 1, top - 3, top - 1, 1, roof);
        if light == Light::Night {
            canvas.light(tower, top + 2, YELLOW);
        }
    }
    canvas.rect(x0 + wide / 2 - 1, horizon - 2, x0 + wide / 2 + 1, horizon, BLACK);
}

/// Fog: the far land and the sky fade to grey.
fn fog(canvas: &mut Canvas, horizon: i32, light: Light) {
    let grey = if light == Light::Night { DARKGRAY } else { LIGHTGRAY };
    for y in 0..canvas.h {
        let t = if y < horizon { 9 } else { (9 - (y - horizon) * 9 / ((canvas.h - horizon) / 2).max(1)).max(0) };
        for x in 0..canvas.w {
            let c = Canvas::mix(x, y, canvas.get(x, y), grey, t);
            canvas.set(x, y, c);
        }
    }
}

/// What falls or blows: rain, snow, hail, sleet, dust, and a storm's lightning.
fn weather(canvas: &mut Canvas, rng: &mut Rng, horizon: i32, light: Light, sky: Sky) {
    let (w, h) = (canvas.w, canvas.h);
    let night = light == Light::Night;
    let (every, marks): (i32, &[(char, u8)]) = match sky {
        Sky::Rain => (10, &[('/', LIGHTBLUE), ('/', LIGHTCYAN)]),
        Sky::Storm => (5, &[('/', LIGHTBLUE), ('/', LIGHTGRAY)]),
        Sky::Snow => (9, &[('*', WHITE), ('∙', WHITE), ('.', LIGHTGRAY)]),
        Sky::Blizzard => (3, &[('*', WHITE), ('∙', WHITE), ('░', WHITE)]),
        Sky::Hail => (10, &[('o', WHITE), ('∙', LIGHTGRAY)]),
        Sky::Sleet => (8, &[('/', LIGHTGRAY), ('*', WHITE)]),
        Sky::Dust => (5, &[('∙', YELLOW), ('.', BROWN), ('░', BROWN)]),
        _ => (0, &[]),
    };
    if every > 0 {
        for _ in 0..(w * h / 2 / every).max(1) {
            let (x, y) = (rng.below(w), rng.below(h));
            let (c, fg) = marks[rng.below(marks.len() as i32) as usize];
            canvas.glyph(x, y, c, if night && fg == WHITE { LIGHTGRAY } else { fg });
        }
    }
    // Snow lies on the land, not on the water or the stone.
    if matches!(sky, Sky::Snow | Sky::Blizzard) {
        let (bright, dull) = if night { (LIGHTGRAY, DARKGRAY) } else { (WHITE, LIGHTGRAY) };
        for y in 0..h {
            for x in 0..w {
                if canvas.lit[(y * w + x) as usize] {
                    continue;
                }
                let c = match canvas.get(x, y) {
                    LIGHTGREEN | YELLOW | LIGHTGRAY | WHITE | LIGHTCYAN => bright,
                    GREEN | BROWN | CYAN => Canvas::mix(x, y, dull, bright, 6),
                    c => c,
                };
                canvas.set(x, y, c);
            }
        }
    }
    if sky == Sky::Storm {
        let mut x = rng.between(w / 5, w * 4 / 5);
        for y in 0..horizon {
            canvas.light(x, y, if y % 3 == 0 { WHITE } else { YELLOW });
            x += rng.between(-1, 1);
        }
    }
}

// ---- Under water ----

/// Under open water: light from above, the sea floor, weed and bubbles.
fn underwater(canvas: &mut Canvas, rng: &mut Rng) {
    let (w, h) = (canvas.w, canvas.h);
    canvas.gradient(0, h, &[CYAN, BLUE, BLUE, BLACK]);
    let floor = ridge(rng, w, 2, 6);
    for x in 0..w {
        for y in h - 3 - floor[x as usize]..h {
            canvas.set(x, y, Canvas::mix(x, y, BROWN, YELLOW, 4));
        }
    }
    for _ in 0..(w / 6).max(1) {
        let x = rng.below(w);
        let tall = rng.between(h / 4, h / 2);
        for i in 0..tall {
            canvas.set(x + (i / 2) % 2, h - 3 - i, if i % 2 == 0 { GREEN } else { LIGHTGREEN });
        }
    }
    for _ in 0..(w * h / 40).max(1) {
        let (x, y) = (rng.below(w), rng.below(h * 3 / 4));
        canvas.glyph(x, y, if rng.chance(60) { 'o' } else { '°' }, LIGHTCYAN);
    }
}

// ---- Indoors ----

/// What an indoor room is made of.
struct Material {
    wall: u8,
    /// The joints: mortar, planks' edges, seams.
    joint: u8,
    side: u8,
    floor: u8,
    floor_joint: u8,
    ceiling: u8,
    /// How the back wall is laid: bricks, planks, panels.
    laid: Laid,
}

#[derive(Clone, Copy, PartialEq)]
enum Laid {
    Bricks,
    Planks,
    Panels,
    Runes,
}

fn indoors(canvas: &mut Canvas, rng: &mut Rng, terrain: &str, high: bool) {
    match terrain {
        "cave" | "cavelakesurface" | "caveseaport" => return cave(canvas, rng, terrain, high),
        "gap" => return gap(canvas, rng),
        _ => {}
    }
    let material = match terrain {
        "wooden" => Material { wall: BROWN, joint: RED, side: RED, floor: BROWN, floor_joint: BLACK, ceiling: BLACK, laid: Laid::Planks },
        "metal" => Material { wall: LIGHTGRAY, joint: DARKGRAY, side: DARKGRAY, floor: DARKGRAY, floor_joint: BLACK, ceiling: BLACK, laid: Laid::Panels },
        "magic" => Material { wall: MAGENTA, joint: LIGHTMAGENTA, side: BLUE, floor: BLUE, floor_joint: MAGENTA, ceiling: BLACK, laid: Laid::Runes },
        "in_underwater" => Material { wall: BLUE, joint: CYAN, side: BLUE, floor: CYAN, floor_joint: BLUE, ceiling: BLACK, laid: Laid::Bricks },
        // Stone, and the sea ports' halls.
        _ => Material { wall: DARKGRAY, joint: LIGHTGRAY, side: DARKGRAY, floor: LIGHTGRAY, floor_joint: DARKGRAY, ceiling: BLACK, laid: Laid::Bricks },
    };
    let (w, h) = (canvas.w, canvas.h);
    let (bx0, bx1) = (w / 5, w - w / 5);
    let (by0, by1) = (h / 6, h * 7 / 10);
    room(canvas, &material, (bx0, by0, bx1, by1));

    // Shutters on a wooden wall (closed: no sky indoors), a doorway on the rest.
    let door_x = rng.between(bx0 + 2, (bx1 - 6).max(bx0 + 2));
    match terrain {
        "wooden" if rng.chance(70) => shutters(canvas, door_x, by0 + 2, (by1 - by0) / 3),
        _ => doorway(canvas, door_x, by1, (by1 - by0) * 2 / 3),
    }
    match terrain {
        "innerseaport" => {
            // A channel of water across the floor, a jetty over it.
            for y in by1 + 1..h {
                for x in 0..w {
                    if (y - by1) * 3 > h - by1 && (y - by1) * 3 < (h - by1) * 2 {
                        canvas.set(x, y, Canvas::mix(x, y, BLUE, LIGHTBLUE, 4));
                    }
                }
            }
            canvas.rect(w / 2 - 2, by1 + 1, w / 2 + 2, h, BROWN);
        }
        "in_underwater" => {
            for _ in 0..(w * h / 40).max(1) {
                let (x, y) = (rng.below(w), rng.below(h));
                canvas.glyph(x, y, if rng.chance(60) { 'o' } else { '°' }, LIGHTCYAN);
            }
        }
        "magic" => {
            // A glowing circle on the floor.
            let (cx, cy) = (w / 2, by1 + (h - by1) / 2);
            let (rx, ry) = ((w / 5).max(3), ((h - by1) / 3).max(1));
            for a in 0..64 {
                let t = f64::from(a) * std::f64::consts::TAU / 64.0;
                let x = cx + (f64::from(rx) * t.cos()).round() as i32;
                let y = cy + (f64::from(ry) * t.sin()).round() as i32;
                canvas.light(x, y, LIGHTCYAN);
            }
        }
        _ => {}
    }
    if high {
        banners(canvas, rng, (bx0, by0, bx1, by1), door_x);
        brazier(canvas, bx0 / 2 + 1, h - 3);
        brazier(canvas, w - bx0 / 2 - 2, h - 3);
    } else {
        torch(canvas, bx0 / 2, by0 + (by1 - by0) / 3);
        torch(canvas, w - bx0 / 2 - 1, by0 + (by1 - by0) / 3);
    }
}

/// A room in one-point perspective: the back wall, the side walls
/// narrowing toward it, the floor and the ceiling.
fn room(canvas: &mut Canvas, m: &Material, (bx0, by0, bx1, by1): (i32, i32, i32, i32)) {
    let (w, h) = (canvas.w, canvas.h);
    for x in 0..w {
        // Where the walls meet the ceiling and the floor in this column.
        let (top, bottom, side) = if x < bx0 {
            let t = x * 64 / bx0.max(1);
            (by0 * t / 64, h - (h - by1) * t / 64, true)
        } else if x >= bx1 {
            let t = (w - 1 - x) * 64 / (w - 1 - bx1).max(1);
            (by0 * t / 64, h - (h - by1) * t / 64, true)
        } else {
            (by0, by1, false)
        };
        for y in 0..h {
            let c = if y < top {
                Canvas::mix(x, y, m.ceiling, m.side, 3)
            } else if y >= bottom {
                // Flagstones or boards, wider nearer.
                let row = (y - by1).max(0);
                if (row % 3 == 0) || (x + row) % 6 == 0 {
                    m.floor_joint
                } else {
                    m.floor
                }
            } else if side {
                Canvas::mix(x, y, m.side, BLACK, 5)
            } else {
                match m.laid {
                    Laid::Bricks if (y - by0) % 3 == 2 || (x + if ((y - by0) / 3) % 2 == 0 { 0 } else { 2 }) % 4 == 0 => m.joint,
                    Laid::Planks if (x - bx0) % 3 == 0 => m.joint,
                    Laid::Panels if (x - bx0) % 6 == 0 || (y - by0) % 5 == 0 => m.joint,
                    _ => m.wall,
                }
            };
            canvas.set(x, y, c);
        }
    }
    if m.laid == Laid::Runes {
        for (i, x) in (bx0 + 2..bx1 - 1).step_by(4).enumerate() {
            let c = ['☼', '♦', '*', '+'][i % 4];
            canvas.glyph(x, by0 + 2 + (i as i32 % 3) * 2, c, if i % 2 == 0 { LIGHTCYAN } else { YELLOW });
        }
    }
    if m.laid == Laid::Panels {
        for x in (bx0 + 3..bx1).step_by(6) {
            canvas.glyph(x, by0 + 2, '∙', DARKGRAY);
        }
    }
}

/// An arched doorway into darkness, standing on the floor line.
fn doorway(canvas: &mut Canvas, x: i32, floor: i32, tall: i32) {
    let tall = tall.max(3);
    canvas.rect(x, floor - tall + 2, x + 4, floor, BLACK);
    canvas.rect(x + 1, floor - tall + 1, x + 3, floor - tall + 2, BLACK);
}

/// A window on the back wall, its shutters closed: indoors shows no sky,
/// no sun, moon or weather.
fn shutters(canvas: &mut Canvas, x: i32, y: i32, tall: i32) {
    let tall = tall.max(2);
    canvas.rect(x - 1, y - 1, x + 6, y + tall + 1, BLACK);
    canvas.rect(x, y, x + 5, y + tall, RED);
    canvas.rect(x + 2, y, x + 3, y + tall, BLACK);
    for row in (y..y + tall).step_by(2) {
        canvas.rect(x, row, x + 2, row + 1, BROWN);
        canvas.rect(x + 3, row, x + 5, row + 1, BROWN);
    }
}

/// A torch in a sconce, and its glow on the wall.
fn torch(canvas: &mut Canvas, x: i32, y: i32) {
    for gy in y - 3..=y + 2 {
        for gx in x - 2..=x + 2 {
            let c = Canvas::mix(gx, gy, canvas.get(gx, gy), BROWN, 5);
            canvas.set(gx, gy, c);
        }
    }
    canvas.rect(x, y, x + 1, y + 3, BROWN);
    canvas.light(x, y - 1, YELLOW);
    canvas.light(x, y - 2, LIGHTRED);
}

/// A small fire on the ground: two logs crossed, a flame, its glow.
fn campfire(canvas: &mut Canvas, x: i32, foot: i32) {
    for gy in foot - 4..=foot {
        for gx in x - 3..=x + 3 {
            let c = Canvas::mix(gx, gy, canvas.get(gx, gy), BROWN, 4);
            canvas.set(gx, gy, c);
        }
    }
    canvas.rect(x - 2, foot, x + 3, foot + 1, BROWN);
    canvas.light(x - 1, foot - 1, LIGHTRED);
    canvas.light(x, foot - 1, YELLOW);
    canvas.light(x + 1, foot - 1, LIGHTRED);
    canvas.light(x, foot - 2, YELLOW);
}

/// High fantasy: a brazier on a stand, burning high.
fn brazier(canvas: &mut Canvas, x: i32, foot: i32) {
    canvas.rect(x, foot - 2, x + 1, foot + 1, DARKGRAY);
    canvas.rect(x - 1, foot - 3, x + 2, foot - 2, DARKGRAY);
    canvas.light(x - 1, foot - 4, LIGHTRED);
    canvas.light(x, foot - 4, YELLOW);
    canvas.light(x + 1, foot - 4, LIGHTRED);
    canvas.light(x, foot - 5, YELLOW);
    canvas.light(x, foot - 6, WHITE);
}

/// High fantasy: banners hanging on the back wall, either side of the door.
fn banners(canvas: &mut Canvas, rng: &mut Rng, (bx0, by0, bx1, by1): (i32, i32, i32, i32), door: i32) {
    let colour = if rng.chance(50) { RED } else { BLUE };
    let long = ((by1 - by0) * 2 / 3).max(3);
    for x in [bx0 + 1, bx1 - 4] {
        if (x - door).abs() < 5 {
            continue;
        }
        canvas.rect(x, by0, x + 3, by0 + long, colour);
        canvas.set(x + 1, by0 + long, colour);
        canvas.glyph(x + 1, by0 + long / 2, '♦', YELLOW);
        canvas.rect(x - 1, by0, x + 4, by0 + 1, YELLOW);
    }
}

/// A cave: rough rock around a dark space, teeth from the roof and the
/// floor; a lake or a landing in the cave's water.
fn cave(canvas: &mut Canvas, rng: &mut Rng, terrain: &str, high: bool) {
    let (w, h) = (canvas.w, canvas.h);
    canvas.gradient(0, h, &[BLACK, BLACK, DARKGRAY]);
    let roof = ridge(rng, w, (h / 4).max(1), 4);
    let floor = ridge(rng, w, (h / 5).max(1), 5);
    for x in 0..w {
        // The walls close in at the edges.
        let edge = ((w / 2 - (x - w / 2).abs()) * 8 / w.max(1)).min(3);
        for y in 0..roof[x as usize] + 3 - edge {
            canvas.set(x, y, Canvas::mix(x, y, DARKGRAY, BROWN, 3));
        }
        for y in h - floor[x as usize] - 3 + edge..h {
            canvas.set(x, y, Canvas::mix(x, y, DARKGRAY, BROWN, 5));
        }
    }
    for _ in 0..(w / 5).max(2) {
        let x = rng.below(w);
        let long = rng.between(2, (h / 3).max(2));
        let base = roof[x as usize] + 2;
        for i in 0..long {
            canvas.set(x, base + i, DARKGRAY);
            if i < long / 2 {
                canvas.set(x + 1, base + i, DARKGRAY);
            }
        }
    }
    for _ in 0..(w / 8).max(1) {
        let x = rng.below(w);
        canvas.peak(x, h - floor[x as usize] - 3 - rng.between(2, (h / 4).max(2)), h - floor[x as usize] - 2, 1, DARKGRAY);
    }
    if terrain != "cave" {
        let surface = h * 3 / 5;
        for y in surface..h {
            for x in 0..w {
                canvas.set(x, y, Canvas::mix(x, y, BLUE, BLACK, (y - surface) * 10 / (h - surface).max(1)));
            }
        }
        for _ in 0..(w / 3).max(1) {
            let (x, y) = (rng.below(w), rng.between(surface, h - 1));
            canvas.glyph(x, y, '~', LIGHTBLUE);
        }
        if terrain == "caveseaport" {
            canvas.rect(0, surface - 1, w / 3, surface + 1, BROWN);
            ship(canvas, rng, w * 2 / 3, surface);
        }
    }
    for _ in 0..(w * h / 60).max(1) {
        let (x, y) = (rng.below(w), rng.below(h));
        if canvas.get(x, y) == DARKGRAY {
            canvas.glyph(x, y, '∙', LIGHTGRAY);
        }
    }
    let lamp = rng.between(w / 4, w * 3 / 4);
    let ground = h - floor[lamp as usize] - 3;
    if high {
        brazier(canvas, lamp, ground);
    } else if terrain == "cave" {
        campfire(canvas, lamp, ground);
    }
}

/// A gap: a chasm between two edges, a rope bridge over the dark.
fn gap(canvas: &mut Canvas, rng: &mut Rng) {
    let (w, h) = (canvas.w, canvas.h);
    canvas.gradient(0, h, &[BLACK, BLACK, BLACK]);
    let (left, right) = (rng.between(w / 6, w / 3), rng.between(w * 2 / 3, w * 5 / 6));
    let (ly, ry) = (rng.between(h / 3, h / 2), rng.between(h / 3, h / 2));
    for x in 0..w {
        let top = if x < left {
            ly + (left - x) / 4
        } else if x > right {
            ry + (x - right) / 4
        } else {
            h
        };
        for y in top..h {
            canvas.set(x, y, Canvas::mix(x, y, DARKGRAY, BROWN, 4));
        }
    }
    for x in left..=right {
        let y = ly + (ry - ly) * (x - left) / (right - left).max(1) + ((x - left) * (right - x)) / (w * 2).max(1);
        canvas.set(x, y, BROWN);
        if (x - left) % 3 == 0 {
            canvas.glyph(x, y - 1, '|', BROWN);
        }
    }
    for _ in 0..(w / 4).max(1) {
        let (x, y) = (rng.between(left + 1, right - 1), rng.between(h * 2 / 3, h - 1));
        canvas.glyph(x, y, '∙', DARKGRAY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TERRAINS: [&str; 25] = [
        "city", "woods", "rocky", "plains", "underwater", "air", "watersurface", "jungle", "swamp", "desert", "hills", "mountains", "spaceport", "seaport", "stone", "wooden", "cave", "magic", "in_underwater", "gap", "cavelakesurface", "metal", "innerseaport", "caveseaport", "somewhere new",
    ];
    const WEATHERS: [&str; 14] = ["clear", "cloudy", "windy", "rain", "thunderstorm", "snow", "hail", "heat", "sleet", "blizzard", "dust", "drought", "cold", "fog"];

    fn scene(terrain: &str, time: Option<&str>, weather: Option<&str>) -> Scene {
        Scene {
            world: "standard".into(),
            room: "Midgaard#3001".into(),
            name: "The Temple".into(),
            zone: "Midgaard".into(),
            terrain: terrain.into(),
            weather: weather.map(Into::into),
            time: time.map(Into::into),
            ..Scene::default()
        }
    }

    fn glyphs(canvas: &Canvas) -> Vec<char> {
        canvas.glyphs.iter().flatten().map(|(c, _)| *c).collect()
    }

    #[test]
    fn every_terrain_time_and_weather_paints_at_every_size() {
        for terrain in TERRAINS {
            for time in [None, Some("dawn"), Some("day"), Some("dusk"), Some("night")] {
                for weather in WEATHERS.iter().map(|w| Some(*w)).chain([None]) {
                    for style in [Style::Fantasy, Style::High] {
                        for (columns, rows) in [(1, 1), (8, 4), (64, 12), (160, 45), (999, 999)] {
                            let art = paint(&scene(terrain, time, weather), style, columns, rows, 0);
                            assert_eq!(art.columns, columns.clamp(LEAST_COLUMNS, MOST_COLUMNS));
                            assert_eq!(art.rows, rows.clamp(LEAST_ROWS, MOST_ROWS));
                            assert_eq!(art.lines.len(), art.rows);
                            for line in &art.lines {
                                assert_eq!(line.iter().map(|s| s.text.chars().count()).sum::<usize>(), art.columns, "{terrain}");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn every_character_is_one_the_font_has() {
        let font: Vec<char> = crate::ansi_art::CP437.chars().collect();
        for terrain in TERRAINS {
            for weather in WEATHERS {
                for style in [Style::Fantasy, Style::High] {
                    let art = paint(&scene(terrain, Some("night"), Some(weather)), style, 64, 12, 0);
                    for c in art.lines.iter().flatten().flat_map(|s| s.text.chars()) {
                        assert!(font.contains(&c), "{c:?} in {terrain}, {weather}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_room_looks_the_same_each_visit_and_another_room_differs() {
        let here = scene("woods", Some("day"), None);
        let there = Scene { room: "Midgaard#3002".into(), ..here.clone() };
        assert_eq!(paint(&here, Style::Fantasy, 64, 12, 0), paint(&here, Style::Fantasy, 64, 12, 0));
        assert_ne!(paint(&here, Style::Fantasy, 64, 12, 0), paint(&there, Style::Fantasy, 64, 12, 0));
        // Asking again gives a new look.
        assert_ne!(paint(&here, Style::Fantasy, 64, 12, 0), paint(&here, Style::Fantasy, 64, 12, 1));
    }

    #[test]
    fn the_sky_follows_the_time_of_day() {
        let day = draw(&scene("plains", Some("day"), None), Style::Fantasy, 64, 12, 0);
        assert!(matches!(day.get(0, 0), BLUE | LIGHTBLUE));
        let night = draw(&scene("plains", Some("night"), None), Style::Fantasy, 64, 12, 0);
        let stars = glyphs(&night).iter().filter(|c| matches!(c, '*' | '+' | '∙' | '.')).count();
        assert!(stars > 5, "{stars} stars");
        assert!(night.pixels.contains(&WHITE), "the moon");
        assert!(!day.glyphs.iter().flatten().any(|(c, _)| *c == '+'));
    }

    /// The middle of the pixels of a color in the sky: where the sun or the moon is.
    fn middle_of(canvas: &Canvas, color: u8, horizon: i32) -> (i32, i32) {
        let found: Vec<(i32, i32)> = (0..horizon).flat_map(|y| (0..canvas.w).map(move |x| (x, y))).filter(|&(x, y)| canvas.get(x, y) == color).collect();
        assert!(!found.is_empty(), "nothing of color {color} in the sky");
        let n = found.len() as i32;
        (found.iter().map(|p| p.0).sum::<i32>() / n, found.iter().map(|p| p.1).sum::<i32>() / n)
    }

    #[test]
    fn the_sun_and_the_moon_cross_by_the_clock_alone() {
        let horizon = 24 * 45 / 100;
        for (time, color) in [("day", YELLOW), ("night", WHITE)] {
            let at = |arc: u16, room: &str| {
                let scene = Scene { arc: Some(arc), room: room.into(), ..scene("plains", Some(time), None) };
                middle_of(&draw(&scene, Style::Fantasy, 64, 12, 0), color, horizon)
            };
            let (rising, noon, setting) = (at(100, "a"), at(500, "a"), at(900, "a"));
            assert!(rising.0 < noon.0 && noon.0 < setting.0, "{time}: left to right, {rising:?} {noon:?} {setting:?}");
            assert!(noon.1 < rising.1 && noon.1 < setting.1, "{time}: highest halfway");
            // Every room's sky agrees.
            assert_eq!(at(500, "b"), noon, "{time}");
        }
    }

    #[test]
    fn the_weather_falls_only_where_there_is_a_sky() {
        let snow = draw(&scene("plains", Some("day"), Some("snow")), Style::Fantasy, 64, 12, 0);
        assert!(glyphs(&snow).contains(&'*'));
        let rain = draw(&scene("plains", Some("day"), Some("rain")), Style::Fantasy, 64, 12, 0);
        assert!(glyphs(&rain).contains(&'/'));
        let inside = draw(&scene("stone", Some("day"), Some("rain")), Style::Fantasy, 64, 12, 0);
        assert!(!glyphs(&inside).contains(&'/'));
        let below = draw(&scene("underwater", Some("day"), Some("snow")), Style::Fantasy, 64, 12, 0);
        assert!(!glyphs(&below).contains(&'*'));
    }

    #[test]
    fn indoors_and_underwater_show_nothing_of_the_sky() {
        for terrain in ["stone", "wooden", "cave", "magic", "in_underwater", "gap", "metal", "innerseaport", "underwater"] {
            for room in ["room#1", "room#2", "room#3", "room#4"] {
                let at = |time: &str, weather: &str| {
                    let s = Scene { world: "standard".into(), room: room.into(), terrain: terrain.into(), time: Some(time.into()), weather: Some(weather.into()), ..Scene::default() };
                    paint(&s, Style::Fantasy, 64, 12, 0)
                };
                let day = at("day", "clear");
                assert_eq!(day, at("night", "clear"), "{terrain} {room}: the time of day shows");
                assert_eq!(day, at("day", "thunderstorm"), "{terrain} {room}: the weather shows");
            }
        }
    }

    #[test]
    fn indoors_has_a_ceiling_and_a_light() {
        let hall = draw(&scene("stone", Some("day"), None), Style::Fantasy, 64, 12, 0);
        assert!(!matches!(hall.get(32, 0), BLUE | LIGHTBLUE | LIGHTCYAN));
        assert!(hall.pixels.contains(&YELLOW), "a torch");
    }

    #[test]
    fn high_fantasy_adds_its_motifs() {
        // A second moon and an aurora at night.
        let plain = draw(&scene("plains", Some("night"), None), Style::Fantasy, 64, 12, 0);
        let high = draw(&scene("plains", Some("night"), None), Style::High, 64, 12, 0);
        assert!(!plain.pixels.contains(&LIGHTRED));
        assert!(high.pixels.contains(&LIGHTRED));
        assert!(high.pixels.contains(&LIGHTGREEN));
        // Banners indoors.
        let hall = draw(&scene("stone", Some("day"), None), Style::High, 64, 12, 0);
        assert!(glyphs(&hall).contains(&'♦'));
    }

    #[test]
    fn lights_stay_lit_at_night() {
        let town = draw(&scene("city", Some("night"), None), Style::Fantasy, 64, 12, 0);
        assert!(town.pixels.contains(&YELLOW), "lit windows");
    }
}

/// Prints pictures to the terminal in ANSI, to see the painter at work:
/// `cargo test --manifest-path src-tauri/Cargo.toml painter::show -- --ignored --nocapture`.
#[cfg(test)]
mod show {
    use super::*;

    fn print(art: &Art) {
        let code = |c: Option<Color>, base: u8| match c {
            Some(Color::Index { index }) if index < 8 => format!("{}", base + index),
            Some(Color::Index { index }) => format!("{}", base + 60 + index - 8),
            _ => "0".into(),
        };
        for line in &art.lines {
            let mut out = String::new();
            for span in line {
                out.push_str(&format!("\x1b[{};{}m{}", code(span.style.fg, 30), code(span.style.bg, 40), span.text));
            }
            println!("{out}\x1b[0m");
        }
    }

    /// The VGA palette, as the view shows the 16 colors.
    const VGA: [[u8; 3]; 16] = [
        [0, 0, 0], [170, 0, 0], [0, 170, 0], [170, 85, 0], [0, 0, 170], [170, 0, 170], [0, 170, 170], [170, 170, 170],
        [85, 85, 85], [255, 85, 85], [85, 255, 85], [255, 255, 85], [85, 85, 255], [255, 85, 255], [85, 255, 255], [255, 255, 255],
    ];

    /// Each picture as pixels 8 wide and tall, a character as a dot of
    /// its color, one under another, written as a PPM to `OUT`.
    fn ppm(canvases: &[Canvas]) -> Vec<u8> {
        let w = canvases.iter().map(|c| c.w).max().unwrap_or(0) * 8;
        let h: i32 = canvases.iter().map(|c| c.h * 8 + 8).sum();
        let mut rgb = vec![40u8; (w * h * 3) as usize];
        let mut top = 0;
        for canvas in canvases {
            for y in 0..canvas.h * 8 {
                for x in 0..canvas.w * 8 {
                    let (px, py) = (x / 8, y / 8);
                    let mut c = canvas.get(px, py);
                    if let Some((_, fg)) = canvas.glyphs[((py / 2) * canvas.w + px) as usize] {
                        let (cx, cy) = (x % 8, y % 16);
                        c = canvas.get(px, (py / 2) * 2);
                        if (2..6).contains(&cx) && (5..11).contains(&cy) {
                            c = fg;
                        }
                    }
                    let i = (((top + y) * w + x) * 3) as usize;
                    rgb[i..i + 3].copy_from_slice(&VGA[c as usize]);
                }
            }
            top += canvas.h * 8 + 8;
        }
        let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
        out.extend(rgb);
        out
    }

    #[test]
    #[ignore]
    fn show() {
        let terrain = std::env::var("TERRAIN").unwrap_or_default();
        let terrains: Vec<&str> = if terrain.is_empty() { vec!["plains", "woods", "mountains", "city", "watersurface", "stone", "wooden", "cave", "magic"] } else { terrain.split(',').collect() };
        let time = std::env::var("TIME").ok();
        let weather = std::env::var("WEATHER").ok();
        let style = if std::env::var("HIGH").is_ok() { Style::High } else { Style::Fantasy };
        let mut canvases = Vec::new();
        for terrain in terrains {
            let scene = Scene { world: "standard".into(), room: "room#1".into(), terrain: terrain.into(), time: time.clone(), weather: weather.clone(), ..Scene::default() };
            print(&paint(&scene, style, 64, 12, 0));
            canvases.push(draw(&scene, style, 64, 12, 0));
        }
        if let Ok(out) = std::env::var("OUT") {
            std::fs::write(out, ppm(&canvases)).unwrap();
        }
    }
}
