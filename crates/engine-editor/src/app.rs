use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy_ecs::entity::Entity;
use bevy_ecs::system::RunSystemOnce;
use bevy_ecs::world::World;
use eframe::egui;
use egui_dock::{DockArea, DockState, TabViewer};
use engine_assets::{AssetServer, SceneDeserializer, SceneSerializer};
use engine_core::{
    create_world, register_core_reflection_types, Camera2d, Camera3d, EditorEntityBundle,
    EntityName, GlobalTransform, PrimaryCamera, RenderLayer2D, RenderLayer3D, Result,
    SpatialBundle, Transform, Visible,
};
use engine_physics::register_physics_reflection_types;
use engine_reflect::{ComponentRegistry, ReflectMetadataRegistry, ReflectTypeRegistry};

use crate::config::EditorConfig;
use crate::layout::{create_default_layout, Tab};
use crate::viewport::{
    MeshRenderable3d, RenderSceneAdapter, SpriteRenderable2d, ViewportRenderer,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum LogLevel {
    Trace,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug)]
struct LogEntry {
    level: LogLevel,
    message: String,
    timestamp: f64,
    module: String,
}

struct ConsolePanel {
    entries: Vec<LogEntry>,
    filter: LogLevel,
    auto_scroll: bool,
}

impl Default for ConsolePanel {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            filter: LogLevel::Trace,
            auto_scroll: true,
        }
    }
}

impl ConsolePanel {
    fn push(&mut self, level: LogLevel, message: impl Into<String>, module: impl Into<String>, timestamp: f64) {
        self.entries.push(LogEntry {
            level,
            message: message.into(),
            module: module.into(),
            timestamp,
        });

        const MAX_ENTRIES: usize = 2_000;
        if self.entries.len() > MAX_ENTRIES {
            let overflow = self.entries.len() - MAX_ENTRIES;
            self.entries.drain(0..overflow);
        }
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.selectable_value(&mut self.filter, LogLevel::Trace, "All");
            ui.selectable_value(&mut self.filter, LogLevel::Info, "Info+");
            ui.selectable_value(&mut self.filter, LogLevel::Warn, "Warn+");
            ui.selectable_value(&mut self.filter, LogLevel::Error, "Error");
            ui.separator();
            if ui.button("Clear").clicked() {
                self.entries.clear();
            }
            ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(self.auto_scroll)
            .show(ui, |ui| {
                for entry in self.entries.iter().filter(|entry| entry.level >= self.filter) {
                    let color = match entry.level {
                        LogLevel::Error => egui::Color32::RED,
                        LogLevel::Warn => egui::Color32::YELLOW,
                        LogLevel::Info => egui::Color32::WHITE,
                        LogLevel::Trace => egui::Color32::GRAY,
                    };

                    ui.colored_label(
                        color,
                        format!(
                            "[{:.2}] [{}] {}",
                            entry.timestamp,
                            entry.module,
                            entry.message
                        ),
                    );
                }
            });
    }
}

pub struct EditorApp {
    pub dock_state: DockState<Tab>,
    pub world: World,
    pub asset_server: AssetServer,
    pub file_path: Option<PathBuf>,
    pub unsaved_changes: bool,
    pub show_about: bool,
    config: EditorConfig,
    selected_entity: Option<Entity>,
    wgpu_render_state: Option<eframe::egui_wgpu::RenderState>,
    viewport_renderer: ViewportRenderer,
    started_at: Instant,
    console: ConsolePanel,
}

impl EditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = EditorConfig::load();
        let mut world = build_editor_world();
        world.spawn(EditorEntityBundle::default());

        let mut console = ConsolePanel::default();
        console.push(
            LogLevel::Info,
            "Editor initialized",
            "engine::editor",
            0.0,
        );

        let mut app = Self {
            dock_state: config.dock_state.clone(),
            world,
            asset_server: AssetServer::new("assets"),
            file_path: None,
            unsaved_changes: false,
            show_about: false,
            config,
            selected_entity: None,
            wgpu_render_state: cc.wgpu_render_state.clone(),
            viewport_renderer: ViewportRenderer::new(),
            started_at: Instant::now(),
            console,
        };

        if let Some(last_scene) = app.config.last_opened_scene.clone() {
            if last_scene.exists() {
                if let Err(error) = app.load_scene(&last_scene) {
                    app.log_message(
                        LogLevel::Warn,
                        format!("Failed to restore last scene {}: {}", last_scene.display(), error),
                    );
                } else {
                    app.file_path = Some(last_scene.clone());
                    app.log_message(
                        LogLevel::Info,
                        format!("Restored last scene {}", last_scene.display()),
                    );
                }
            }
        }

        app.bootstrap_viewport_scene();

        app
    }

    fn now_seconds(&self) -> f64 {
        self.started_at.elapsed().as_secs_f64()
    }

    fn log_message(&mut self, level: LogLevel, message: impl Into<String>) {
        let timestamp = self.now_seconds();
        self.console
            .push(level, message.into(), "engine::editor", timestamp);
    }

    fn sync_config_from_runtime(&mut self) {
        self.config.dock_state = self.dock_state.clone();
        self.config.last_opened_scene = self.file_path.clone();
    }

    fn persist_config(&mut self) {
        self.sync_config_from_runtime();
        if let Err(error) = self.config.save() {
            self.log_message(
                LogLevel::Warn,
                format!("Failed to persist editor config: {}", error),
            );
        }
    }

    fn bootstrap_viewport_scene(&mut self) {
        self.ensure_viewport_world_defaults();
        self.ensure_primary_camera();
        self.ensure_preview_mesh();
        self.sync_global_transforms_for_viewport();
    }

    fn ensure_viewport_world_defaults(&mut self) {
        let entities: Vec<Entity> = self.world.iter_entities().map(|entity| entity.id()).collect();

        for entity in entities {
            let has_transform = self.world.get::<Transform>(entity).is_some();
            let has_global = self.world.get::<GlobalTransform>(entity).is_some();
            let has_camera3d = self.world.get::<Camera3d>(entity).is_some();
            let has_camera2d = self.world.get::<Camera2d>(entity).is_some();
            let has_mesh = self.world.get::<MeshRenderable3d>(entity).is_some();
            let has_sprite = self.world.get::<SpriteRenderable2d>(entity).is_some();
            let has_visible = self.world.get::<Visible>(entity).is_some();
            let has_layer3d = self.world.get::<RenderLayer3D>(entity).is_some();
            let has_layer2d = self.world.get::<RenderLayer2D>(entity).is_some();

            if let Ok(mut entity_ref) = self.world.get_entity_mut(entity) {
                if has_transform && !has_global {
                    entity_ref.insert(GlobalTransform::default());
                }

                if (has_camera3d || has_camera2d || has_mesh || has_sprite) && !has_visible {
                    entity_ref.insert(Visible);
                }

                if has_mesh && !has_layer3d {
                    entity_ref.insert(RenderLayer3D);
                }

                if has_sprite && !has_layer2d {
                    entity_ref.insert(RenderLayer2D);
                }
            }
        }
    }

    fn ensure_primary_camera(&mut self) {
        let has_primary_camera = self
            .world
            .iter_entities()
            .any(|entity| {
                let id = entity.id();
                self.world.get::<Camera3d>(id).is_some()
                    && self.world.get::<PrimaryCamera>(id).is_some()
            });

        if has_primary_camera {
            return;
        }

        self.world.spawn((
            EntityName::new("Editor Camera"),
            SpatialBundle {
                transform: Transform::from_xyz(0.0, 4.0, 10.0),
                ..SpatialBundle::default()
            },
            Camera3d::default(),
            PrimaryCamera,
            Visible,
        ));
        self.log_message(LogLevel::Info, "Inserted default editor camera for viewport rendering");
    }

    fn ensure_preview_mesh(&mut self) {
        let has_mesh = self
            .world
            .iter_entities()
            .any(|entity| self.world.get::<MeshRenderable3d>(entity.id()).is_some());

        if has_mesh {
            return;
        }

        let mesh = match self.asset_server.load_mesh_handle("meshes/cube.glb") {
            Ok(handle) => handle,
            Err(error) => {
                self.log_message(
                    LogLevel::Warn,
                    format!("Preview mesh load failed (meshes/cube.glb): {}", error),
                );
                return;
            }
        };

        let texture = match self.asset_server.load_texture_handle("textures/placeholder.png") {
            Ok(handle) => handle,
            Err(error) => {
                self.log_message(
                    LogLevel::Warn,
                    format!("Preview texture load failed (textures/placeholder.png): {}", error),
                );
                return;
            }
        };

        let material = match self.asset_server.load_material_handle("materials/default.ron") {
            Ok(handle) => handle,
            Err(error) => {
                self.log_message(
                    LogLevel::Warn,
                    format!("Preview material load failed (materials/default.ron): {}", error),
                );
                return;
            }
        };

        self.world.spawn((
            EntityName::new("Preview Cube"),
            SpatialBundle {
                transform: Transform::from_xyz(0.0, 0.5, 0.0),
                ..SpatialBundle::default()
            },
            MeshRenderable3d::new(mesh, texture, material),
            Visible,
            RenderLayer3D,
        ));

        self.log_message(
            LogLevel::Info,
            "Inserted default preview cube for viewport validation",
        );
    }

    fn sync_global_transforms_for_viewport(&mut self) {
        if let Err(error) = self.world.run_system_once(engine_core::propagate_transforms) {
            self.log_message(
                LogLevel::Warn,
                format!("Transform propagation failed before viewport render: {}", error),
            );
        }
    }

    fn draw_menu_bar(&mut self, ctx: &egui::Context) {
        let recent_files = self.config.recent_files.clone();
        let mut open_recent: Option<PathBuf> = None;

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui: &mut egui::Ui| {
                ui.menu_button("File", |ui: &mut egui::Ui| {
                    if ui.button("New Scene          Ctrl+N").clicked() {
                        if let Err(error) = self.cmd_new_scene() {
                            self.log_message(LogLevel::Error, format!("New scene failed: {}", error));
                        }
                    }

                    if ui.button("Open Scene...      Ctrl+O").clicked() {
                        if let Err(error) = self.cmd_open_scene() {
                            self.log_message(LogLevel::Error, format!("Open scene failed: {}", error));
                        }
                    }

                    ui.menu_button("Open Recent", |ui| {
                        if recent_files.is_empty() {
                            ui.add_enabled(false, egui::Button::new("No recent files"));
                        } else {
                            for path in &recent_files {
                                if ui.button(path.display().to_string()).clicked() {
                                    open_recent = Some(path.clone());
                                    ui.close_menu();
                                }
                            }
                        }
                    });

                    ui.separator();

                    if ui
                        .add_enabled(self.file_path.is_some(), egui::Button::new("Save            Ctrl+S"))
                        .clicked()
                    {
                        if let Err(error) = self.cmd_save() {
                            self.log_message(LogLevel::Error, format!("Save failed: {}", error));
                        }
                    }

                    if ui.button("Save As...      Ctrl+Shift+S").clicked() {
                        if let Err(error) = self.cmd_save_as() {
                            self.log_message(LogLevel::Error, format!("Save as failed: {}", error));
                        }
                    }

                    ui.separator();

                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Edit", |ui: &mut egui::Ui| {
                    ui.add_enabled(false, egui::Button::new("Undo    Ctrl+Z"));
                    ui.add_enabled(false, egui::Button::new("Redo    Ctrl+Y"));
                    ui.separator();
                    ui.add_enabled(false, egui::Button::new("Delete    Del"));
                    ui.add_enabled(false, egui::Button::new("Duplicate    Ctrl+D"));
                });

                ui.menu_button("Help", |ui: &mut egui::Ui| {
                    if ui.button("About").clicked() {
                        self.show_about = true;
                    }
                });
            });
        });

        if let Some(path) = open_recent {
            if let Err(error) = self.open_scene_from_path(&path) {
                self.log_message(LogLevel::Error, format!("Open recent failed: {}", error));
            }
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let mut new_scene = false;
        let mut open_scene = false;
        let mut save_scene = false;
        let mut save_as_scene = false;
        let mut clear_selection = false;

        ctx.input_mut(|input| {
            new_scene = input.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::N,
            ));
            open_scene = input.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::O,
            ));
            save_scene = input.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::S,
            ));
            save_as_scene = input.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers {
                    alt: false,
                    ctrl: true,
                    shift: true,
                    mac_cmd: false,
                    command: false,
                },
                egui::Key::S,
            ));
            clear_selection = input.key_pressed(egui::Key::Escape);
        });

        if new_scene {
            if let Err(error) = self.cmd_new_scene() {
                self.log_message(LogLevel::Error, format!("New scene failed: {}", error));
            }
        }

        if open_scene {
            if let Err(error) = self.cmd_open_scene() {
                self.log_message(LogLevel::Error, format!("Open scene failed: {}", error));
            }
        }

        if save_as_scene {
            if let Err(error) = self.cmd_save_as() {
                self.log_message(LogLevel::Error, format!("Save as failed: {}", error));
            }
        } else if save_scene {
            if let Err(error) = self.cmd_save() {
                self.log_message(LogLevel::Error, format!("Save failed: {}", error));
            }
        }

        if clear_selection {
            self.selected_entity = None;
        }
    }

    fn draw_status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(22.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let entity_count = self.world.entities().len();
                    ui.label(format!("Entities: {}", entity_count));
                    ui.separator();

                    let fps = 1.0 / ctx.input(|i| i.predicted_dt.max(0.0001));
                    ui.label(format!("FPS: {:.0}", fps));
                    ui.separator();

                    if let Some(path) = &self.file_path {
                        let name = path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("untitled");

                        if self.unsaved_changes {
                            ui.label(format!("* {}", name));
                        } else {
                            ui.label(name);
                        }
                    } else {
                        ui.label("untitled");
                    }
                });
            });
    }

    fn draw_modals(&mut self, ctx: &egui::Context) {
        if !self.show_about {
            return;
        }

        let mut open = true;
        egui::Window::new("About Motley Editor")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Motley Editor (EP-09 foundation)");
                ui.label("Rust + egui + wgpu");
            });

        self.show_about = open;
    }

    fn cmd_new_scene(&mut self) -> Result<()> {
        self.world.clear_entities();
        self.world.spawn(EditorEntityBundle::default());
        self.bootstrap_viewport_scene();
        self.file_path = None;
        self.selected_entity = None;
        self.unsaved_changes = false;
        self.persist_config();
        self.log_message(LogLevel::Info, "Created new scene");
        Ok(())
    }

    fn cmd_open_scene(&mut self) -> Result<()> {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Scene RON", &["ron"])
            .pick_file()
        else {
            return Ok(());
        };

        self.open_scene_from_path(&path)
    }

    fn open_scene_from_path(&mut self, path: &Path) -> Result<()> {
        self.load_scene(path)?;
        self.file_path = Some(path.to_path_buf());
        self.config.touch_recent_file(path.to_path_buf());
        self.unsaved_changes = false;
        self.persist_config();
        self.log_message(LogLevel::Info, format!("Opened scene {}", path.display()));
        Ok(())
    }

    fn cmd_save(&mut self) -> Result<()> {
        if let Some(path) = self.file_path.clone() {
            self.save_scene(&path)?;
            self.config.touch_recent_file(path.clone());
            self.unsaved_changes = false;
            self.persist_config();
            self.log_message(LogLevel::Info, format!("Saved scene {}", path.display()));
            return Ok(());
        }

        self.cmd_save_as()
    }

    fn cmd_save_as(&mut self) -> Result<()> {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Scene RON", &["ron"])
            .set_file_name("untitled.scene.ron")
            .save_file()
        else {
            return Ok(());
        };

        self.save_scene(&path)?;
        self.file_path = Some(path.clone());
        self.config.touch_recent_file(path.clone());
        self.unsaved_changes = false;
        self.persist_config();
        self.log_message(LogLevel::Info, format!("Saved scene {}", path.display()));

        Ok(())
    }

    fn save_scene(&mut self, path: &Path) -> Result<()> {
        let scene_name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");

        let render_scene_adapter = RenderSceneAdapter;

        self.with_scene_context(|world, component_registry, type_registry, metadata_registry, asset_server| {
            let serializer = SceneSerializer::new(world, component_registry, type_registry)
                .with_metadata_registry(metadata_registry)
                .with_asset_server(asset_server)
                .with_external_components(&render_scene_adapter);
            serializer.save_file(path, scene_name)?;
            Ok(())
        })
    }

    fn load_scene(&mut self, path: &Path) -> Result<()> {
        let render_scene_adapter = RenderSceneAdapter;

        self.with_scene_context(|world, component_registry, type_registry, _metadata_registry, asset_server| {
            world.clear_entities();
            let mut deserializer = SceneDeserializer::new(world, component_registry, type_registry, asset_server)
                .with_external_components(&render_scene_adapter);
            let _ = deserializer.load_file(path)?;
            Ok(())
        })?;

        self.bootstrap_viewport_scene();
        self.selected_entity = None;

        Ok(())
    }

    fn with_scene_context<R>(
        &mut self,
        f: impl FnOnce(
            &mut World,
            &ComponentRegistry,
            &ReflectTypeRegistry,
            &ReflectMetadataRegistry,
            &mut AssetServer,
        ) -> Result<R>,
    ) -> Result<R> {
        let type_registry = self
            .world
            .remove_resource::<ReflectTypeRegistry>()
            .unwrap_or_default();
        let component_registry = self
            .world
            .remove_resource::<ComponentRegistry>()
            .unwrap_or_default();
        let metadata_registry = self
            .world
            .remove_resource::<ReflectMetadataRegistry>()
            .unwrap_or_default();

        let result = f(
            &mut self.world,
            &component_registry,
            &type_registry,
            &metadata_registry,
            &mut self.asset_server,
        );

        self.world.insert_resource(type_registry);
        self.world.insert_resource(component_registry);
        self.world.insert_resource(metadata_registry);

        result
    }

    fn show_viewport_panel(&mut self, ui: &mut egui::Ui) {
        let Some(render_state) = self.wgpu_render_state.clone() else {
            ui.vertical_centered_justified(|ui| {
                ui.label("WGPU renderer state unavailable in this runtime.");
            });
            return;
        };

        let available = ui.available_size();
        let width = available.x.max(1.0) as u32;
        let height = available.y.max(1.0) as u32;

        self.ensure_viewport_world_defaults();
        self.sync_global_transforms_for_viewport();

        self.viewport_renderer
            .ensure_size(&render_state, width, height);
        self.viewport_renderer
            .render(&render_state, &mut self.world, &self.asset_server);

        if let Some(texture_id) = self.viewport_renderer.texture_id() {
            let sized_texture = egui::load::SizedTexture::new(
                texture_id,
                egui::vec2(width as f32, height as f32),
            );
            let _ = ui.add(egui::Image::new(sized_texture).sense(egui::Sense::click_and_drag()));
        }

        if let Some(error) = self.viewport_renderer.last_error() {
            ui.colored_label(egui::Color32::RED, format!("Viewport render error: {}", error));
        } else {
            ui.label("Viewport rendering via editor offscreen backend.");
        }
    }

    fn show_scene_tree_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Scene Hierarchy");
        ui.separator();

        let roots: Vec<Entity> = self
            .world
            .iter_entities()
            .filter_map(|entity_ref| {
                let entity = entity_ref.id();
                if self.world.get::<engine_core::Parent>(entity).is_some() {
                    None
                } else {
                    Some(entity)
                }
            })
            .collect();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for entity in roots.iter().copied() {
                self.draw_entity_node(ui, entity, 0);
            }
        });
    }

    fn draw_entity_node(&mut self, ui: &mut egui::Ui, entity: Entity, depth: usize) {
        let indent = depth as f32 * 14.0;

        let name = self
            .world
            .get::<EntityName>(entity)
            .map(|value| value.0.clone())
            .unwrap_or_else(|| format!("Entity {:?}", entity));

        let is_selected = self.selected_entity == Some(entity);
        ui.horizontal(|ui| {
            ui.add_space(indent);
            if ui.selectable_label(is_selected, name).clicked() {
                self.selected_entity = Some(entity);
            }
        });

        let children = self
            .world
            .get::<engine_core::Children>(entity)
            .map(|children| children.0.clone())
            .unwrap_or_default();

        for child in children {
            self.draw_entity_node(ui, child, depth + 1);
        }
    }

    fn show_inspector_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Inspector");
        ui.separator();
        ui.label("Read-only foundation. Property editing starts in EP-10.");

        let Some(entity) = self.selected_entity else {
            ui.label("No entity selected.");
            return;
        };

        if self.world.get_entity(entity).is_err() {
            self.selected_entity = None;
            ui.label("Selection no longer exists.");
            return;
        }

        ui.label(format!("Entity: {:?}", entity));

        if let Some(name) = self.world.get::<EntityName>(entity) {
            ui.label(format!("EntityName: {}", name.0));
        }

        if let Some(transform) = self.world.get::<engine_core::Transform>(entity) {
            ui.separator();
            ui.label("Transform");
            ui.label(format!(
                "translation: ({:.3}, {:.3}, {:.3})",
                transform.translation.x, transform.translation.y, transform.translation.z
            ));
            ui.label(format!(
                "scale: ({:.3}, {:.3}, {:.3})",
                transform.scale.x, transform.scale.y, transform.scale.z
            ));
        }
    }

    fn show_asset_browser_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Assets");
        ui.separator();
        ui.label(format!("Root: {}", self.asset_server.root().as_str()));

        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Ok(entries) = std::fs::read_dir("assets") {
                for entry in entries.flatten() {
                    ui.label(entry.path().display().to_string());
                }
            } else {
                ui.label("assets/ directory not found");
            }
        });
    }

    fn show_console_panel(&mut self, ui: &mut egui::Ui) {
        self.console.show(ui);
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_keyboard_shortcuts(ctx);
        self.draw_menu_bar(ctx);
        self.draw_status_bar(ctx);

        let mut dock_state = std::mem::replace(&mut self.dock_state, create_default_layout());
        {
            let mut tab_viewer = EditorTabViewer { app: self };
            DockArea::new(&mut dock_state)
                .style(egui_dock::Style::from_egui(ctx.style().as_ref()))
                .show(ctx, &mut tab_viewer);
        }
        self.dock_state = dock_state;

        self.draw_modals(ctx);
    }
}

impl Drop for EditorApp {
    fn drop(&mut self) {
        if let Some(render_state) = self.wgpu_render_state.as_ref() {
            self.viewport_renderer.free(render_state);
        }

        self.sync_config_from_runtime();
        if let Err(error) = self.config.save() {
            log::warn!(
                target: "engine::editor",
                "Failed to persist editor config on shutdown: {}",
                error
            );
        }
    }
}

struct EditorTabViewer<'a> {
    app: &'a mut EditorApp,
}

impl TabViewer for EditorTabViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Viewport => self.app.show_viewport_panel(ui),
            Tab::SceneTree => self.app.show_scene_tree_panel(ui),
            Tab::Inspector => self.app.show_inspector_panel(ui),
            Tab::AssetBrowser => self.app.show_asset_browser_panel(ui),
            Tab::Console => self.app.show_console_panel(ui),
        }
    }
}

fn build_editor_world() -> World {
    let mut world = create_world();

    let mut type_registry = ReflectTypeRegistry::default();
    let mut component_registry = ComponentRegistry::default();
    let mut metadata_registry = ReflectMetadataRegistry::default();

    register_core_reflection_types(
        &mut type_registry,
        &mut component_registry,
        &mut metadata_registry,
    );
    register_physics_reflection_types(
        &mut type_registry,
        &mut component_registry,
        &mut metadata_registry,
    );

    world.insert_resource(type_registry);
    world.insert_resource(component_registry);
    world.insert_resource(metadata_registry);

    world
}
