#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use chrono::prelude::{DateTime, Local};
use eframe::egui;
use serde::{Deserialize, Serialize};
use serde_json::Value;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "LV Log Viewer",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Box::<MyApp>::default()
        }),
    )
}

struct MyApp {
    picked_path: Option<String>,
    logs: Vec<Log>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            picked_path: None,
            logs: vec![],
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
    extra_fields: HashMap<String, Value>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Open file…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.picked_path = Some(path.display().to_string());
                    self.logs.clear();
                    let buffer = Box::new(BufReader::new(File::open(path.display().to_string()).unwrap()));
                    for line in buffer.lines() {
                        if let Ok(json_str) = line {
                            if let Ok(json_line) = serde_json::from_str::<JsonLine>(&json_str) {
                                self.logs.push(Log {
                                    time: Log::time_from_string(json_line.ts),
                                    level: json_line.level,
                                    message: json_line.msg,
                                    caller: json_line.caller,
                                })
                            }
                        }
                    }
                }
            }

            if let Some(picked_path) = &self.picked_path {
                ui.monospace(picked_path);
            }

            ui.separator();

            let body_text_size = egui::TextStyle::Body.resolve(ui.style()).size;
            use egui_extras::{Size, StripBuilder};
            StripBuilder::new(ui)
                .size(Size::remainder().at_least(100.0))
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
                                    body.rows(text_height, self.logs.len(), |mut row| {
                                        let row_index = row.index();

                                        row.col(|ui| {
                                            ui.label(self.logs[row_index].time.to_rfc3339());
                                        });
                                        row.col(|ui| {
                                            let level = self.logs[row_index].level.to_string();
                                            if level == "DEBUG" {
                                                ui.colored_label(egui::Color32::from_rgb(10, 10, 240), level);
                                            } else if level == "INFO" {
                                                ui.colored_label(egui::Color32::from_rgb(10, 240, 10), level);
                                            } else if level == "WARN" {
                                                ui.colored_label(egui::Color32::from_rgb(240, 240, 10), level);
                                            } else if level == "ERROR" {
                                                ui.colored_label(egui::Color32::from_rgb(240, 10, 10), level);
                                            }
                                        });
                                        row.col(|ui| {
                                            ui.label(self.logs[row_index].message.to_string());
                                        });
                                        row.col(|_ui| {});
                                        row.col(|ui| { ui.label(self.logs[row_index].caller.to_string()); });
                                    });
                                })
                        });
                    });
                });
        });
    }
}

struct Log {
    time: DateTime<Local>,
    level: String,
    message: String,
    caller: String,
}

impl Log {
    fn default() -> Self {
        Self {
            time: Default::default(),
            level: "".to_string(),
            message: "".to_string(),
            caller: "".to_string(),
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
