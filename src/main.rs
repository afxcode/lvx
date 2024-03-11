#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use chrono::prelude::{DateTime, Local};
use eframe::egui;
use serde::{Deserialize, Serialize};

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        follow_system_theme: false,
        default_theme: eframe::Theme::Dark,
        viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "LVX - Log Viewer",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Box::<MyApp>::default()
        }),
    )
}

struct MyApp {
    picked_path: Option<String>,
    logs: Vec<Log>,
    filtered_logs: Vec<Log>,
    filter: String,
    selection: std::collections::HashSet<usize>,
}

impl MyApp {
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
                                level: json_line.level,
                                message: json_line.msg,
                                caller: json_line.caller,
                                payload: payload.to_string(),
                            });
                        }
                    }
                }
            }
            self.filter = "".to_string();
            self.filtered_logs = self.logs.clone()
        }
    }
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            picked_path: None,
            logs: vec![],
            filtered_logs: vec![],
            filter: "".to_string(),
            selection: Default::default(),
        }
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

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.picked_path = Some(path.display().to_string());
                        self.read_file()
                    }
                }

                if ui.button("Reload file…").clicked() {
                    self.read_file()
                }
            });

            if let Some(picked_path) = &self.picked_path {
                ui.monospace(picked_path);
            }

            ui.separator();

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                ui.label("Filter");

                if ui.text_edit_singleline(&mut self.filter).changed() {
                    self.filtered_logs = self.logs.iter()
                        .filter(|row| row.message.contains(&self.filter) || row.payload.contains(&self.filter))
                        .cloned()
                        .collect::<Vec<_>>();
                }
            });

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
                                .min_scrolled_height(0.0);

                            table = table.sense(egui::Sense::click());

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
                                        ui.strong("Data");
                                    });
                                    header.col(|ui| {
                                        ui.strong("Caller");
                                    });
                                })
                                .body(|body| {
                                    body.rows(text_height, self.filtered_logs.len(), |mut row| {
                                        let row_index = row.index();
                                        row.set_selected(self.selection.contains(&row_index));

                                        row.col(|ui| {
                                            ui.label(self.filtered_logs[row_index].time.to_rfc3339());
                                        });
                                        row.col(|ui| {
                                            let level = self.filtered_logs[row_index].level.to_string();
                                            if level == "DEBUG" {
                                                ui.colored_label(egui::Color32::from_rgb(10, 10, 240), level);
                                            } else if level == "INFO" {
                                                ui.colored_label(egui::Color32::from_rgb(10, 240, 10), level);
                                            } else if level == "WARN" {
                                                ui.colored_label(egui::Color32::from_rgb(240, 240, 10), level);
                                            } else if level == "ERROR" {
                                                ui.colored_label(egui::Color32::from_rgb(240, 60, 10), level);
                                            } else if level == "PANIC" {
                                                ui.colored_label(egui::Color32::from_rgb(240, 10, 10), level);
                                            }
                                        });
                                        row.col(|ui| {
                                            ui.label(self.filtered_logs[row_index].message.to_string());
                                        });
                                        row.col(|ui| {
                                            ui.label(self.filtered_logs[row_index].payload.to_string());
                                        });
                                        row.col(|ui| { ui.label(self.filtered_logs[row_index].caller.to_string()); });

                                        self.toggle_row_selection(row_index, &row.response());
                                    });
                                })
                        });
                    });
                });
        });
    }
}

#[derive(Clone)]
struct Log {
    time: DateTime<Local>,
    level: String,
    message: String,
    caller: String,
    payload: String,
}

impl Log {
    fn default() -> Self {
        Self {
            time: Default::default(),
            level: "".to_string(),
            message: "".to_string(),
            caller: "".to_string(),
            payload: "".to_string(),
        }
    }

    fn time_from_string(time_string: String) -> DateTime<Local> {
        return match DateTime::parse_from_str(&time_string, "%Y-%m-%dT%H:%M:%S%.3f%z") {
            Ok(ts) => {
                ts.with_timezone(&Local)
            }

            _ => { Default::default() }
        };
    }
}
