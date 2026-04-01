use engine_assets::AssetServer;

#[test]
fn resolve_path_rejects_parent_traversal() {
    let server = AssetServer::new("assets");
    let error = server
        .resolve_path("../outside.txt")
        .expect_err("parent traversal should be rejected");

    assert!(error.to_string().contains("traversal"));
}

#[test]
fn resolve_path_rejects_absolute_path() {
    let server = AssetServer::new("assets");
    let absolute = std::env::temp_dir().join("outside.txt");
    let error = server
        .resolve_path(&absolute.to_string_lossy())
        .expect_err("absolute paths should be rejected");

    assert!(error.to_string().contains("root-relative"));
}
