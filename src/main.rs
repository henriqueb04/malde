#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod architecture;
mod parsers;
mod virtual_machine;

use log::{debug, error};

use std::{fmt::Display, fs};

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::{
    architecture::signals::CONTROL_SIGNAL_NAMES,
    virtual_machine::{
        DATA_SEGMENT_START, MEMORY_SIZE, REGISTER_NAMES, TEXT_SEGMENT_START, VM, VMResponse,
    },
};

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "MALDE: Simulador de linguagem MAL",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new()))),
    )
}

#[derive(Default)]
pub struct MyApp {
    vm: VM,
    macroprogram: Option<String>,
    microprogram: Option<String>,
    msg_modal_open: bool,
    msg_modal_text: String,
    value_format: ValueFormatType,
    cur_mpc: usize,
    next_mpc: usize,
    selected: usize,
    scroll_mpc: Option<usize>,
    mem_view_index: usize,
    mem_goto: Option<MemGoto>,
    last_mem_goto: MemGoto,
    bottom_panel_tab: BottomPanelTab,
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);
        egui::Panel::right("right_panel")
            .resizable(true)
            .default_size(350.0)
            .show_inside(ui, |ui| {
                self.side_panel_ui(ui);
            });
        egui::Panel::bottom("bottom_panel")
            .resizable(true)
            .default_size(440.0)
            .show_inside(ui, |ui| {
                self.bottom_panel_ui(ui);
            });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Carregar arquivo MAC").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_file()
                {
                    debug!("Macroprograma: {}", path.display());
                    self.macroprogram = Some(path.display().to_string());
                }
                ui.label(self.macroprogram.as_deref().unwrap_or(""));
            });
            ui.horizontal(|ui| {
                if ui.button("Carregar arquivo MAL").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_file()
                {
                    debug!("Microprograma: {}", path.display());
                    self.microprogram = Some(path.display().to_string());
                }
                ui.label(self.microprogram.as_deref().unwrap_or(""));
            });
            ui.horizontal(|ui| {
                if let Some(micro_path) = self.microprogram.clone()
                    && ui.button("🔧 Montar Microprograma").clicked()
                {
                    self.assemble_micro(micro_path.as_str());
                }
                if let Some(macro_path) = self.macroprogram.clone()
                    && ui.button("🔧 Montar Macroprograma").clicked()
                {
                    self.assemble_macro(macro_path.as_str());
                }
            });
            ui.separator();
            let available_height = ui.available_height();
            if self.vm.is_ready() {
                let mut mal_table = TableBuilder::new(ui)
                    .striped(true)
                    .resizable(false)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::auto())
                    .column(Column::remainder().clip(true))
                    .min_scrolled_height(0.0)
                    .max_scroll_height(available_height)
                    .sense(egui::Sense::click());
                if let Some(mpc) = self.scroll_mpc.take() {
                    mal_table = mal_table.scroll_to_row(mpc, None);
                }
                let mics = self.vm.get_microinstructions();
                mal_table.body(|body| {
                    body.rows(text_height, mics.len(), |mut row| {
                        let row_index = row.index();
                        row.set_selected(row_index == self.selected);
                        row.col(|ui| {
                            if row_index == self.cur_mpc {
                                ui.strong(row_index.to_string());
                            } else {
                                ui.label(row_index.to_string());
                            }
                        });
                        row.col(|ui| {
                            let text = mics
                                .get(row_index)
                                .map(|v| v.content.as_str())
                                .unwrap_or("");
                            if row_index == self.next_mpc {
                                ui.label(egui::RichText::new(text).monospace().strong())
                                    .on_hover_text("Próxima microinstrução");
                            } else if row_index == self.cur_mpc {
                                ui.monospace(text).on_hover_text("Microinstrução executada");
                            } else {
                                ui.monospace(text);
                            }
                        });
                        if row.response().clicked() {
                            self.selected = row_index;
                        }
                    });
                });
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
            // FIXME: retirar caminhos fixos
            macroprogram: Some(String::from("/home/henrique/code/mac1/teste2.asm")),
            microprogram: Some(String::from("/home/henrique/code/mac1/malde.mal")),
            vm: VM::new(),
            mem_goto: Some(MemGoto::Data),
            ..Default::default()
        }
    }
    fn assemble_micro(&mut self, path: &str) {
        let Ok(contents) = fs::read_to_string(path) else {
            self.show_error_modal(String::from("Falha ao ler arquivo"));
            return;
        };
        if let Err(err) = self.vm.assemble_mic(&contents) {
            self.show_error_modal(err.to_string());
        };
    }
    fn assemble_macro(&mut self, path: &str) {
        let Ok(contents) = fs::read_to_string(path) else {
            self.show_error_modal(String::from("Falha ao ler arquivo"));
            return;
        };
        if let Err(err) = self.vm.assemble_mac(&contents) {
            self.show_error_modal(err.to_string());
        };
    }
    fn reset_vm(&mut self) {
        self.vm.reset();
        self.selected = 0;
        self.cur_mpc = 0;
        self.next_mpc = 0;
    }

    fn advance_microinstruction(&mut self) {
        let VMResponse { mpc, prev_mpc } = self.vm.advance_microinstruction();
        self.next_mpc = mpc;
        self.cur_mpc = prev_mpc;
        self.scroll_mpc = Some(self.cur_mpc);
        self.selected = self.cur_mpc;
    }

    fn advance_macroinstruction(&mut self) {
        let VMResponse { mpc, prev_mpc } = self.vm.advance_macroinstruction();
        self.next_mpc = mpc;
        self.cur_mpc = prev_mpc;
        self.scroll_mpc = Some(self.cur_mpc);
        self.selected = self.cur_mpc;
    }

    ////////////
    // UI

    fn format_value(&self, value: usize) -> String {
        let value = value as i16;
        match self.value_format {
            ValueFormatType::Decimal => format!("{:05}", value),
            ValueFormatType::Hexadecimal => format!("0x{:04X}", value),
            ValueFormatType::Binary => format!("0b{:016b}", value),
        }
    }

    fn show_error_modal(&mut self, msg: String) {
        error!("{}", msg);
        self.msg_modal_text = msg;
        self.msg_modal_open = true;
    }

    fn side_panel_ui(&mut self, ui: &mut egui::Ui) {
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);
        if self.vm.is_ready() {
            ui.horizontal(|ui| {
                if ui.button("Próxima microinstrução").clicked() {
                    self.advance_microinstruction();
                }
                if ui.button("Próxima macroinstrução").clicked() {
                    self.advance_macroinstruction();
                }
                if ui.button("Resetar").clicked() {
                    self.reset_vm();
                }
            });
            ui.separator();
            let mir = &self.vm.get_microinstructions()[self.selected].mir;
            ui.set_min_width(50.0);
            ui.strong("Registrador de Microinstrução:");
            let mir_vals = mir.to_array();
            let mic_table = TableBuilder::new(ui)
                .auto_shrink([true; 2])
                .id_salt("mic_table")
                .striped(true)
                .resizable(false)
                .vscroll(false)
                .cell_layout(egui::Layout::top_down(egui::Align::Center))
                .column(Column::auto())
                .column(Column::remainder().clip(true).resizable(true))
                .min_scrolled_height(0.0);
            mic_table
                .header(text_height, |mut header| {
                    header.col(|ui| {
                        ui.strong("Registrador");
                    });
                    header.col(|ui| {
                        ui.strong("Valor");
                    });
                })
                .body(|body| {
                    body.rows(text_height, 13, |mut row| {
                        let row_index = row.index();
                        row.col(|ui| {
                            ui.label(CONTROL_SIGNAL_NAMES[row_index]);
                        });
                        row.col(|ui| {
                            ui.label(mir_vals[row_index].to_string());
                        });
                    });
                });
            ui.strong("Registradores:");
            let (mar, mbr, registers) = self.vm.get_registers();
            let reg_table = TableBuilder::new(ui)
                .auto_shrink([true; 2])
                .id_salt("reg_table")
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::top_down(egui::Align::Center))
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::remainder().clip(true).resizable(true))
                .min_scrolled_height(0.0);
            reg_table
                .header(text_height, |mut header| {
                    header.col(|ui| {
                        ui.strong("Número");
                    });
                    header.col(|ui| {
                        ui.strong("Registrador");
                    });
                    header.col(|ui| {
                        ui.strong("Valor");
                    });
                })
                .body(|mut body| {
                    body.row(text_height, |mut row| {
                        row.set_selected(self.vm.get_events().mar_written.is_some());
                        row.col(|ui| {
                            ui.label("");
                        });
                        row.col(|ui| {
                            ui.label("mar");
                        });
                        row.col(|ui| {
                            let label = egui::Label::new(self.format_value(mar as usize));
                            if let Some(event) = &self.vm.get_events().mar_written {
                                ui.add(label).on_hover_text(format!(
                                    "Anterior: {}",
                                    self.format_value(event.before as usize)
                                ));
                            } else {
                                ui.add(label);
                            }
                        });
                    });
                    body.row(text_height, |mut row| {
                        row.set_selected(self.vm.get_events().mbr_written.is_some());
                        row.col(|ui| {
                            ui.label("");
                        });
                        row.col(|ui| {
                            ui.label("mbr");
                        });
                        row.col(|ui| {
                            let label = egui::Label::new(self.format_value(mbr as usize));
                            if let Some(event) = &self.vm.get_events().mbr_written {
                                ui.add(label).on_hover_text(format!(
                                    "Anterior: {}",
                                    self.format_value(event.before as usize)
                                ));
                            } else {
                                ui.add(label);
                            }
                        });
                    });
                    body.rows(text_height, 16, |mut row| {
                        let row_index = row.index();
                        let reg_name = REGISTER_NAMES.get(row_index).map_or("", |v| v);
                        if self
                            .vm
                            .get_events()
                            .register_writes
                            .contains_key(&(row_index as u8))
                        {
                            row.set_selected(true);
                        }
                        row.col(|ui| {
                            ui.label(row_index.to_string());
                        });
                        row.col(|ui| {
                            ui.label(reg_name);
                        });
                        row.col(|ui| {
                            let label = egui::Label::new(
                                if reg_name == "ir"
                                    || reg_name == "tir"
                                    || reg_name == "amask"
                                    || reg_name == "smask"
                                {
                                    format!("0b{:016b}", registers[row_index])
                                } else {
                                    self.format_value(registers[row_index] as usize)
                                },
                            );
                            if let Some(event) =
                                &self.vm.get_events().register_writes.get(&(row_index as u8))
                            {
                                ui.add(label).on_hover_text(format!(
                                    "Anterior: {}",
                                    self.format_value(event.before as usize)
                                ));
                            } else {
                                ui.add(label);
                            }
                        });
                    });
                });
        }
    }

    fn bottom_panel_ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            self.bottom_panel_tab == BottomPanelTab::MemTable,
                            "Memória",
                        )
                        .clicked()
                    {
                        self.bottom_panel_tab = BottomPanelTab::MemTable;
                    }
                    if ui
                        .selectable_label(self.bottom_panel_tab == BottomPanelTab::Stdout, "Saída")
                        .clicked()
                    {
                        self.bottom_panel_tab = BottomPanelTab::Stdout;
                    }
                });
                ui.separator();
                match self.bottom_panel_tab {
                    BottomPanelTab::MemTable => self.mem_table_ui(ui),
                    BottomPanelTab::Stdout => self.stdout_ui(ui),
                }
            });
    }

    fn stdout_ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(self.vm.get_stdout());
        });
    }

    fn mem_table_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(goto) = self.mem_goto.take() {
            self.mem_view_index = goto.get_slot();
            self.last_mem_goto = goto;
        }
        let memory = self.vm.get_memory();
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);
        let available_height = ui.available_height();
        let n_rows = 16;
        let n_cols = self.value_format.table_columns();
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::right_to_left(egui::Align::Center))
            .column(Column::auto().at_least(100.0).clip(true).resizable(true))
            .columns(Column::remainder().clip(true), n_cols)
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
                        if row_index < MEMORY_SIZE {
                            ui.strong(self.format_value(row_index));
                        } else {
                            ui.strong("---");
                        }
                    });
                    for i in 0..n_cols {
                        row.col(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                            let mem_slot = row_index + i;
                            let before = self
                                .vm
                                .get_events()
                                .memory_writes
                                .get(&(mem_slot as u16))
                                .map(|v| v.before);
                            let text = if let Some(v) = memory.get(mem_slot) {
                                self.format_value(*v as usize)
                            } else {
                                String::from("---")
                            };
                            if let Some(before) = before {
                                ui.painter().rect_filled(
                                    ui.max_rect(),
                                    0,
                                    ui.visuals().selection.bg_fill,
                                );
                                ui.strong(text).on_hover_text(format!(
                                    "Anterior: {}",
                                    self.format_value(before as usize)
                                ));
                            } else {
                                ui.label(text);
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
                if new_index < MEMORY_SIZE {
                    self.mem_view_index = new_index;
                }
            }
            egui::ComboBox::from_label("Visualização")
                .selected_text(self.value_format.to_string())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.value_format,
                        ValueFormatType::Decimal,
                        "Decimal",
                    );
                    ui.selectable_value(
                        &mut self.value_format,
                        ValueFormatType::Hexadecimal,
                        "Hexadecimal",
                    );
                    ui.selectable_value(&mut self.value_format, ValueFormatType::Binary, "Binário");
                });
            egui::ComboBox::from_label("Memória")
                .selected_text(self.last_mem_goto.to_string())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.mem_goto,
                        Some(MemGoto::Text),
                        MemGoto::Text.to_string(),
                    );
                    ui.selectable_value(
                        &mut self.mem_goto,
                        Some(MemGoto::Data),
                        MemGoto::Data.to_string(),
                    );
                });
        });
    }
}

#[derive(Default, PartialEq, Eq)]
enum BottomPanelTab {
    #[default]
    MemTable,
    Stdout,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum ValueFormatType {
    Decimal,
    #[default]
    Hexadecimal,
    Binary,
}

impl Display for ValueFormatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ValueFormatType::Decimal => "Decimal",
                ValueFormatType::Hexadecimal => "Hexadecimal",
                ValueFormatType::Binary => "Binário",
            }
        )
    }
}

impl ValueFormatType {
    pub const fn table_columns(&self) -> usize {
        match self {
            ValueFormatType::Decimal => 12,
            ValueFormatType::Hexadecimal => 12,
            ValueFormatType::Binary => 6,
        }
    }
}

#[derive(Default, PartialEq, Eq)]
enum MemGoto {
    #[default]
    Data,
    Text,
}

impl MemGoto {
    fn get_slot(&self) -> usize {
        match self {
            MemGoto::Data => DATA_SEGMENT_START,
            MemGoto::Text => TEXT_SEGMENT_START,
        }
    }
}

impl Display for MemGoto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MemGoto::Data => format!(".data (0x{:04X})", DATA_SEGMENT_START),
                MemGoto::Text => format!(".text (0x{:04X})", TEXT_SEGMENT_START),
            }
        )
    }
}
