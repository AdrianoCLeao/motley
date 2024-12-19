use criterion::{criterion_group, criterion_main, Criterion};
use glam::*;
use std::sync::{Arc, Mutex};
use Motley::{gui::Window, model::load_model};

fn benchmark_window_creation(c: &mut Criterion) {
    c.bench_function("Window creation", |b| {
        b.iter(|| {
            let mut window = Window::new("Benchmark Test", 512, 512);
            window.display();
        });
    });
}

fn benchmark_model_loading(c: &mut Criterion) {
    c.bench_function("Model loading", |b| {
        b.iter(|| {
            let model = load_model("assets/DamagedHelmet/DamagedHelmet.gltf");
            assert!(model.meshes.len() > 0); 
        });
    });
}

fn benchmark_rendering(c: &mut Criterion) {
    c.bench_function("Triangle rendering", |b| {
        b.iter(|| {
            let framebuffer = Motley::gui::Framebuffer::new(512, 512);
            let depth_buffer = Motley::gui::Framebuffer::new(512, 512);

            // Exemplo de renderização fictícia (adicione seus parâmetros reais):
            // draw_triangle(&mut framebuffer, &mut depth_buffer, &vertex_0, &vertex_1, &vertex_2, &mvp, &inv_trans_model_matrix);
        });
    });
}

criterion_group!(benches, benchmark_window_creation, benchmark_model_loading, benchmark_rendering);
criterion_main!(benches);
