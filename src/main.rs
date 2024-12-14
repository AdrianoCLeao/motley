use std::time::SystemTime;

use glam::*;

mod window;
use window::{Window, Framebuffer};
mod model;
use model::{Model, Vertex, load_model};

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
    v0: &Vertex, v1: &Vertex, v2: &Vertex,
    mvp: &Mat4,
    inv_trans_model_matrix: &Mat4
) {
    let v0_clip_space = project(&v0.position, mvp);
    let v1_clip_space = project(&v1.position, mvp);
    let v2_clip_space = project(&v2.position, mvp);

    let screen_size = Vec2::new(framebuffer.width() as f32, framebuffer.height() as f32);
    let v0_screen_space = clip_to_screen_space(&v0_clip_space.0.xy(), &screen_size);
    let v1_screen_space = clip_to_screen_space(&v1_clip_space.0.xy(), &screen_size);
    let v2_screen_space = clip_to_screen_space(&v2_clip_space.0.xy(), &screen_size);

    let min = v0_screen_space.min(v1_screen_space.min(v2_screen_space)).max(Vec2::ZERO);
    let max = (v0_screen_space.max(v1_screen_space.max(v2_screen_space)) + 1.0).min(screen_size);

    for x in (min.x as usize)..(max.x as usize) {
        for y in (min.y as usize)..(max.y as usize) {
            let p = Vec2::new(x as f32, y as f32) + 0.5;
            
            let a0 = edge_function(&v1_screen_space, &v2_screen_space, &p);
            let a1 = edge_function(&v2_screen_space, &v0_screen_space, &p);
            let a2 = edge_function(&v0_screen_space, &v1_screen_space, &p);
            let overlaps = a0 > 0.0 && a1 > 0.0 && a2 > 0.0;
            
            if overlaps {
                let area_rep = 1.0 / edge_function(&v0_screen_space, &v1_screen_space, &v2_screen_space);
                let bary_coords = Vec3::new(a0, a1, a2) * area_rep;
                let correction = 1.0 / (bary_coords.x * v0_clip_space.1
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
                                        + n2 * v2_clip_space.1 * bary_coords.z).xyz()
                                            * correction).normalize();

                    let normal_as_color = (normal * 0.5 + 0.5) * 255.99;

                    framebuffer.set_pixel(x, y, from_u8_rgb(normal_as_color.x as u8, normal_as_color.y as u8, normal_as_color.z as u8));
                }
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
    inv_trans_model_matrix: &Mat4
) {
    for mesh in &model.meshes {
        for i in 0..(mesh.indices.len() / 3) {
            let v0 = mesh.vertices[mesh.indices[i * 3] as usize];
            let v1 = mesh.vertices[mesh.indices[i * 3 + 1] as usize];
            let v2 = mesh.vertices[mesh.indices[i * 3 + 2] as usize];

            draw_triangle(
                framebuffer,
                depth_buffer,
                &v0, &v1, &v2,
                mvp,
                inv_trans_model_matrix
            );
        }
    }
}

/*
Main function sets up the window, depth buffer, and the rendering pipeline. It loads a GLTF model
and continuously renders it to the screen while applying transformations for rotation.
*/
fn main() {
    let mut window: Window = Window::new("Motley project", 512, 512);
    let mut depth_buffer = Framebuffer::new(window.framebuffer().width(), window.framebuffer().height());

    let model = load_model("assets/DamagedHelmet/DamagedHelmet.gltf");

    let timer = SystemTime::now();

    while !window.should_close() {
        let framebuffer = window.framebuffer();

        if framebuffer.width() != depth_buffer.width() || framebuffer.height() != depth_buffer.height() {
            depth_buffer = Framebuffer::new(framebuffer.width(), framebuffer.height());
        }

        framebuffer.clear(from_u8_rgb(20, 20, 20));
        depth_buffer.clear(u32::MAX);

        let aspect_ratio = framebuffer.width() as f32 / framebuffer.height() as f32;
        let model_matrix = Mat4::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), timer.elapsed().unwrap().as_secs_f32()) * Mat4::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), (90.0f32).to_radians());
        let view_matrix = Mat4::from_translation(Vec3::new(0.0, 0.0, -2.5));
        let proj_matrix = Mat4::perspective_rh((60.0f32).to_radians(), aspect_ratio, 0.01, 300.0);
        let mvp_matrix = proj_matrix * view_matrix * model_matrix;
        let inv_trans_model_matrix = model_matrix.inverse().transpose();

        draw_model(
            framebuffer,
            &mut depth_buffer,
            &model,
            &mvp_matrix,
            &inv_trans_model_matrix
        );

        window.display();
    }
}