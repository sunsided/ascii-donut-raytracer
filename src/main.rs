use std::f32::consts::PI;
use std::io::{stdout, Write};
use std::thread::sleep;
use std::time::Duration;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Color, SetForegroundColor, ResetColor},
    terminal::{size as term_size, EnterAlternateScreen, LeaveAlternateScreen},
};

const COLOR_SCALE: f32 = 1.5; // Adjusted scaling factor for color intensity

#[derive(Copy, Clone, Debug, Default)]
struct Vec2 {
    x: f32,
    y: f32,
}
impl Vec2 {
    fn new(x: f32, y: f32) -> Self { Self { x, y } }
}

#[derive(Copy, Clone, Debug, Default)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self { Self { x, y, z } }
    fn add(self, o: Vec3) -> Self { Self::new(self.x + o.x, self.y + o.y, self.z + o.z) }
    fn sub(self, o: Vec3) -> Self { Self::new(self.x - o.x, self.y - o.y, self.z - o.z) }
    fn mul(self, s: f32) -> Self { Self::new(self.x * s, self.y * s, self.z * s) }
    fn dot(self, o: Vec3) -> f32 { self.x * o.x + self.y * o.y + self.z * o.z }
    fn len(self) -> f32 { self.dot(self).sqrt() }
    fn norm(self) -> Self {
        let l = self.len();
        if l > 0.0 { self.mul(1.0 / l) } else { self }
    }
}

fn rot_z(v: Vec3, angle_rad: f32) -> Vec3 {
    let (s, c) = angle_rad.sin_cos();
    // rotate the Y–Z plane like original AZ quaternion-from-euler(z)
    // (their camera used X forward; we keep X as-is, rotate (y,z))
    Vec3::new(
        v.x,
        c * v.y - s * v.z,
        s * v.y + c * v.z,
    )
}

fn sd_torus(p: Vec3, t: Vec2, tdir: Vec3) -> f32 {
    // project p onto plane orthogonal to tdir,
    // then pull it onto the major radius circle (length = t.x),
    // distance to that circle minus tube radius t.y
    let p_proj = p.sub(tdir.mul(p.dot(tdir)));
    let p_proj = p_proj.norm().mul(t.x);
    p_proj.sub(p).len() - t.y
}

fn torus_normal(p: Vec3, t: Vec2, tdir: Vec3) -> Vec3 {
    let eps = 0.005;
    let dx = sd_torus(Vec3::new(p.x + eps, p.y, p.z), t, tdir)
        - sd_torus(Vec3::new(p.x - eps, p.y, p.z), t, tdir);
    let dy = sd_torus(Vec3::new(p.x, p.y + eps, p.z), t, tdir)
        - sd_torus(Vec3::new(p.x, p.y - eps, p.z), t, tdir);
    let dz = sd_torus(Vec3::new(p.x, p.y, p.z + eps), t, tdir)
        - sd_torus(Vec3::new(p.x, p.y, p.z - eps), t, tdir);
    Vec3::new(dx, dy, dz).norm()
}

// Helper to linearly interpolate between two u8 values
fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round().clamp(0.0, 255.0) as u8
}

// Helper to linearly interpolate between two Color::Rgb
fn lerp_color(a: &Color, b: &Color, t: f32) -> Color {
    match (a, b) {
        (Color::Rgb { r: r1, g: g1, b: b1 }, Color::Rgb { r: r2, g: g2, b: b2 }) => {
            Color::Rgb {
                r: lerp(*r1, *r2, t),
                g: lerp(*g1, *g2, t),
                b: lerp(*b1, *b2, t),
            }
        }
        _ => *a, // fallback, should not happen
    }
}

fn get_color_from_intensity(intensity: f32) -> Color {
    // Gradient stops: (threshold, Color)
    // Blue to orange temperature gradient
    const GRADIENT: &[(f32, Color)] = &[
        (0.0,  Color::Rgb { r: 0,   g: 0,   b: 150 }),   // Deep blue
        (0.2,  Color::Rgb { r: 50,  g: 100, b: 255 }),   // Medium blue
        (0.4,  Color::Rgb { r: 100, g: 200, b: 255 }),   // Cyan
        (0.6,  Color::Rgb { r: 200, g: 255, b: 155 }),   // Yellowish
        (0.8,  Color::Rgb { r: 255, g: 155, b: 0   }),   // Orange
        (1.0,  Color::Rgb { r: 255, g: 255, b: 100 }),   // White-orange
    ];
    
    let clamped = intensity.max(0.0).min(1.0);
    
    // Find the two stops between which clamped falls
    for i in 0..GRADIENT.len() - 1 {
        let (t0, c0) = GRADIENT[i];
        let (t1, c1) = GRADIENT[i + 1];
        if clamped >= t0 && clamped <= t1 {
            let t = (clamped - t0) / (t1 - t0);
            return lerp_color(&c0, &c1, t);
        }
    }
    
    // Fallback: if clamped == 1.0 exactly, return last color
    GRADIENT.last().unwrap().1
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // terminal setup
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, Hide)?;
    let (mut width, mut height) = term_size()?;
    if width < 20 { width = 80; }
    if height < 10 { height = 24; }

    // aspect and shading
    let aspect = width as f32 / height as f32;
    let pixel_aspect = 11.0f32 / 24.0; // non-square terminal pixels
    let gradient = b" .:-=+*#%@"; // Brighter character progression
    let grad_size = (gradient.len() as i32) - 1;
    let min_col = 1.0 / grad_size as f32;

    // scene parameters
    let moving = 20_000;                         // frames
    let light = Vec3::new(-1.0, -1.0, -1.0).norm();
    let ro = Vec3::new(-2.5, 0.0, 0.0);          // camera origin
    let in_rad = 0.3_f32;                        // tube radius
    let out_rad = 1.2_f32;                       // main radius
    let camp_pos_x = -2.0_f32;                   // like C++ variable name
    let torus = Vec2::new(out_rad, in_rad);

    let mut frame_buf = vec![b' '; (width as usize) * (height as usize)];
    let mut color_buf = vec![Color::Reset; (width as usize) * (height as usize)];

    for t in 0..moving {
        // rotate torus axis over time: start with (1,1,1) and rotate around Z
        let base_axis = Vec3::new(1.0, 1.0, 1.0).norm();
        // original used "degrees = t", convert to radians; slow it down a bit
        let angle = (t as f32) * 0.6_f32 * (PI / 180.0);
        let tdir = rot_z(base_axis, angle).norm();

        // clear frame buffer
        frame_buf.fill(b' ');
        color_buf.fill(Color::Reset);

        for j in 0..height {
            for i in 0..width {
                // uv in [-1, 1], correct aspect and pixel aspect
                let mut ux = (i as f32 / width as f32) * 2.0 - 1.0;
                let uy = (j as f32 / height as f32) * 2.0 - 1.0;
                ux *= aspect * pixel_aspect;

                // ray dir: X forward (like original: rd = normalize(1, uv.x, uv.y))
                let rd = Vec3::new(1.0, ux, uy).norm();

                // simple marching along rd up to a rough far bound
                let mut diff = 0.0_f32;
                let far = out_rad * 2.0 - camp_pos_x;
                let mut k = 0.0_f32;
                while k < far {
                    let p = ro.add(rd.mul(k));
                    let d = sd_torus(p, torus, tdir);
                    if d < in_rad {
                        let n = torus_normal(p, torus, tdir);
                        diff += n.dot(light).max(min_col);
                        break;
                    }
                    // step similar to tube radius; the C++ used fixed inRad steps
                    k += in_rad;
                }

                let mut ci = (diff * 20.0) as i32;
                if ci < 0 { ci = 0; }
                if ci > grad_size { ci = grad_size; }
                let px = gradient[ci as usize];
                
                // Calculate color based on lighting intensity with better blending
                let raw_intensity = diff / COLOR_SCALE; // More sensitive to lighting changes
                let intensity = raw_intensity.clamp(0.1, 1.0); // Ensure minimum brightness
                let color = get_color_from_intensity(intensity);

                let idx = (i as usize) + (j as usize) * (width as usize);
                frame_buf[idx] = px;
                color_buf[idx] = color;
            }
        }

        // draw
        execute!(out, MoveTo(0, 0))?; // Move to top-left, but don't clear screen
        for j in 0..height {
            execute!(out, MoveTo(0, j))?; // Move to start of each line
            for i in 0..width {
                let idx = (i as usize) + (j as usize) * (width as usize);
                let ch = frame_buf[idx] as char;
                let color = color_buf[idx];
                
                // Set color and write character
                execute!(out, SetForegroundColor(color))?;
                write!(out, "{}", ch)?;
            }
        }
        execute!(out, ResetColor)?;
        out.flush().unwrap();

        // small delay so it’s visible; adjust or remove as you like
        sleep(Duration::from_millis(16));
    }

    // restore terminal
    execute!(out, Show, LeaveAlternateScreen)?;
    Ok(())
}
