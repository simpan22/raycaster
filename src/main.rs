extern crate sdl2;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use nalgebra::{UnitVector3, Vector3};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::Sdl;

#[derive(Debug, Copy, Clone, PartialEq)]
enum Material {
    R,
    G,
    B,
    Empty,
}
use Material::*;

static W: u32 = 600;
static H: u32 = 600;

struct Camera {
    pos: Vector3<f32>,
    normal: Vector3<f32>,
    width: u32,
    height: u32,
}

struct Wall {
    pos: Vector3<f32>,
    s1: Vector3<f32>,
    s2: Vector3<f32>,
    color: Color,
}

fn read_map(file: &Path) -> Vec<Vec<Material>> {
    let file = File::open(file).unwrap();
    let mut rows = vec![];
    let lines = BufReader::new(file).lines();
    for line in lines {
        let mut entries = vec![];
        let line = line.unwrap();
        for char in line.chars() {
            entries.push(match char {
                'E' => Empty,
                'R' => R,
                'G' => G,
                'B' => B,
                _ => Empty,
            })
        }
        rows.push(entries);
    }
    rows
}

fn calculate_pixel_color(x: u32, y: u32, cam: &Camera, walls: &[Wall]) -> Result<Color, String> {
    let y = cam.height - y;

    // TODO: Normlize with respect to X only
    let x = (x as f32 / cam.width as f32) - 0.5;
    let y = (y as f32 / cam.height as f32) - 0.5;

    let center_pixel = cam.pos + cam.normal * 1.0; // TODO: 1.0 is focal_length
    let pixel_pos =
        center_pixel + y * Vector3::z() + x * (cam.normal.cross(&Vector3::z())).normalize();

    let mut ray_dir = pixel_pos - cam.pos;
    ray_dir.normalize_mut();

    let mut closest_color = None;
    let mut closest_distance = f32::INFINITY;

    for wall in walls {
        let plane_normal = wall.s1.cross(&wall.s2).normalize();
        let angle = ray_dir.dot(&plane_normal);

        if angle < 0.0 {
            let a = (wall.pos - cam.pos).dot(&plane_normal) / angle;
            let p = cam.pos + a * ray_dir;

            let diff = p - wall.pos;

            let q1 = diff.dot(&wall.s1) / wall.s1.norm();
            let q2 = diff.dot(&wall.s2) / wall.s2.norm();

            if q1 > 0.0 && q1 < wall.s1.norm() && q2 > 0.0 && q2 < wall.s2.norm() && a > 0.0 {
                let distance = (p - pixel_pos).norm();
                if distance < closest_distance {
                    closest_color = Some(wall.color);
                    closest_distance = distance;
                }
            }
        }
    }

    if let Some(color) = closest_color {
        Ok(color)
    } else if ray_dir.z < 0.0 {
        Ok(Color::RGB(70, 50, 40))
    } else {
        Ok(Color::RGB(170, 170, 230))
    }
}

fn create_walls(map: &Vec<Vec<Material>>) -> Vec<Wall> {
    let mut walls = vec![];
    for (y, row) in map.iter().enumerate() {
        for (x, color) in row.iter().enumerate() {
            let color = match color {
                R => Some(Color::RED),
                G => Some(Color::GREEN),
                B => Some(Color::BLUE),
                Empty => None,
            };
            if let Some(color) = color {
                let center = Vector3::new(x as f32 * 1.0, y as f32 * 1.0, 1.0);
                walls.push(Wall {
                    pos: center + Vector3::new(-0.5, -0.5, -0.5),
                    s1: Vector3::new(1.0, 0.0, 0.0),
                    s2: Vector3::new(0.0, 0.0, 1.0),
                    color,
                });
                walls.push(Wall {
                    pos: center + Vector3::new(0.5, -0.5, -0.5),
                    s1: Vector3::new(0.0, 1.0, 0.0),
                    s2: Vector3::new(0.0, 0.0, 1.0),
                    color,
                });
                walls.push(Wall {
                    pos: center + Vector3::new(0.5, 0.5, -0.5),
                    s1: Vector3::new(-1.0, 0.0, 0.0),
                    s2: Vector3::new(0.0, 0.0, 1.0),
                    color,
                });
                walls.push(Wall {
                    pos: center + Vector3::new(-0.5, 0.5, -0.5),
                    s1: Vector3::new(0.0, -1.0, 0.0),
                    s2: Vector3::new(0.0, 0.0, 1.0),
                    color,
                });
            }
        }
    }
    walls
}

fn create_canvas(sdl_context: &Sdl) -> Result<Canvas<Window>, String> {
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("rust-sdl2 demo: Video", W, H)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    window.into_canvas().build().map_err(|e| e.to_string())
}

fn render(canvas: &mut Canvas<Window>, camera: &Camera, walls: &Vec<Wall>) -> Result<(), String> {
    canvas.set_draw_color(Color::RGB(255, 0, 0));
    canvas.clear();

    for y in 0..H {
        for x in 0..W {
            canvas.set_draw_color(calculate_pixel_color(x, y, &camera, &walls)?);
            canvas.draw_point((x as i32, y as i32))?;
        }
    }

    canvas.present();
    Ok(())
}

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let mut canvas = create_canvas(&sdl_context)?;

    canvas.set_draw_color(Color::WHITE);
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump()?;
    let mut camera = Camera {
        pos: Vector3::new(0.0, -1.0, 1.0),
        normal: Vector3::new(0.0, 1.0, 0.0),
        height: H,
        width: W,
    };

    let map = read_map(Path::new("map.map"));
    let walls = create_walls(&map);

    let left_rotation =
        nalgebra::UnitQuaternion::from_axis_angle(&UnitVector3::new_normalize(Vector3::z()), 0.1);
    let right_rotation =
        nalgebra::UnitQuaternion::from_axis_angle(&UnitVector3::new_normalize(Vector3::z()), -0.1);

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::Up => camera.pos += camera.normal * 0.2,
                    Keycode::Down => camera.pos -= camera.normal * 0.2,
                    Keycode::Left => camera.normal = left_rotation * camera.normal,
                    Keycode::Right => camera.normal = right_rotation * camera.normal,
                    _ => {}
                },
                _ => {}
            }
        }
        render(&mut canvas, &camera, &walls)?;
        // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 30));
        // The rest of the game loop goes here...
    }

    Ok(())
}
