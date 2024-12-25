use glam::*;
use std::sync::{Arc, Mutex};

pub mod gui;
use gui::components::setup_menu;
use gui::{Framebuffer, Window};

pub mod model;
use model::{load_model, Material, Model, Vertex};

/*
This program implements a basic 3D renderer using a software rasterizer. It includes functionalities
to load and process 3D models, transform vertices, and render triangles onto a framebuffer. It
utilizes depth buffering for proper triangle occlusion and calculates pixel-level normals for shading.
*/

/*
Converts RGB color values from individual u8 components to a single 32-bit integer
for framebuffer compatibility.
*/
fn from_u8_rgb(r: u8, g: u8, b: u8) -> u32 {
    let (r, g, b) = (r as u32, g as u32, b as u32);
    (r << 16) | (g << 8) | b
}

fn from_vec3_rgb(rgb: &Vec3) -> u32 {
    from_u8_rgb(
        (rgb.x * 255.99) as u8,
        (rgb.y * 255.99) as u8,
        (rgb.z * 255.99) as u8,
    )
}

/*
Calculates the edge function for a triangle, which is used to determine whether
a point lies inside the triangle based on its barycentric coordinates.
*/
fn edge_function(a: &Vec2, c: &Vec2, b: &Vec2) -> f32 {
    (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)
}

/*
Renders a single triangle to the framebuffer. It performs perspective transformations, rasterization,
depth testing, and normal correction to compute a color for each pixel in the triangle.
*/
fn draw_triangle(
    framebuffer: &mut Framebuffer,
    depth_buffer: &mut Framebuffer,
    v0: &Vertex,
    v1: &Vertex,
    v2: &Vertex,
    mvp: &Mat4,
    inv_trans_model_matrix: &Mat4,
    material: &Material,
) {
    // --- Back-face Culling ---
    let normal = (v1.position - v0.position)
        .cross(v2.position - v0.position)
        .normalize();
    let camera_dir = Vec3::new(0.0, 0.0, -1.0);
    let cos_angle = normal.dot(camera_dir);

    if cos_angle < -0.9 {
        return;
    }

    let v0_clip_space = project(&v0.position, mvp);
    let v1_clip_space = project(&v1.position, mvp);
    let v2_clip_space = project(&v2.position, mvp);

    let screen_size = Vec2::new(framebuffer.width() as f32, framebuffer.height() as f32);
    let v0_screen = clip_to_screen_space(&v0_clip_space.0.xy(), &screen_size);
    let v1_screen = clip_to_screen_space(&v1_clip_space.0.xy(), &screen_size);
    let v2_screen = clip_to_screen_space(&v2_clip_space.0.xy(), &screen_size);

    let area_rep = 1.0 / edge_function(&v0_screen, &v1_screen, &v2_screen);

    // --- Tile-based Rasterization ---
    let tile_size = 4;
    let min = v0_screen.min(v1_screen.min(v2_screen)).max(Vec2::ZERO);
    let max = (v0_screen.max(v1_screen.max(v2_screen)) + 1.0).min(screen_size);

    for tile_y in ((min.y as usize)..(max.y as usize)).step_by(tile_size) {
        for tile_x in ((min.x as usize)..(max.x as usize)).step_by(tile_size) {
            let tile_min = Vec2::new(tile_x as f32, tile_y as f32);
            let tile_max = (tile_min + tile_size as f32).min(screen_size);

            let w0 = edge_function(&v1_screen, &v2_screen, &tile_min);
            let w1 = edge_function(&v2_screen, &v0_screen, &tile_min);
            let w2 = edge_function(&v0_screen, &v1_screen, &tile_min);

            if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                continue;
            }

            // --- Edge Function Incremental ---
            let step_x = Vec3::new(
                v1_screen.y - v2_screen.y,
                v2_screen.y - v0_screen.y,
                v0_screen.y - v1_screen.y,
            );

            let step_y = Vec3::new(
                v2_screen.x - v1_screen.x,
                v0_screen.x - v2_screen.x,
                v1_screen.x - v0_screen.x,
            );

            let mut w0_row = w0;
            let mut w1_row = w1;
            let mut w2_row = w2;

            for y in (tile_min.y as usize)..(tile_max.y as usize) {
                let mut w0 = w0_row;
                let mut w1 = w1_row;
                let mut w2 = w2_row;

                for x in (tile_min.x as usize)..(tile_max.x as usize) {
                    if w0 > 0.0 && w1 > 0.0 && w2 > 0.0 {
                        let bary_coords = Vec3::new(w0, w1, w2) * area_rep;
                        let correction = 1.0
                            / (bary_coords.x * v0_clip_space.1
                                + bary_coords.y * v1_clip_space.1
                                + bary_coords.z * v2_clip_space.1);

                        let z = v0_clip_space.0.z * bary_coords.x
                            + v1_clip_space.0.z * bary_coords.y
                            + v2_clip_space.0.z * bary_coords.z;

                        let depth = depth_buffer.get_pixel_f32(x, y);
                        if z < depth {
                            depth_buffer.set_pixel_f32(x, y, z);

                            let n0 = *inv_trans_model_matrix * Vec4::from((v0.normal, 1.0));
                            let n1 = *inv_trans_model_matrix * Vec4::from((v1.normal, 1.0));
                            let n2 = *inv_trans_model_matrix * Vec4::from((v2.normal, 1.0));
                            let normal = ((n0 * v0_clip_space.1 * bary_coords.x
                                + n1 * v1_clip_space.1 * bary_coords.y
                                + n2 * v2_clip_space.1 * bary_coords.z)
                                .xyz()
                                * correction)
                                .normalize();

                            let tex_coord = (v0.tex_coord * v0_clip_space.1 * bary_coords.x
                                + v1.tex_coord * v1_clip_space.1 * bary_coords.y
                                + v2.tex_coord * v2_clip_space.1 * bary_coords.z)
                                * correction;

                            let mut base_color = material.base_color;
                            if let Some(base_color_texture) = &material.base_color_texture {
                                base_color *=
                                    base_color_texture.sample_pixel(tex_coord.x, tex_coord.y);
                            }

                            let light_dir = Vec3::new(0.3, -0.8, -0.4).normalize();
                            let light_intensity = normal.dot(-light_dir);
                            let final_color = base_color * light_intensity;

                            framebuffer.set_pixel(x, y, from_vec3_rgb(&final_color.xyz()));
                        }
                    }
                    w0 += step_x.x;
                    w1 += step_x.y;
                    w2 += step_x.z;
                }

                w0_row += step_y.x;
                w1_row += step_y.y;
                w2_row += step_y.z;
            }
        }
    }
}

/*
Applies a perspective projection to a vertex position using the Model-View-Projection (MVP) matrix.
Returns the projected position and its reciprocal for later depth correction.
*/
fn project(p: &Vec3, mvp: &Mat4) -> (Vec3, f32) {
    let proj_pos = *mvp * Vec4::from((*p, 1.0));
    let rec = 1.0 / proj_pos.w;
    let rec_pos = proj_pos * rec;
    (Vec3::new(rec_pos.x, rec_pos.y, rec_pos.z), rec)
}

/*
Converts a vertex position from clip space to screen space, scaling it to fit the framebuffer dimensions.
*/
fn clip_to_screen_space(clip_space: &Vec2, screen_size: &Vec2) -> Vec2 {
    (*clip_space * -0.5 + 0.5) * *screen_size
}

/*
Renders all the meshes in a model by iterating through their indices. Each triangle is
transformed and rasterized onto the framebuffer.
*/
fn draw_model(
    framebuffer: &mut Framebuffer,
    depth_buffer: &mut Framebuffer,
    model: &Model,
    mvp: &Mat4,
    inv_trans_model_matrix: &Mat4,
) {
    for mesh in &model.meshes {
        for i in 0..(mesh.indices.len() / 3) {
            let v0 = mesh.vertices[mesh.indices[i * 3] as usize];
            let v1 = mesh.vertices[mesh.indices[i * 3 + 1] as usize];
            let v2 = mesh.vertices[mesh.indices[i * 3 + 2] as usize];

            let material = &model.materials[mesh.material_idx];

            draw_triangle(
                framebuffer,
                depth_buffer,
                &v0,
                &v1,
                &v2,
                mvp,
                inv_trans_model_matrix,
                material,
            );
        }
    }
}

/*
Main function sets up the window, depth buffer, and the rendering pipeline. It loads a GLTF model
and continuously renders it to the screen while applying transformations for rotation.
*/

fn main() {
    let mut window: Window = Window::new("Motley Project", 800, 600);
    let (fb_width, fb_height) = window.framebuffer_area();
    let mut depth_buffer = Framebuffer::new(fb_width, fb_height);

    let model = load_model("assets/DamagedHelmet/DamagedHelmet.gltf");

    let rotation = Arc::new(Mutex::new(Vec2::ZERO));
    let zoom = Arc::new(Mutex::new(2.5));

    setup_menu(&mut window, Arc::clone(&rotation), Arc::clone(&zoom));

    let last_mouse_pos: Arc<Mutex<Option<(f32, f32)>>> = Arc::new(Mutex::new(None));

    while !window.should_close() {
        let mouse_pos = window.get_mouse_pos();
        let mouse_left_down = window.is_mouse_down(minifb::MouseButton::Left);

        if let Some(mouse_pos) = mouse_pos {
            let within_framebuffer = mouse_pos.0 >= window.sidebar_width() as f32
                && mouse_pos.0 < (window.sidebar_width() + fb_width) as f32
                && mouse_pos.1 >= 0.0
                && mouse_pos.1 < fb_height as f32;

            if mouse_left_down {
                if within_framebuffer {
                    let mut last_mouse_pos = last_mouse_pos.lock().unwrap();
                    if let Some(last_pos) = *last_mouse_pos {
                        let delta = Vec2::new(last_pos.0 - mouse_pos.0, mouse_pos.1 - last_pos.1);
                        *rotation.lock().unwrap() += Vec2::new(delta.x, delta.y) * 0.01;
                    }
                    *last_mouse_pos = Some(mouse_pos);
                } else {
                    if mouse_pos.0 < window.sidebar_width() as f32 {
                        window.process_menu_click(mouse_pos.0, mouse_pos.1);
                    }
                }
            } else {
                *last_mouse_pos.lock().unwrap() = None;
            }
        }

        let framebuffer = window.framebuffer();

        if framebuffer.width() != depth_buffer.width()
            || framebuffer.height() != depth_buffer.height()
        {
            depth_buffer = Framebuffer::new(framebuffer.width(), framebuffer.height());
        }

        framebuffer.clear(0x141414);
        depth_buffer.clear(u32::MAX);

        let aspect_ratio = framebuffer.width() as f32 / framebuffer.height() as f32;

        let rotation = rotation.lock().unwrap();
        let zoom = zoom.lock().unwrap();
        let model_matrix = Mat4::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), rotation.x)
            * Mat4::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), rotation.y);
        let view_matrix = Mat4::from_translation(Vec3::new(0.0, 0.0, -(*zoom)));
        let proj_matrix = Mat4::perspective_rh((60.0f32).to_radians(), aspect_ratio, 0.01, 300.0);
        let mvp_matrix = proj_matrix * view_matrix * model_matrix;
        let inv_trans_model_matrix = model_matrix.inverse().transpose();

        draw_model(
            framebuffer,
            &mut depth_buffer,
            &model,
            &mvp_matrix,
            &inv_trans_model_matrix,
        );

        window.framebuffer().render_orientation_cube(50);
        window.render_bottom_bar();
        window.display();
    }
}
