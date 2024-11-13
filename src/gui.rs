use eframe::egui::{ CentralPanel, Context, ScrollArea, SidePanel, TopBottomPanel, Visuals};

pub struct MyApp {
    pub dark_mode: bool,
    pub counter: i32,
    pub input_text: String,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            dark_mode: false,
            counter: 0,
            input_text: String::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.set_theme(ctx);

        self.show_top_panel(ctx);
        self.show_side_panel(ctx);
        self.show_central_panel(ctx);
        self.show_bottom_panel(ctx);
    }
}

impl MyApp {
    fn set_theme(&self, ctx: &Context) {
        if self.dark_mode {
            ctx.set_visuals(Visuals::dark());
        } else {
            ctx.set_visuals(Visuals::light());
        }
    }

    fn show_top_panel(&mut self, ctx: &Context) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Modo Claro/Escuro").clicked() {
                    self.dark_mode = !self.dark_mode;
                }
                ui.label("Mudança de Tema");
            });
        });
    }

    fn show_side_panel(&mut self, ctx: &Context) {
        SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Menu");
            if ui.button("Incrementar Contador").clicked() {
                self.counter += 1;
            }
            if ui.button("Decrementar Contador").clicked() {
                self.counter -= 1;
            }
            ui.label(format!("Contador: {}", self.counter));
        });
    }

    fn show_central_panel(&mut self, ctx: &Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Central Panel");
            ui.label("Digite algo:");
            ui.text_edit_singleline(&mut self.input_text);
            ui.label(format!("Você digitou: {}", self.input_text));
        });
    }

    fn show_bottom_panel(&self, ctx: &Context) {
        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ScrollArea::horizontal().show(ui, |ui| {
                ui.label("Rodapé com informações adicionais.");
            });
        });
    }
}
