use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy_ecs::entity::Entity;
use bevy_ecs::system::RunSystemOnce;
use bevy_ecs::world::World;
use eframe::egui;
use egui_dock::{DockArea, DockState, TabViewer};
use engine_assets::{AssetServer, SceneDeserializer, SceneSerializer};
use engine_core::{
    create_world, register_core_reflection_types, Camera2d, Camera3d, Children, EditorEntityBundle,
    EntityName, GlobalTransform, Parent, PrimaryCamera, RenderLayer2D, RenderLayer3D, Result,
    SpatialBundle, Transform, Visible,
};
use engine_physics::register_physics_reflection_types;
use engine_reflect::{ComponentRegistry, ReflectMetadataRegistry, ReflectTypeRegistry};

use crate::commands::{
    CommandHistory, DeleteEntityCommand, DuplicateEntityCommand, RenameEntityCommand,
    ReparentEntityCommand, SpawnEntityCommand,
};
use crate::config::EditorConfig;
use crate::inspector::InspectorPanel;
use crate::layout::{create_default_layout, Tab};
use crate::selection::Selection;
use crate::viewport::{MeshRenderable3d, RenderSceneAdapter, SpriteRenderable2d, ViewportRenderer};

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

enum SceneTreeAction {
    AddRootEntity,
    AddChildEntity(Entity),
    Reparent {
        entity: Entity,
        new_parent: Option<Entity>,
    },
    BeginRename(Entity),
    CommitRename,
    CancelRename,
    Duplicate(Entity),
    Delete(Entity),
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
    fn push(
        &mut self,
        level: LogLevel,
        message: impl Into<String>,
        module: impl Into<String>,
        timestamp: f64,
    ) {
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
                for entry in self
                    .entries
                    .iter()
                    .filter(|entry| entry.level >= self.filter)
                {
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
                            entry.timestamp, entry.module, entry.message
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
    selection: Selection,
    command_history: CommandHistory,
    scene_filter_query: String,
    renaming_entity: Option<Entity>,
    rename_buffer: String,
    rename_focus_pending: bool,
    dragging_entity: Option<Entity>,
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
        console.push(LogLevel::Info, "Editor initialized", "engine::editor", 0.0);

        let mut app = Self {
            dock_state: config.dock_state.clone(),
            world,
            asset_server: AssetServer::new("assets"),
            file_path: None,
            unsaved_changes: false,
            show_about: false,
            config,
            selection: Selection::default(),
            command_history: CommandHistory::new(100),
            scene_filter_query: String::new(),
            renaming_entity: None,
            rename_buffer: String::new(),
            rename_focus_pending: false,
            dragging_entity: None,
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
                        format!(
                            "Failed to restore last scene {}: {}",
                            last_scene.display(),
                            error
                        ),
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

    fn apply_selection_hint(&mut self, entity: Option<Entity>) {
        if let Some(entity) = entity {
            if self.world.get_entity(entity).is_ok() {
                self.selection.select_single(entity);
                return;
            }
        }

        self.selection.deselect();
    }

    fn validate_selection_state(&mut self) {
        if let Some(entity) = self.selection.primary() {
            if self.world.get_entity(entity).is_err() {
                self.selection.deselect();
            }
        }

        if let Some(entity) = self.renaming_entity {
            if self.world.get_entity(entity).is_err() {
                self.cancel_rename_entity();
            }
        }
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
        let entities: Vec<Entity> = self
            .world
            .iter_entities()
            .map(|entity| entity.id())
            .collect();

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
        let has_primary_camera = self.world.iter_entities().any(|entity| {
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
        self.log_message(
            LogLevel::Info,
            "Inserted default editor camera for viewport rendering",
        );
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

        let texture = match self
            .asset_server
            .load_texture_handle("textures/placeholder.png")
        {
            Ok(handle) => handle,
            Err(error) => {
                self.log_message(
                    LogLevel::Warn,
                    format!(
                        "Preview texture load failed (textures/placeholder.png): {}",
                        error
                    ),
                );
                return;
            }
        };

        let material = match self
            .asset_server
            .load_material_handle("materials/default.ron")
        {
            Ok(handle) => handle,
            Err(error) => {
                self.log_message(
                    LogLevel::Warn,
                    format!(
                        "Preview material load failed (materials/default.ron): {}",
                        error
                    ),
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
        if let Err(error) = self
            .world
            .run_system_once(engine_core::propagate_transforms)
        {
            self.log_message(
                LogLevel::Warn,
                format!(
                    "Transform propagation failed before viewport render: {}",
                    error
                ),
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
                            self.log_message(
                                LogLevel::Error,
                                format!("New scene failed: {}", error),
                            );
                        }
                    }

                    if ui.button("Open Scene...      Ctrl+O").clicked() {
                        if let Err(error) = self.cmd_open_scene() {
                            self.log_message(
                                LogLevel::Error,
                                format!("Open scene failed: {}", error),
                            );
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
                        .add_enabled(
                            self.file_path.is_some(),
                            egui::Button::new("Save            Ctrl+S"),
                        )
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
                    if ui
                        .add_enabled(
                            self.command_history.can_undo(),
                            egui::Button::new("Undo    Ctrl+Z"),
                        )
                        .clicked()
                    {
                        let selection_hint = self.command_history.undo(&mut self.world);
                        self.apply_selection_hint(selection_hint);
                        self.unsaved_changes = true;
                    }

                    if ui
                        .add_enabled(
                            self.command_history.can_redo(),
                            egui::Button::new("Redo    Ctrl+Y"),
                        )
                        .clicked()
                    {
                        let selection_hint = self.command_history.redo(&mut self.world);
                        self.apply_selection_hint(selection_hint);
                        self.unsaved_changes = true;
                    }

                    ui.separator();
                    if ui
                        .add_enabled(
                            self.selection.has_selection(),
                            egui::Button::new("Delete    Del"),
                        )
                        .clicked()
                    {
                        self.cmd_delete_selected();
                    }

                    if ui
                        .add_enabled(
                            self.selection.has_selection(),
                            egui::Button::new("Duplicate    Ctrl+D"),
                        )
                        .clicked()
                    {
                        self.cmd_duplicate_selected();
                    }
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
        let mut undo = false;
        let mut redo = false;
        let mut delete_selected = false;
        let mut duplicate_selected = false;
        let mut begin_rename = false;
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

            undo = input.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::Z,
            ));

            redo = input.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::Y,
            ));

            duplicate_selected = input.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::D,
            ));

            delete_selected = input.key_pressed(egui::Key::Delete);
            begin_rename = input.key_pressed(egui::Key::F2);

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

        if undo {
            let selection_hint = self.command_history.undo(&mut self.world);
            self.apply_selection_hint(selection_hint);
            self.unsaved_changes = true;
        }

        if redo {
            let selection_hint = self.command_history.redo(&mut self.world);
            self.apply_selection_hint(selection_hint);
            self.unsaved_changes = true;
        }

        if duplicate_selected {
            self.cmd_duplicate_selected();
        }

        if delete_selected {
            self.cmd_delete_selected();
        }

        if begin_rename {
            if let Some(entity) = self.selection.primary() {
                self.begin_rename_entity(entity);
            }
        }

        if clear_selection {
            if self.renaming_entity.is_some() {
                self.cancel_rename_entity();
            } else {
                self.selection.deselect();
            }
        }
    }

    fn cmd_delete_selected(&mut self) {
        let Some(entity) = self.selection.primary() else {
            return;
        };

        self.cmd_delete_entity(entity);
    }

    fn cmd_delete_entity(&mut self, entity: Entity) {
        let selection_hint = self
            .command_history
            .execute(Box::new(DeleteEntityCommand::new(entity)), &mut self.world);

        self.apply_selection_hint(selection_hint);
        self.unsaved_changes = true;
        self.log_message(LogLevel::Info, "Deleted selected entity");
    }

    fn cmd_duplicate_selected(&mut self) {
        let Some(entity) = self.selection.primary() else {
            return;
        };

        self.cmd_duplicate_entity(entity);
    }

    fn cmd_duplicate_entity(&mut self, entity: Entity) {
        let selection_hint = self.command_history.execute(
            Box::new(DuplicateEntityCommand::new(entity)),
            &mut self.world,
        );

        self.apply_selection_hint(selection_hint);
        self.unsaved_changes = true;
        self.log_message(LogLevel::Info, "Duplicated selected entity");
    }

    fn cmd_add_root_entity(&mut self) {
        let selection_hint = self
            .command_history
            .execute(Box::new(SpawnEntityCommand::new_root()), &mut self.world);

        self.apply_selection_hint(selection_hint);
        self.unsaved_changes = true;
        self.log_message(LogLevel::Info, "Added root entity");
    }

    fn cmd_add_child_entity(&mut self, parent: Entity) {
        if self.world.get_entity(parent).is_err() {
            self.log_message(
                LogLevel::Warn,
                format!(
                    "Cannot add child entity: parent {:?} does not exist",
                    parent
                ),
            );
            return;
        }

        let selection_hint = self.command_history.execute(
            Box::new(SpawnEntityCommand::new_child(parent)),
            &mut self.world,
        );

        self.apply_selection_hint(selection_hint);
        self.unsaved_changes = true;
        self.log_message(LogLevel::Info, "Added child entity");
    }

    fn cmd_reparent_entity(&mut self, entity: Entity, new_parent: Option<Entity>) {
        if self.world.get_entity(entity).is_err() {
            self.log_message(
                LogLevel::Warn,
                format!("Cannot reparent entity {:?}: entity does not exist", entity),
            );
            return;
        }

        if let Some(parent) = new_parent {
            if parent == entity {
                self.log_message(
                    LogLevel::Warn,
                    format!(
                        "Cannot reparent entity {:?}: entity cannot be parent of itself",
                        entity
                    ),
                );
                return;
            }

            if self.world.get_entity(parent).is_err() {
                self.log_message(
                    LogLevel::Warn,
                    format!(
                        "Cannot reparent entity {:?}: parent {:?} does not exist",
                        entity, parent
                    ),
                );
                return;
            }

            if self.scene_tree_is_descendant(parent, entity) {
                self.log_message(
                    LogLevel::Warn,
                    format!(
                        "Cannot reparent entity {:?}: target parent {:?} is in its descendant chain",
                        entity,
                        parent
                    ),
                );
                return;
            }
        }

        if self.world.get::<Parent>(entity).map(|parent| parent.0) == new_parent {
            return;
        }

        let selection_hint = self.command_history.execute(
            Box::new(ReparentEntityCommand::new(entity, new_parent)),
            &mut self.world,
        );

        self.apply_selection_hint(selection_hint);
        self.unsaved_changes = true;

        if let Some(parent) = new_parent {
            self.log_message(
                LogLevel::Info,
                format!("Reparented entity {:?} under {:?}", entity, parent),
            );
        } else {
            self.log_message(LogLevel::Info, format!("Moved entity {:?} to root", entity));
        }
    }

    fn scene_tree_is_descendant(&self, candidate: Entity, ancestor: Entity) -> bool {
        is_scene_tree_descendant(&self.world, candidate, ancestor)
    }

    fn begin_rename_entity(&mut self, entity: Entity) {
        let current_name = self
            .world
            .get::<EntityName>(entity)
            .map(|value| value.0.clone())
            .unwrap_or_else(|| "Entity".to_owned());

        self.renaming_entity = Some(entity);
        self.rename_buffer = current_name;
        self.rename_focus_pending = true;
    }

    fn commit_rename_entity(&mut self) {
        let Some(entity) = self.renaming_entity else {
            return;
        };

        let new_name = self.rename_buffer.trim().to_owned();
        if new_name.is_empty() {
            self.renaming_entity = None;
            self.rename_buffer.clear();
            self.rename_focus_pending = false;
            return;
        }

        let old_name = self
            .world
            .get::<EntityName>(entity)
            .map(|value| value.0.clone())
            .unwrap_or_else(|| "Entity".to_owned());

        if old_name == new_name {
            self.renaming_entity = None;
            self.rename_buffer.clear();
            self.rename_focus_pending = false;
            return;
        }

        let selection_hint = self.command_history.execute(
            Box::new(RenameEntityCommand::new(entity, old_name, new_name)),
            &mut self.world,
        );

        self.apply_selection_hint(selection_hint);
        self.unsaved_changes = true;
        self.renaming_entity = None;
        self.rename_buffer.clear();
        self.rename_focus_pending = false;
        self.log_message(LogLevel::Info, "Renamed entity");
    }

    fn cancel_rename_entity(&mut self) {
        self.renaming_entity = None;
        self.rename_buffer.clear();
        self.rename_focus_pending = false;
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
        egui::Window::new("About Starman Editor")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Starman Editor (EP-09 foundation)");
                ui.label("Rust + egui + wgpu");
            });

        self.show_about = open;
    }

    fn cmd_new_scene(&mut self) -> Result<()> {
        self.world.clear_entities();
        self.world.spawn(EditorEntityBundle::default());
        self.bootstrap_viewport_scene();
        self.file_path = None;
        self.selection.deselect();
        self.cancel_rename_entity();
        self.scene_filter_query.clear();
        self.dragging_entity = None;
        self.command_history.clear();
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

        self.with_scene_context(
            |world, component_registry, type_registry, metadata_registry, asset_server| {
                let serializer = SceneSerializer::new(world, component_registry, type_registry)
                    .with_metadata_registry(metadata_registry)
                    .with_asset_server(asset_server)
                    .with_external_components(&render_scene_adapter);
                serializer.save_file(path, scene_name)?;
                Ok(())
            },
        )
    }

    fn load_scene(&mut self, path: &Path) -> Result<()> {
        let render_scene_adapter = RenderSceneAdapter;

        self.with_scene_context(
            |world, component_registry, type_registry, _metadata_registry, asset_server| {
                world.clear_entities();
                let mut deserializer =
                    SceneDeserializer::new(world, component_registry, type_registry, asset_server)
                        .with_external_components(&render_scene_adapter);
                let _ = deserializer.load_file(path)?;
                Ok(())
            },
        )?;

        self.bootstrap_viewport_scene();
        self.selection.deselect();
        self.cancel_rename_entity();
        self.scene_filter_query.clear();
        self.dragging_entity = None;
        self.command_history.clear();

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
            let sized_texture =
                egui::load::SizedTexture::new(texture_id, egui::vec2(width as f32, height as f32));
            let _ = ui.add(egui::Image::new(sized_texture).sense(egui::Sense::click_and_drag()));
        }

        if let Some(error) = self.viewport_renderer.last_error() {
            ui.colored_label(
                egui::Color32::RED,
                format!("Viewport render error: {}", error),
            );
        } else {
            ui.label("Viewport rendering via editor offscreen backend.");
        }
    }

    fn show_scene_tree_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Scene Hierarchy");
        ui.separator();

        let mut pending_action = None;

        ui.horizontal(|ui| {
            if ui.button("+").on_hover_text("Add Entity").clicked() {
                pending_action = Some(SceneTreeAction::AddRootEntity);
            }

            ui.separator();
            ui.label("Search:");
            let _ = ui.add(
                egui::TextEdit::singleline(&mut self.scene_filter_query)
                    .desired_width(200.0)
                    .hint_text("Filter entities..."),
            );
        });

        ui.separator();

        let roots = collect_scene_tree_roots(&self.world);

        let filter_query = self.scene_filter_query.trim().to_ascii_lowercase();
        let visibility = if filter_query.is_empty() {
            None
        } else {
            Some(self.build_scene_tree_visibility(&roots, &filter_query))
        };

        let mut root_drop_hovered = false;

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut visited = HashSet::new();
            for entity in roots.iter().copied() {
                self.draw_entity_node(
                    ui,
                    entity,
                    0,
                    visibility.as_ref(),
                    &mut visited,
                    &mut pending_action,
                );
            }

            let available = ui.available_size_before_wrap();
            let drop_area = ui.allocate_response(
                egui::vec2(available.x.max(0.0), available.y.max(24.0)),
                egui::Sense::hover(),
            );
            root_drop_hovered = drop_area.hovered();

            if self.dragging_entity.is_some() && root_drop_hovered {
                let color = ui.visuals().selection.bg_fill.gamma_multiply(0.20);
                ui.painter().rect_filled(drop_area.rect, 4.0, color);
                ui.painter().text(
                    drop_area.rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Drop here to move to root",
                    egui::FontId::default(),
                    ui.visuals().strong_text_color(),
                );
            }
        });

        let pointer_released = ui.input(|input| input.pointer.any_released());
        if pointer_released {
            if let Some(dragging_entity) = self.dragging_entity {
                let node_drop_already_selected =
                    matches!(pending_action, Some(SceneTreeAction::Reparent { .. }));
                if !node_drop_already_selected && root_drop_hovered {
                    pending_action = Some(SceneTreeAction::Reparent {
                        entity: dragging_entity,
                        new_parent: None,
                    });
                }

                self.dragging_entity = None;
            }
        }

        if let Some(action) = pending_action {
            match action {
                SceneTreeAction::AddRootEntity => {
                    self.cmd_add_root_entity();
                }
                SceneTreeAction::AddChildEntity(parent) => {
                    self.cmd_add_child_entity(parent);
                }
                SceneTreeAction::Reparent { entity, new_parent } => {
                    self.cmd_reparent_entity(entity, new_parent);
                }
                SceneTreeAction::BeginRename(entity) => {
                    self.begin_rename_entity(entity);
                }
                SceneTreeAction::CommitRename => {
                    self.commit_rename_entity();
                }
                SceneTreeAction::CancelRename => {
                    self.cancel_rename_entity();
                }
                SceneTreeAction::Duplicate(entity) => {
                    self.selection.select_single(entity);
                    self.cmd_duplicate_entity(entity);
                }
                SceneTreeAction::Delete(entity) => {
                    self.selection.select_single(entity);
                    self.cmd_delete_entity(entity);
                }
            }
        }
    }

    fn draw_entity_node(
        &mut self,
        ui: &mut egui::Ui,
        entity: Entity,
        depth: usize,
        visibility: Option<&HashMap<Entity, bool>>,
        visited: &mut HashSet<Entity>,
        pending_action: &mut Option<SceneTreeAction>,
    ) {
        if let Some(visibility) = visibility {
            if !visibility.get(&entity).copied().unwrap_or(false) {
                return;
            }
        }

        let indent = depth as f32 * 14.0;

        let name = self.entity_name_for_scene_tree(entity);

        if !visited.insert(entity) {
            ui.horizontal(|ui| {
                ui.add_space(indent);
                ui.colored_label(egui::Color32::YELLOW, format!("{} (cycle)", name));
            });
            return;
        }

        let is_selected = self.selection.primary() == Some(entity);
        ui.horizontal(|ui| {
            ui.add_space(indent);

            if self.renaming_entity == Some(entity) {
                let response = ui
                    .add(egui::TextEdit::singleline(&mut self.rename_buffer).desired_width(180.0));

                if self.rename_focus_pending {
                    response.request_focus();
                    self.rename_focus_pending = false;
                }

                if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                    *pending_action = Some(SceneTreeAction::CommitRename);
                }

                if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                    *pending_action = Some(SceneTreeAction::CancelRename);
                }

                return;
            }

            let response = ui.selectable_label(is_selected, name);

            if response.clicked() {
                self.selection.select_single(entity);
            }

            if response.drag_started() {
                self.dragging_entity = Some(entity);
            }

            if let Some(dragging_entity) = self.dragging_entity {
                let pointer_released = ui.input(|input| input.pointer.any_released());
                let is_drop_target = dragging_entity != entity && response.hovered();

                if is_drop_target {
                    let color = ui.visuals().selection.bg_fill.gamma_multiply(0.20);
                    ui.painter()
                        .rect_filled(response.rect.expand(1.0), 2.0, color);
                }

                if pointer_released && is_drop_target {
                    *pending_action = Some(SceneTreeAction::Reparent {
                        entity: dragging_entity,
                        new_parent: Some(entity),
                    });
                }
            }

            if response.double_clicked() {
                *pending_action = Some(SceneTreeAction::BeginRename(entity));
            }

            response.context_menu(|ui| {
                if ui.button("Add Child Entity").clicked() {
                    *pending_action = Some(SceneTreeAction::AddChildEntity(entity));
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Rename").clicked() {
                    *pending_action = Some(SceneTreeAction::BeginRename(entity));
                    ui.close_menu();
                }

                if ui.button("Duplicate").clicked() {
                    *pending_action = Some(SceneTreeAction::Duplicate(entity));
                    ui.close_menu();
                }

                if ui.button("Delete").clicked() {
                    *pending_action = Some(SceneTreeAction::Delete(entity));
                    ui.close_menu();
                }
            });
        });

        if self.renaming_entity == Some(entity) {
            visited.remove(&entity);
            return;
        }

        let children = self
            .world
            .get::<Children>(entity)
            .map(|children| children.0.clone())
            .unwrap_or_default();

        for child in children {
            self.draw_entity_node(ui, child, depth + 1, visibility, visited, pending_action);
        }

        visited.remove(&entity);
    }

    fn build_scene_tree_visibility(&self, roots: &[Entity], query: &str) -> HashMap<Entity, bool> {
        build_scene_tree_visibility_map(&self.world, roots, query)
    }

    fn entity_name_for_scene_tree(&self, entity: Entity) -> String {
        scene_tree_entity_name(&self.world, entity)
    }

    fn show_inspector_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Inspector");
        ui.separator();
        if InspectorPanel::show(
            ui,
            &mut self.world,
            &self.selection,
            &mut self.command_history,
        ) {
            self.unsaved_changes = true;
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
        self.validate_selection_state();
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

pub(crate) fn collect_scene_tree_roots(world: &World) -> Vec<Entity> {
    world
        .iter_entities()
        .filter_map(|entity_ref| {
            let entity = entity_ref.id();
            match world.get::<Parent>(entity) {
                Some(parent) if world.get_entity(parent.0).is_ok() => None,
                _ => Some(entity),
            }
        })
        .collect()
}

pub(crate) fn build_scene_tree_visibility_map(
    world: &World,
    roots: &[Entity],
    query: &str,
) -> HashMap<Entity, bool> {
    let mut visibility = HashMap::new();
    let mut stack = HashSet::new();

    for root in roots {
        let _ = compute_scene_tree_visibility_map(world, *root, query, &mut visibility, &mut stack);
    }

    visibility
}

fn compute_scene_tree_visibility_map(
    world: &World,
    entity: Entity,
    query: &str,
    visibility: &mut HashMap<Entity, bool>,
    stack: &mut HashSet<Entity>,
) -> bool {
    if let Some(existing) = visibility.get(&entity) {
        return *existing;
    }

    if !stack.insert(entity) {
        visibility.insert(entity, false);
        return false;
    }

    let mut is_visible = scene_tree_entity_name(world, entity)
        .to_ascii_lowercase()
        .contains(query);

    let children = world
        .get::<Children>(entity)
        .map(|children| children.0.clone())
        .unwrap_or_default();

    for child in children {
        if compute_scene_tree_visibility_map(world, child, query, visibility, stack) {
            is_visible = true;
        }
    }

    stack.remove(&entity);
    visibility.insert(entity, is_visible);
    is_visible
}

pub(crate) fn scene_tree_entity_name(world: &World, entity: Entity) -> String {
    world
        .get::<EntityName>(entity)
        .map(|value| value.0.clone())
        .unwrap_or_else(|| format!("Entity {:?}", entity))
}

pub(crate) fn is_scene_tree_descendant(world: &World, candidate: Entity, ancestor: Entity) -> bool {
    let mut visited = HashSet::new();
    let mut current = Some(candidate);

    while let Some(entity) = current {
        if !visited.insert(entity) {
            break;
        }

        if entity == ancestor {
            return true;
        }

        current = world.get::<Parent>(entity).map(|parent| parent.0);
    }

    false
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
