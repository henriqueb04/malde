#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod architecture;
mod parsers;

use std::fs;

use eframe::egui;

use crate::architecture::Cpu;
use crate::architecture::datapath::REGISTOR_NAMES;
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
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Carregar arquivo MAC").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_file()
                {
                    println!("Macroprograma: {}", path.display());
                    self.macroprogram = Some(path.display().to_string());
                }
                ui.label(self.macroprogram.as_ref().map(|v| v.as_str()).unwrap_or(""));
            });
            ui.horizontal(|ui| {
                if ui.button("Carregar arquivo MAL").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_file()
                {
                    println!("Microprograma: {}", path.display());
                    self.microprogram = Some(path.display().to_string());
                }
                ui.label(self.microprogram.as_ref().map(|v| v.as_str()).unwrap_or(""));
            });
            ui.horizontal(|ui| {
                if let Some(micro_path) = self.microprogram.clone() {
                    if ui.button("Montar Microprograma").clicked() {
                        self.assemble_micro(micro_path.as_str());
                    }
                }
                if let Some(macro_path) = self.macroprogram.clone() {
                    if ui.button("Montar Macroprograma").clicked() {
                        self.assemble_macro(macro_path.as_str());
                    }
                }
            });
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
                ui.label(format!(
                    "Registrador de Microinstrução:
amux: {}
cond: {}
alu: {}
sh: {}
mbr: {}
mar: {}
rd: {}
wr: {}
enc: {}
c: {}
b: {}
a: {}
addr: {}
",
                    mir.amux,
                    mir.cond,
                    mir.alu,
                    mir.sh,
                    mir.mbr,
                    mir.mar,
                    mir.rd,
                    mir.wr,
                    mir.enc,
                    mir.c,
                    mir.b,
                    mir.a,
                    mir.addr
                ));
                let mut regs = String::new();
                let (mar, mbr, registors) = self.cpu.get_registors();
                for (i, &reg) in REGISTOR_NAMES.iter().enumerate() {
                    if reg == "ir" || reg == "tir" || reg == "amask" || reg == "smask" {
                        regs.push_str(format!("{}: {:016b}\n", reg, registors[i] as i16).as_str());
                    } else {
                        regs.push_str(format!("{}: {}\n", reg, registors[i] as i16).as_str());
                    }
                }
                regs.push_str(format!("mar: {}", mar).as_str());
                regs.push_str(format!("mbr: {}", mbr as i16).as_str());
                ui.label("Registradores:");
                ui.label(regs.as_str());
            }
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
        }
    }
    fn assemble_micro(&mut self, path: &str) {
        let Ok(contents) = fs::read_to_string(path) else {
            self.show_error_modal(String::from("Falha ao ler arquivo"));
            return;
        };
        let mut mal_parser = mal::MALParser::new(&contents);
        match mal_parser.parse_instructions() {
            Ok(microinstructions) => self.cpu.load_microinstructions(
                microinstructions
                    .iter()
                    .map(|v| u32::from(v.clone()))
                    .collect(),
            ),
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
                error_type.to_string()
            )),
        }
    }
    fn reset_cpu(&mut self) {
        self.cpu.reset();
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
