#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide a console window on Windows in release

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use chrono::prelude::{DateTime, Local};
use eframe::egui;
use serde::{Deserialize, Serialize};

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "LVX - Log Viewer",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<App>::default())
        }),
    )
}

struct App {
    picked_path: Option<String>,
    logs: Vec<Log>,
    filtered_logs: Vec<Log>,
    filter_level_debug: bool,
    filter_level_info: bool,
    filter_level_warning: bool,
    filter_level_error: bool,
    filter_level_panic: bool,
    filter_message: String,
    filter_payload: String,
    filter_caller: String,
    search_founds: Vec<usize>,
    search_found_cursor: usize,
    search_found_scroll_row: Option<usize>,
    search_level_debug: bool,
    search_level_info: bool,
    search_level_warning: bool,
    search_level_error: bool,
    search_level_panic: bool,
    search_message: String,
    search_payload: String,
    search_caller: String,
    selection: std::collections::HashSet<usize>,
}


impl Default for App {
    fn default() -> Self {
        Self {
            picked_path: None,
            logs: vec![],
            filtered_logs: vec![],
            filter_level_debug: true,
            filter_level_info: true,
            filter_level_warning: true,
            filter_level_error: true,
            filter_level_panic: true,
            filter_message: "".to_string(),
            filter_payload: "".to_string(),
            filter_caller: "".to_string(),
            search_founds: vec![],
            search_found_cursor: 0,
            search_found_scroll_row: None,
            search_level_debug: false,
            search_level_info: false,
            search_level_warning: false,
            search_level_error: false,
            search_level_panic: false,
            search_message: "".to_string(),
            search_payload: "".to_string(),
            search_caller: "".to_string(),
            selection: Default::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        catppuccin_egui::set_theme(&ctx, catppuccin_egui::MACCHIATO);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                if ui.button("📂 Open").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.picked_path = Some(path.display().to_string());
                        self.read_file()
                    }
                }

                if let Some(picked_path) = &self.picked_path.clone() {
                    if ui.button("↺ Reload").clicked() {
                        self.read_file();
                    }
                    ui.horizontal(|ui| {
                        ui.label("File:");
                        ui.monospace(picked_path);
                    });
                }
            });

            if let Some(_picked_path) = &self.picked_path {
                ui.horizontal(|ui| {
                    ui.menu_button("🔍", |ui| {
                        ui.vertical(|ui| {
                            egui::Grid::new("filter_grid")
                                .num_columns(2)
                                .spacing([40.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.strong("Filter");
                                    ui.horizontal(|ui| {
                                        if ui.button("✖").on_hover_text("Reset").clicked() {
                                            self.filter_reset();
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Level");
                                    ui.horizontal(|ui| {
                                        if ui.selectable_label(self.filter_level_debug, "DEBUG").clicked() {
                                            self.filter_level_debug = !self.filter_level_debug;
                                            self.filter();
                                        }
                                        if ui.selectable_label(self.filter_level_info, "INFO").clicked() {
                                            self.filter_level_info = !self.filter_level_info;
                                            self.filter();
                                        }
                                        if ui.selectable_label(self.filter_level_warning, "WARNING").clicked() {
                                            self.filter_level_warning = !self.filter_level_warning;
                                            self.filter();
                                        }
                                        if ui.selectable_label(self.filter_level_error, "ERROR").clicked() {
                                            self.filter_level_error = !self.filter_level_error;
                                            self.filter();
                                        }
                                        if ui.selectable_label(self.filter_level_panic, "PANIC").clicked() {
                                            self.filter_level_panic = !self.filter_level_panic;
                                            self.filter();
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Message");
                                    if ui.text_edit_singleline(&mut self.filter_message).changed() {
                                        self.filter();
                                    }
                                    ui.end_row();

                                    ui.label("Payload");
                                    if ui.text_edit_singleline(&mut self.filter_payload).changed() {
                                        self.filter();
                                    }
                                    ui.end_row();

                                    ui.label("Caller");
                                    if ui.text_edit_singleline(&mut self.filter_caller).changed() {
                                        self.filter();
                                    }
                                    ui.end_row();
                                });

                            ui.separator();
                            egui::Grid::new("search_grid")
                                .num_columns(2)
                                .spacing([40.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.strong("Search");
                                    ui.horizontal(|ui| {
                                        if ui.button("🔝").on_hover_text("First").clicked() {
                                            self.search_first();
                                        }
                                        if ui.button("⬅").on_hover_text("Previous").clicked() {
                                            self.search_previous();
                                        }
                                        if ui.button("➡").on_hover_text("Next").clicked() {
                                            self.search_next();
                                        }
                                        if ui.button("🔚").on_hover_text("Last").clicked() {
                                            self.search_last();
                                        }
                                        if ui.button("✖").on_hover_text("Reset").clicked() {
                                            self.search_reset();
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Level");
                                    ui.horizontal(|ui| {
                                        if ui.selectable_label(self.search_level_debug, "DEBUG").clicked() {
                                            self.search_level_debug = !self.search_level_debug;
                                            self.search();
                                        }
                                        if ui.selectable_label(self.search_level_info, "INFO").clicked() {
                                            self.search_level_info = !self.search_level_info;
                                            self.search();
                                        }
                                        if ui.selectable_label(self.search_level_warning, "WARNING").clicked() {
                                            self.search_level_warning = !self.search_level_warning;
                                            self.search();
                                        }
                                        if ui.selectable_label(self.search_level_error, "ERROR").clicked() {
                                            self.search_level_error = !self.search_level_error;
                                            self.search();
                                        }
                                        if ui.selectable_label(self.search_level_panic, "PANIC").clicked() {
                                            self.search_level_panic = !self.search_level_panic;
                                            self.search();
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Message");
                                    if ui.text_edit_singleline(&mut self.search_message).changed() {
                                        self.search();
                                    }
                                    ui.end_row();

                                    ui.label("Payload");
                                    if ui.text_edit_singleline(&mut self.search_payload).changed() {
                                        self.search();
                                    }
                                    ui.end_row();

                                    ui.label("Caller");
                                    if ui.text_edit_singleline(&mut self.search_caller).changed() {
                                        self.search();
                                    }
                                    ui.end_row();
                                });
                        });
                    }).response.on_hover_text("Filter & Search");

                    ui.label("Filtered");
                    ui.monospace(self.filtered_logs.len().to_string());
                    ui.label("from total");
                    ui.monospace(self.logs.len().to_string());
                });
            }

            ui.separator();

            let body_text_size = egui::TextStyle::Body.resolve(ui.style()).size;
            use egui_extras::{Size, StripBuilder};
            StripBuilder::new(ui)
                .size(Size::remainder().at_least(0.0))
                .size(Size::exact(body_text_size))
                .vertical(|mut strip| {
                    strip.cell(|ui| {
                        egui::ScrollArea::horizontal().show(ui, |ui| {
                            use egui_extras::{Column, TableBuilder};

                            let text_height = egui::TextStyle::Body
                                .resolve(ui.style())
                                .size
                                .max(ui.spacing().interact_size.y);

                            let mut table = TableBuilder::new(ui)
                                .striped(true)
                                .resizable(true)
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                .column(Column::exact(200.0))
                                .column(Column::exact(50.0))
                                .column(Column::initial(100.00).at_least(100.0))
                                .column(Column::initial(100.00).at_least(100.0))
                                .column(Column::remainder())
                                .min_scrolled_height(0.0)
                                .max_scroll_height(2000.0);

                            table = table.sense(egui::Sense::click());

                            if let Some(row_index) = self.search_found_scroll_row.take() {
                                table = table.scroll_to_row(row_index, None);
                            }

                            table
                                .header(20.0, |mut header| {
                                    header.col(|ui| {
                                        ui.strong("Time");
                                    });
                                    header.col(|ui| {
                                        ui.strong("Level");
                                    });
                                    header.col(|ui| {
                                        ui.strong("Message");
                                    });
                                    header.col(|ui| {
                                        ui.strong("Payload");
                                    });
                                    header.col(|ui| {
                                        ui.strong("Caller");
                                    });
                                })
                                .body(|body| {
                                    body.rows(text_height, self.filtered_logs.len(), |mut row| {
                                        let row_index = row.index();
                                        row.set_selected(self.selection.contains(&row_index) || self.index_at_search_found_cursor(row_index));

                                        let found_on_search = self.search_founds.contains(&row_index);

                                        row.col(|ui| {
                                            let ts = self.filtered_logs[row_index].time.to_rfc3339();
                                            if found_on_search { ui.strong(ts); } else { ui.weak(ts); }
                                        });
                                        row.col(|ui| {
                                            let level = self.filtered_logs[row_index].level.clone();
                                            let mut color = egui::Color32::from_rgb(80, 80, 80);
                                            match level {
                                                Level::Debug => { color = egui::Color32::from_rgb(10, 10, 240); }
                                                Level::Info => { color = egui::Color32::from_rgb(10, 240, 10); }
                                                Level::Warning => { color = egui::Color32::from_rgb(240, 240, 10); }
                                                Level::Error => { color = egui::Color32::from_rgb(240, 60, 10); }
                                                Level::Panic => { color = egui::Color32::from_rgb(240, 10, 10); }
                                                _ => {}
                                            }
                                            ui.colored_label(color, level.to_string());
                                        });
                                        row.col(|ui| {
                                            let msg = self.filtered_logs[row_index].message.to_string();
                                            if found_on_search { ui.strong(msg); } else { ui.weak(msg); }
                                        });
                                        row.col(|ui| {
                                            let py = self.filtered_logs[row_index].payload.to_string();
                                            if found_on_search { ui.strong(py); } else { ui.weak(py); }
                                        });
                                        row.col(|ui| {
                                            let ca = self.filtered_logs[row_index].caller.to_string();
                                            if found_on_search { ui.strong(ca); } else { ui.weak(ca); }
                                        });

                                        self.toggle_row_selection(row_index, &row.response());
                                    });
                                })
                        });
                    });
                });
        });
    }
}


impl App {
    fn toggle_row_selection(&mut self, row_index: usize, row_response: &egui::Response) {
        if row_response.clicked() {
            if self.selection.contains(&row_index) {
                self.selection.remove(&row_index);
            } else {
                self.selection.insert(row_index);
            }
        }
    }

    fn read_file(&mut self) {
        self.logs.clear();
        if let Some(path) = &self.picked_path {
            let buffer = Box::new(BufReader::new(File::open(path.to_string()).unwrap()));
            for line in buffer.lines() {
                if let Ok(json_str) = line {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        if let Ok(json_line) = serde_json::from_value::<JsonLine>(value) {
                            let mut payload = String::from("");
                            if !json_line.payload.is_empty() {
                                let mut keys: Vec<_> = json_line.payload.keys().cloned().collect();
                                keys.sort();
                                let mut sorted = serde_json::json!({});
                                for key in keys {
                                    sorted[key.clone()] = json_line.payload[&key].clone();
                                }
                                payload = sorted.to_string()
                            }

                            self.logs.push(Log {
                                time: Log::time_from_string(json_line.ts),
                                level: Level::from_string(json_line.level.as_str()),
                                message: json_line.msg,
                                payload: payload.to_string(),
                                caller: json_line.caller,
                            });
                        }
                    }
                }
            }
            self.filter_reset();
        }
    }

    fn filter(&mut self) {
        self.filtered_logs = self.logs.iter()
            .filter(|row| {
                let mut level = row.level == Level::Unknown;
                level |= row.level == Level::Debug && self.filter_level_debug;
                level |= row.level == Level::Info && self.filter_level_info;
                level |= row.level == Level::Warning && self.filter_level_warning;
                level |= row.level == Level::Error && self.filter_level_error;
                level |= row.level == Level::Panic && self.filter_level_panic;
                let message = row.message.to_lowercase().contains(&self.filter_message.to_lowercase());
                let payload = row.payload.to_lowercase().contains(&self.filter_payload.to_lowercase());
                let caller = row.caller.to_lowercase().contains(&self.filter_caller.to_lowercase());
                level && message && payload && caller
            })
            .cloned()
            .collect::<Vec<_>>();
        self.search();
    }

    fn filter_reset(&mut self) {
        self.filter_level_debug = true;
        self.filter_level_info = true;
        self.filter_level_warning = true;
        self.filter_level_error = true;
        self.filter_level_panic = true;
        self.filter_message = "".to_string();
        self.filter_payload = "".to_string();
        self.filter_caller = "".to_string();
        self.filter();
    }

    fn search(&mut self) {
        if !self.search_level_debug
            && !self.search_level_info
            && !self.search_level_warning
            && !self.search_level_error
            && !self.search_level_panic
            && self.search_message.is_empty()
            && self.search_payload.is_empty()
            && self.search_caller.is_empty() {
            self.search_reset();
            return;
        }

        self.search_founds.clear();
        for (index, row) in self.filtered_logs.iter().enumerate() {
            let mut level = row.level == Level::Unknown;
            level |= row.level == Level::Debug && self.search_level_debug;
            level |= row.level == Level::Info && self.search_level_info;
            level |= row.level == Level::Warning && self.search_level_warning;
            level |= row.level == Level::Error && self.search_level_error;
            level |= row.level == Level::Panic && self.search_level_panic;
            let message = row.message.to_lowercase().contains(&self.search_message.to_lowercase());
            let payload = row.payload.to_lowercase().contains(&self.search_payload.to_lowercase());
            let caller = row.caller.to_lowercase().contains(&self.search_caller.to_lowercase());

            if level && message && payload && caller {
                self.search_founds.push(index)
            }
        }

        self.search_found_cursor = 0
    }

    fn index_at_search_found_cursor(&mut self, index: usize) -> bool {
        if self.search_founds.is_empty() {
            return false;
        }
        if self.search_found_cursor > self.search_founds.len() - 1 {
            return false;
        }
        return index == self.search_founds[self.search_found_cursor];
    }

    fn search_reset(&mut self) {
        self.search_level_debug = false;
        self.search_level_info = false;
        self.search_level_warning = false;
        self.search_level_error = false;
        self.search_level_panic = false;
        self.search_message = "".to_string();
        self.search_payload = "".to_string();
        self.search_caller = "".to_string();
        self.search_founds.clear();
    }

    fn search_first(&mut self) {
        self.search_found_cursor = 0;
        if self.search_founds.is_empty() {
            self.search_found_scroll_row = None;
            return;
        }
        self.search_found_scroll_row = self.search_founds.get(self.search_found_cursor).copied();
    }

    fn search_previous(&mut self) {
        if self.search_founds.is_empty() || self.search_found_cursor <= 0 {
            self.search_found_scroll_row = None;
            return;
        }
        self.search_found_cursor -= 1;
        self.search_found_scroll_row = self.search_founds.get(self.search_found_cursor).copied();
    }

    fn search_next(&mut self) {
        if self.search_founds.is_empty() || self.search_found_cursor >= self.search_founds.len() - 1 {
            self.search_found_scroll_row = None;
            return;
        }
        self.search_found_cursor += 1;
        self.search_found_scroll_row = self.search_founds.get(self.search_found_cursor).copied();
    }

    fn search_last(&mut self) {
        if self.search_founds.is_empty() {
            self.search_found_cursor = 0;
            self.search_found_scroll_row = None;
        }
        self.search_found_cursor = self.search_founds.len() - 1;
        self.search_found_scroll_row = self.search_founds.get(self.search_found_cursor).copied();
    }
}


#[derive(PartialEq, Clone)]
enum Level {
    Unknown,
    Debug,
    Info,
    Warning,
    Error,
    Panic,
}

impl Level {
    fn from_string(level: &str) -> Level {
        match level {
            "DEBUG" => Level::Debug,
            "INFO" => Level::Info,
            "WARN" => Level::Warning,
            "ERROR" => Level::Error,
            "PANIC" => Level::Panic,
            _ => Level::Unknown,
        }
    }
    fn to_string(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warning => "WARN",
            Level::Error => "ERROR",
            Level::Panic => "PANIC",
            _ => "N/A",
        }
    }
}


#[derive(Clone)]
struct Log {
    time: DateTime<Local>,
    level: Level,
    message: String,
    caller: String,
    payload: String,
}

impl Log {
    fn time_from_string(time_string: String) -> DateTime<Local> {
        return match DateTime::parse_from_str(&time_string, "%Y-%m-%dT%H:%M:%S%.3f%z") {
            Ok(ts) => {
                ts.with_timezone(&Local)
            }

            _ => { Default::default() }
        };
    }
}


#[derive(Serialize, Deserialize)]
struct JsonLine {
    level: String,
    ts: String,
    msg: String,
    #[serde(default)]
    caller: String,
    #[serde(flatten)]
    payload: HashMap<String, serde_json::Value>,
}