use criterion::{Criterion, criterion_group, criterion_main};
use motley::model::load_model;

fn benchmark_model_loading(c: &mut Criterion) {
    c.bench_function("Model loading", |b| {
        b.iter(|| {
            let model = load_model("assets/DamagedHelmet/DamagedHelmet.gltf");
            assert!(model.meshes.len() > 0);
        });
    });
}

fn create_criterion() -> Criterion {
    Criterion::default().configure_from_args()
}

criterion_group! {
    name = benches;
    config = create_criterion();
    targets = benchmark_model_loading
}

criterion_main!(benches);