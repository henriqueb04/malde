#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod architecture;
mod parsers;

use std::fs;

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::architecture::Cpu;
use crate::architecture::datapath::REGISTOR_NAMES;
use crate::architecture::memory::MEMORY_SIZE;
use crate::architecture::signals::ControlSignals;
use crate::parsers::{mac, mal};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Native file dialogs and drag-and-drop files",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new()))),
    )
}

pub struct MyApp {
    macroprogram: Option<String>,
    microprogram: Option<String>,
    msg_modal_open: bool,
    msg_modal_text: String,
    cpu: Cpu,
    mir: Option<ControlSignals>,
    cur_mpc: usize,
    microinstructions: Vec<String>,
    mem_view_index: usize,
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::right("right_panel")
            .resizable(true)
            .min_size(350.0)
            .show_inside(ui, |ui| {
                if self.cpu.is_ready() {
                    if ui.button("Próxima microinstrução").clicked() {
                        self.advance_microinstruction();
                        self.mir = Some(self.cpu.get_control_signals().clone());
                    }
                    if ui.button("Resetar").clicked() {
                        self.reset_cpu();
                    }
                }
                if let Some(mir) = self.mir.as_ref() {
                    ui.strong("Microinstrução atual:");
                    ui.label(
                        self.microinstructions
                            .get(self.cur_mpc)
                            .map(|s| s.as_str())
                            .unwrap_or("")
                    );
                    ui.strong("Registrador de Microinstrução:");
                    ui.label(format!(
                        concat!(
                            "amux: {}\n",
                            "cond: {}\n",
                            "alu: {}\n",
                            "sh: {}\n",
                            "mbr: {}\n",
                            "mar: {}\n",
                            "rd: {}\n",
                            "wr: {}\n",
                            "enc: {}\n",
                            "c: {}\n",
                            "b: {}\n",
                            "a: {}\n",
                            "addr: {}\n",
                        ),
                        mir.amux as i8,
                        mir.cond,
                        mir.alu,
                        mir.sh,
                        mir.mbr as i8,
                        mir.mar as i8,
                        mir.rd as i8,
                        mir.wr as i8,
                        mir.enc as i8,
                        mir.c,
                        mir.b,
                        mir.a,
                        mir.addr
                    ));
                    let mut regs = String::new();
                    let (mar, mbr, registors) = self.cpu.get_registors();
                    for (i, &reg) in REGISTOR_NAMES.iter().enumerate() {
                        if reg == "ir" || reg == "tir" || reg == "amask" || reg == "smask" {
                            regs.push_str(
                                format!("({}) {}: {:016b}\n", i, reg, registors[i] as i16).as_str(),
                            );
                        } else {
                            regs.push_str(
                                format!("({}) {}: {}\n", i, reg, registors[i] as i16).as_str(),
                            );
                        }
                    }
                    regs.push_str(format!("mar: {}\n", mar).as_str());
                    regs.push_str(format!("mbr: {}", mbr as i16).as_str());
                    ui.strong("Registradores:");
                    ui.label(regs.as_str());
                }
            });
        egui::Panel::bottom("bottom_panel")
            .resizable(true)
            .min_size(500.0)
            .show_inside(ui, |ui| {
                let memory = self.cpu.get_memory();
                let text_height = egui::TextStyle::Body
                    .resolve(ui.style())
                    .size
                    .max(ui.spacing().interact_size.y);
                let available_height = ui.available_height();
                let n_rows = 20;
                let n_cols = 12;
                let table = TableBuilder::new(ui)
                    .striped(true)
                    .resizable(false)
                    .cell_layout(egui::Layout::right_to_left(egui::Align::Center))
                    .column(
                        Column::remainder()
                            .at_least(100.0)
                            .clip(true)
                            .resizable(true),
                    )
                    .columns(Column::auto(), n_cols)
                    .min_scrolled_height(0.0)
                    .max_scroll_height(available_height);
                table
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("Endereço");
                        });
                        for i in 0..n_cols {
                            header.col(|ui| {
                                ui.strong(format!("(+{})", i));
                            });
                        }
                    })
                    .body(|body| {
                        body.rows(text_height, n_rows, |mut row| {
                            let row_index = self.mem_view_index + row.index() * n_cols;
                            row.col(|ui| {
                                if row_index < MEMORY_SIZE as usize {
                                    ui.strong(row_index.to_string());
                                } else {
                                    ui.strong("---");
                                }
                            });
                            for i in 0..n_cols {
                                row.col(|ui| {
                                    if let Some(v) = memory.get(row_index + i).map(|v| *v as i16) {
                                        // ui.label(format!("{:#06x}", v));
                                        ui.label(format!("{:05}", v));
                                    } else {
                                        ui.label("---");
                                    }
                                });
                            }
                        })
                    });
                ui.horizontal(|ui| {
                    if ui.button("⬅").clicked() {
                        self.mem_view_index = self.mem_view_index.saturating_sub(n_cols * n_rows);
                    }
                    if ui.button("➡").clicked() {
                        let new_index = self.mem_view_index + n_cols * n_rows;
                        if new_index < MEMORY_SIZE as usize {
                            self.mem_view_index = new_index;
                        }
                    }
                });
            });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Carregar arquivo MAC").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_file()
                {
                    println!("Macroprograma: {}", path.display());
                    self.macroprogram = Some(path.display().to_string());
                }
                ui.label(self.macroprogram.as_deref().unwrap_or(""));
            });
            ui.horizontal(|ui| {
                if ui.button("Carregar arquivo MAL").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_file()
                {
                    println!("Microprograma: {}", path.display());
                    self.microprogram = Some(path.display().to_string());
                }
                ui.label(self.microprogram.as_deref().unwrap_or(""));
            });
            ui.horizontal(|ui| {
                if let Some(micro_path) = self.microprogram.clone()
                    && ui.button("🔧 Montar Microprograma").clicked() {
                        self.assemble_micro(micro_path.as_str());
                    }
                if let Some(macro_path) = self.macroprogram.clone()
                    && ui.button("🔧 Montar Macroprograma").clicked() {
                        self.assemble_macro(macro_path.as_str());
                    }
            });
        });
        if self.msg_modal_open {
            let modal = egui::Modal::new(egui::Id::new("Msg modal 1")).show(ui, |ui| {
                ui.set_width(300.0);
                ui.heading("Message");

                ui.label(self.msg_modal_text.clone());

                egui::Sides::new().show(
                    ui,
                    |_ui| {},
                    |ui| {
                        if ui.button("Ok").clicked() {
                            ui.close();
                        }
                    },
                )
            });

            if modal.should_close() {
                self.msg_modal_open = false;
            }
        }
    }
}

impl MyApp {
    fn new() -> Self {
        MyApp {
            macroprogram: None,
            microprogram: None,
            msg_modal_open: false,
            msg_modal_text: String::new(),
            cpu: Cpu::new(Vec::new()),
            mir: None,
            cur_mpc: 0,
            microinstructions: Vec::new(),
            mem_view_index: 0,
        }
    }
    fn assemble_micro(&mut self, path: &str) {
        let Ok(contents) = fs::read_to_string(path) else {
            self.show_error_modal(String::from("Falha ao ler arquivo"));
            return;
        };
        let mut mal_parser = mal::MALParser::new(&contents);
        match mal_parser.parse_instructions() {
            Ok((micro_mem, microinstructions)) => {
                self.cpu.load_microinstructions(
                    micro_mem.iter().map(|v| u32::from(v.clone())).collect(),
                );
                self.microinstructions = microinstructions
                    .iter()
                    .map(|m| String::from(m.content))
                    .collect();
            }
            Err(err) => self.show_error_modal(err.to_string()),
        }
    }
    fn assemble_macro(&mut self, path: &str) {
        let Ok(contents) = fs::read_to_string(path) else {
            self.show_error_modal(String::from("Falha ao ler arquivo"));
            return;
        };
        let mut mac_parser = mac::ASMParser::new();
        match mac_parser.parse_text(&contents) {
            Ok(mem) => {
                self.cpu.init_memory(mem);
                self.reset_cpu();
            }
            Err((lineno, error_type)) => self.show_error_modal(format!(
                "Erro no macroprograma, linha {}: {}",
                lineno,
                error_type
            )),
        }
    }
    fn reset_cpu(&mut self) {
        self.cpu.reset();
        self.mir = None;
    }

    fn advance_microinstruction(&mut self) {
        (_, self.cur_mpc) = self.cpu.advance_microinstruction();
    }

    fn show_error_modal(&mut self, msg: String) {
        println!("{}", msg);
        self.msg_modal_text = msg;
        self.msg_modal_open = true;
    }
}
