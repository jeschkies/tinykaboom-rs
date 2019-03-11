use cgmath;
use cgmath::prelude::*;
use png::HasParameters;
use rayon::prelude::*;

type Vec3f = cgmath::Vector3<f32>;

use std::error::Error;
use std::f32;
use std::fs::File;
use std::io::BufWriter;
use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;

const SPHERE_RADIUS: f32 = 1.5;
const NOISE_AMPLITUDE: f32 = 0.2;

fn lerp<V>(v0: V, v1: V, t: f32) -> V
where
    V: Add<V, Output = V> + Sub<V, Output = V> + Mul<f32, Output = V> + Clone,
{
    let v2 = v0.clone();
    v0 + (v1 - v2) * t.min(1.).max(0.)
}

// Many magic variables from https://github.com/ssloy/tinykaboom/commit/6ac4658d75cadaf095af7994572d79ceb395af9a.
fn hash(n: f32) -> f32 {
    let x = n.sin() * 43758.5453;
    x - x.floor()
}

fn noise(x: Vec3f) -> f32 {
    let p = Vec3f::new(x.x.floor(), x.y.floor(), x.z.floor());
    let mut f = Vec3f::new(x.x - p.x, x.y - p.y, x.z - p.z);
    f = f * (f.dot(Vec3f::new(3., 3., 3.) - f * 2.));
    let n = p.dot(Vec3f::new(1., 57., 113.));
    lerp(
        lerp(
            lerp(hash(n + 0.), hash(n + 1.), f.x),
            lerp(hash(n + 57.), hash(n + 58.), f.x),
            f.y,
        ),
        lerp(
            lerp(hash(n + 113.), hash(n + 114.), f.x),
            lerp(hash(n + 170.), hash(n + 171.), f.x),
            f.y,
        ),
        f.z,
    )
}

fn rotate(v: Vec3f) -> Vec3f {
    Vec3f::new(
        Vec3f::new(0.00, 0.80, 0.60).dot(v),
        Vec3f::new(-0.80, 0.36, -0.48).dot(v),
        Vec3f::new(-0.60, -0.48, 0.64).dot(v),
    )
}

fn fractal_brownian_motion(p: Vec3f) -> f32 {
    let mut p: Vec3f = rotate(p);
    let mut f: f32 = 0.;
    f += 0.5000 * noise(p);
    p = p * 2.32;
    f += 0.2500 * noise(p);
    p = p * 3.03;
    f += 0.1250 * noise(p);
    p = p * 2.61;
    f += 0.0625 * noise(p);
    f / 0.9375
}

fn palette_fire(d: f32) -> Vec3f {
    const YELLOW: Vec3f = Vec3f::new(1.7, 1.3, 1.0); // note that the color is "hot", i.e. has components >1
    const ORANGE: Vec3f = Vec3f::new(1.0, 0.6, 0.0);
    const RED: Vec3f = Vec3f::new(1.0, 0.0, 0.0);
    const DARKGREY: Vec3f = Vec3f::new(0.2, 0.2, 0.2);
    const GRAY: Vec3f = Vec3f::new(0.4, 0.4, 0.4);

    let x = d.min(1.).max(0.);
    if x < 0.25 {
        lerp(GRAY, DARKGREY, x * 4.)
    } else if x < 0.5 {
        lerp(DARKGREY, RED, x * 4. - 1.)
    } else if x < 0.75 {
        lerp(RED, ORANGE, x * 4. - 2.)
    } else {
        lerp(ORANGE, YELLOW, x * 4. - 3.)
    }
}

fn signed_distance(p: Vec3f) -> f32 {
    let displacement: f32 = -fractal_brownian_motion(p * 3.4) * NOISE_AMPLITUDE;
    p.magnitude() - (SPHERE_RADIUS + displacement)
}

fn sphere_trace(orig: Vec3f, dir: Vec3f) -> Option<Vec3f> {
    if (orig.dot(orig) - orig.dot(dir).powi(2)) > SPHERE_RADIUS.powi(2) {
        return None;
    } // early discard

    let mut pos: Vec3f = orig;
    for _i in 0..128 {
        let d = signed_distance(pos);
        if d < 0_f32 {
            return Some(pos);
        }
        pos += dir * (d * 0.1_f32).max(0.01_f32);
    }
    None
}

fn distance_field_normal(pos: Vec3f) -> Vec3f {
    const EPS: f32 = 0.1;
    let d = signed_distance(pos);
    let nx = signed_distance(pos + Vec3f::new(EPS, 0., 0.)) - d;
    let ny = signed_distance(pos + Vec3f::new(0., EPS, 0.)) - d;
    let nz = signed_distance(pos + Vec3f::new(0., 0., EPS)) - d;
    Vec3f::new(nx, ny, nz).normalize()
}

fn clamp(val: f32) -> u8 {
    (255. * val.max(0.).min(1.)) as u8
}

fn main() -> Result<(), Box<Error>> {
    const WIDTH: usize = 640;
    const HEIGHT: usize = 480;
    const FOV: f32 = f32::consts::PI / 3.;
    let mut framebuffer: Vec<Vec3f> = vec![Vec3f::new(0., 0., 0.); WIDTH * HEIGHT];

    // Render
    framebuffer
        .par_iter_mut()
        .enumerate()
        .for_each(|(index, buffer)| {
            let i = index % WIDTH;
            let j = (index - i) / WIDTH;

            let dir_x: f32 = (i as f32 + 0.5) - WIDTH as f32 / 2.;
            let dir_y: f32 = -(j as f32 + 0.5) + HEIGHT as f32 / 2.;
            let dir_z: f32 = -(HEIGHT as f32) / (2. * (FOV / 2.).tan());
            if let Some(hit) = sphere_trace(
                Vec3f::new(0., 0., 3.), // the camera is placed to (0,0,3) and it looks along the -z axis
                Vec3f::new(dir_x, dir_y, dir_z).normalize(),
            ) {
                let noise_level = (SPHERE_RADIUS - hit.magnitude()) / NOISE_AMPLITUDE;
                let light_dir: Vec3f = (Vec3f::new(10., 10., 10.) - hit).normalize(); // one light is placed to (10,10,10)
                let light_intensity: f32 = 0.4_f32.max(light_dir.dot(distance_field_normal(hit)));

                *buffer = palette_fire((-0.2 + noise_level) * 2.) * light_intensity;
            } else {
                *buffer = Vec3f::new(0.2, 0.7, 0.8); // background color
            }
        });

    // Save image
    let path = "step_7.png";
    let file = File::create(path)?;
    let w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, WIDTH as u32, HEIGHT as u32);
    encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;

    let mut data = Vec::with_capacity(WIDTH * HEIGHT);
    for v in framebuffer {
        data.push(clamp(v[0]));
        data.push(clamp(v[1]));
        data.push(clamp(v[2]));
        data.push(255);
    }
    writer.write_image_data(&data)?;

    Ok(())
}
