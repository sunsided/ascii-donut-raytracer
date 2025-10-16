use std::f32::consts::PI;
use std::io::{stdout, Write};
use std::thread::sleep;
use std::time::Duration;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    terminal::{size as term_size, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

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
    let gradient = b" .:!/r(l1Z4H9W8$@";
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

    for t in 0..moving {
        // rotate torus axis over time: start with (1,1,1) and rotate around Z
        let base_axis = Vec3::new(1.0, 1.0, 1.0).norm();
        // original used "degrees = t", convert to radians; slow it down a bit
        let angle = (t as f32) * 0.6_f32 * (PI / 180.0);
        let tdir = rot_z(base_axis, angle).norm();

        // clear frame buffer
        frame_buf.fill(b' ');

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

                frame_buf[(i as usize) + (j as usize) * (width as usize)] = px;
            }
        }

        // draw
        execute!(out, MoveTo(0, 0), Clear(ClearType::All))?;
        for j in 0..height {
            let start = (j as usize) * (width as usize);
            let end = start + (width as usize);
            out.write_all(&frame_buf[start..end]).unwrap();
            out.write_all(b"\r\n").unwrap();
        }
        out.flush().unwrap();

        // small delay so it’s visible; adjust or remove as you like
        sleep(Duration::from_millis(16));
    }

    // restore terminal
    execute!(out, Show, LeaveAlternateScreen)?;
    Ok(())
}
