use criterion::{Criterion, criterion_group, criterion_main};
use motley::{gui::Window, model::load_model};

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

// Registra os benchmarks
criterion_group!(
    benches,
    benchmark_window_creation,
    benchmark_model_loading,
);

// Define o ponto de entrada principal dos benchmarks
criterion_main!(benches);
