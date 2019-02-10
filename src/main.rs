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

fn signed_distance(p: Vec3f) -> f32 {
    return p.magnitude() - SPHERE_RADIUS;
}

fn sphere_trace(orig: Vec3f, dir: Vec3f) -> bool {
    let mut pos: Vec3f = orig;
    for _i in 0..128 {
        let d = signed_distance(pos);
        if d < 0_f32 {
            return true;
        }
        pos = pos + dir * (d * 0.1_f32).max(0.01_f32);
    }
    return false;
}

fn clamp(val: f32) -> u8 {
    (255. * val.max(0.).min(1.)) as u8
}

fn main() -> Result<(), Box<Error>> {
    const WIDTH: usize = 640;
    const HEIGHT: usize = 480;
    const FOV: f32 = f32::consts::PI / 2.;
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
            if sphere_trace(
                Vec3f::new(0., 0., 3.),
                Vec3f::new(dir_x, dir_y, dir_z).normalize(),
            ) {
                // the camera is placed to (0,0,3) and it looks along the -z axis
                *buffer = Vec3f::new(1., 1., 1.);
            } else {
                *buffer = Vec3f::new(0.2, 0.7, 0.8); // background color
            }
        });

    // Save image
    let path = "step1.png";
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
