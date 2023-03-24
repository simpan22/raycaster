extern crate sdl2;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use nalgebra::Vector2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::video::Window;
use sdl2::Sdl;

use image::io::Reader as ImageReader;

#[derive(Debug, Copy, Clone, PartialEq)]
enum Material {
    R,
    G,
    B,
    Empty,
}
use Material::*;

static W: u32 = 800;
static H: u32 = 800;

struct Texture {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl Texture {
    fn get_vline(&self, x: usize) -> &[u8] {
        // let mut vline = vec![];
        // for y in 0..self.height {
        //     let r = self.data[((y * self.width + (x as u32))*3) as usize];
        //     let g = self.data[((y * self.width + (x as u32))*3) as usize + 1];
        //     let b = self.data[((y * self.width + (x as u32))*3) as usize + 2];

        //     vline.push(Color::RGB(r, g, b));
        // }
        // vline
        let offset = 3 * self.height as usize * x;
        &self.data[offset..offset + (self.height as usize) * 3]
    }
}

struct Camera {
    pos: Vector2<f32>,
    normal: Vector2<f32>,
    width: u32,
    height: u32,
}

struct Wall {
    pos: Vector2<f32>,
    side: Vector2<f32>,
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

fn cross(v: &Vector2<f32>, w: &Vector2<f32>) -> f32 {
    v.x * w.y - v.y * w.x
}

fn calculate_vline(
    x: u32,
    cam: &Camera,
    walls: &[Wall],
    texture: &Texture,
) -> Result<Vec<Color>, String> {
    let x = (x as f32 / cam.width as f32) - 0.5;
    let center_pixel = cam.pos + cam.normal * 1.0;
    let camera_x_unit = Vector2::new(-cam.normal.y, cam.normal.x);
    let vline_pos = center_pixel + x * camera_x_unit;

    let ray_dir = (vline_pos - cam.pos).normalize();

    let mut vline = vec![Color::WHITE; H as usize];
    let mut closest_distance = f32::INFINITY;

    for i in 0..cam.height {
        let y = cam.height - i;
        let y = (y as f32 / cam.height as f32) - 0.5;

        if y < 0.0 {
            vline[i as usize] = Color::BLACK;
        } else {
            vline[i as usize] = Color::WHITE;
        }
    }

    for wall in walls {
        let a = cross(&(cam.pos - wall.pos), &ray_dir) / cross(&wall.side, &ray_dir);

        if a > 0.0 && a < 1.0 {
            let t = cross(&(wall.pos - cam.pos), &wall.side) / cross(&ray_dir, &wall.side);
            let instersection_point = cam.pos + ray_dir * t;
            let distance = (instersection_point - cam.pos).dot(&cam.normal);
            if t < 0.0 || distance > closest_distance {
                continue;
            }
            closest_distance = distance;

            let wall_height = 1.0 / distance;

            let texture_vline = texture.get_vline((a * texture.width as f32) as usize);

            for i in 0..cam.height {
                let y = cam.height - i;
                let y = (y as f32 / cam.height as f32) - 0.5;

                if y < -(wall_height / 2.0) {
                    vline[i as usize] = Color::BLACK;
                } else if y > -(wall_height / 2.0) && y < wall_height / 2.0 {
                    let offset_from_top_of_wall = (wall_height / 2.0) - y;
                    let wall_pixel_ratio = offset_from_top_of_wall / wall_height;

                    let pixel_idx = wall_pixel_ratio * texture.height as f32;

                    let r = texture_vline[pixel_idx as usize * 3];
                    let g = texture_vline[pixel_idx as usize * 3 + 1];
                    let b = texture_vline[pixel_idx as usize * 3 + 2];
                    vline[i as usize] = Color::RGB(r, g, b);
                } else {
                    vline[i as usize] = Color::WHITE;
                }
            }
        }
    }

    Ok(vline)
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
                let center = Vector2::new(x as f32 * 1.0, y as f32 * 1.0);
                walls.push(Wall {
                    pos: center + Vector2::new(-0.5, -0.5),
                    side: Vector2::new(1.0, 0.0),
                    color,
                });
                walls.push(Wall {
                    pos: center + Vector2::new(0.5, -0.5),
                    side: Vector2::new(0.0, 1.0),
                    color,
                });
                walls.push(Wall {
                    pos: center + Vector2::new(0.5, 0.5),
                    side: Vector2::new(-1.0, 0.0),
                    color,
                });
                walls.push(Wall {
                    pos: center + Vector2::new(-0.5, 0.5),
                    side: Vector2::new(0.0, -1.0),
                    color,
                });
            }
        }
    }
    walls
}

fn create_window(sdl_context: &Sdl) -> Result<Window, String> {
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Raycasting demo", W, H)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;
    Ok(window)
}

fn render(
    buffer: &mut [u8],
    camera: &Camera,
    walls: &Vec<Wall>,
    texture: &Texture,
) -> Result<(), String> {

    for x in 0..W {
        let vline: Vec<Color> = calculate_vline(x, camera, walls, texture)?;
        for y in 0..H {
            let color = vline[y as usize];
            let pixel_offset = (x + y * W) * 4;
            buffer[pixel_offset as usize + 0] = color.b; 
            buffer[pixel_offset as usize + 1] = color.g; 
            buffer[pixel_offset as usize + 2] = color.r; 
            // buffer[pixel_offset as usize + 3] = ignored;

        }
    }
    Ok(())
}

fn load_texture() -> Result<Texture, String> {
    let image = ImageReader::open("images/bricks-small.png").unwrap();
    let image = image.decode().unwrap();
    let image = image.into_rgb8();

    let row_major = image.clone().into_raw();
    let mut column_major = vec![];
    column_major.resize((image.height() * image.width() * 3) as usize, 0);
    for x in 0..image.width() {
        for y in 0..image.height() {
            for color in 0..3 {
                let dst_idx = (x * image.height() + y) * 3 + color;
                let src_idx = (y * image.width() + x) * 3 + color;

                column_major[dst_idx as usize] = row_major[src_idx as usize]
            }
        }
    }

    Ok(Texture {
        data: column_major,
        width: image.width(),
        height: image.height(),
    })
}

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let window = create_window(&sdl_context)?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    
    canvas.set_draw_color(Color::WHITE);
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump()?;
    let mut camera = Camera {
        pos: Vector2::new(1.5, 1.5),
        normal: Vector2::new(0.0, 1.0),
        height: H,
        width: W,
    };

    let map = read_map(Path::new("map.map"));
    // let map = sample_map();
    let walls = create_walls(&map);

    let left_rotation = nalgebra::UnitComplex::from_angle(-0.1);
    let right_rotation = nalgebra::UnitComplex::from_angle(0.1);

    let texture = load_texture()?;

    let texture_creator = canvas.texture_creator();
    let mut back_buffer = texture_creator
        .create_texture(
            None,
            sdl2::render::TextureAccess::Target,
            W,
            H,
        )
        .unwrap();

    let mut cpu_buffer = vec![];
    cpu_buffer.resize((W*H*4) as usize, 0 as u8);

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        if event_pump
            .keyboard_state()
            .is_scancode_pressed(sdl2::keyboard::Scancode::Left)
        {
            camera.normal = left_rotation * camera.normal;
        }
        if event_pump
            .keyboard_state()
            .is_scancode_pressed(sdl2::keyboard::Scancode::Right)
        {
            camera.normal = right_rotation * camera.normal;
        }
        if event_pump
            .keyboard_state()
            .is_scancode_pressed(sdl2::keyboard::Scancode::Up)
        {
            camera.pos += camera.normal * 0.2;
        }
        if event_pump
            .keyboard_state()
            .is_scancode_pressed(sdl2::keyboard::Scancode::Down)
        {
            camera.pos -= camera.normal * 0.2;
        }

        render(&mut cpu_buffer, &camera, &walls, &texture).unwrap();
        back_buffer.update(None, &cpu_buffer, (W*4) as usize).unwrap();
        canvas.copy(&back_buffer, None, None).unwrap();
        canvas.present();
    }

    Ok(())
}
