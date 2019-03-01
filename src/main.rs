use cgmath;
use cgmath::prelude::*;
use png::HasParameters;
use rayon::prelude::*;

type Vec3f = cgmath::Vector3<f32>;

use std::error::Error;
use std::f32;
use std::fs::File;
use std::io::BufWriter;

const SPHERE_RADIUS: f32 = 1.5;
const NOISE_AMPLITUDE: f32 = 0.2;

fn signed_distance(p: Vec3f) -> f32 {
    let hit = p.normalize() * SPHERE_RADIUS;
    let displacement: f32 = (16.*hit.x).sin() * (16.*hit.y).sin() * (16.*hit.z).sin() *NOISE_AMPLITUDE;
    p.magnitude() - (SPHERE_RADIUS + displacement)
}

fn sphere_trace(orig: Vec3f, dir: Vec3f) -> Option<Vec3f> {
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
                let light_dir: Vec3f = (Vec3f::new(10., 10., 10.) - hit).normalize(); // one light is placed to (10,10,10)
                let light_intensity: f32 = 0.4_f32.max(light_dir.dot(distance_field_normal(hit)));

                *buffer = Vec3f::new(1., 1., 1.) * light_intensity;
            } else {
                *buffer = Vec3f::new(0.2, 0.7, 0.8); // background color
            }
        });

    // Save image
    let path = "step_4.png";
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
