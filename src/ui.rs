use std::{fmt::Display, fs, path::PathBuf};

use anyhow::anyhow;
use crossbeam_channel::{Sender, unbounded};
use eframe::{egui, wgpu::naga::keywords};
use egui_extras::{Column, TableBuilder};
use egui_inbox::UiInbox;
use log::{debug, error};

use crate::{
    architecture::{datapath::DataRegisters, signals::CONTROL_SIGNAL_NAMES},
    parsers::{
        asm::{Instruction, KeywordMap},
        mal::Microinstruction,
        source_map::SourceMap,
    },
    virtual_machine::{
        DATA_SEGMENT_START, MEMORY_SIZE, Register, TEXT_SEGMENT_START, VM, VMExecutionInfo,
        VMExecutionType, VMInputRequest, VMInputRequestType, VMInputResponse, VMResponse, VMState,
    },
};

#[derive(Default)]
pub struct MyApp {
    vm: Option<VM>,
    task_inbox: UiInbox<(Result<VMTask, String>, VM)>,
    vm_input_request: Option<UIInputInboxes>,
    vm_pauser: Option<Sender<()>>,
    memory: Vec<u16>,
    stdout_inbox: UiInbox<String>,
    stdout: String,
    input_modal_request: Option<VMInputRequest>,
    input_modal_text: String,
    input_model_error: String,
    last_res: Box<VMResponse>,
    instructions: Vec<Instruction>,
    microinstructions: Vec<Microinstruction>,
    keywords: KeywordMap,
    breaks_mic: Vec<bool>,
    breaks_mac: Vec<bool>,
    macroprogram: Option<PathBuf>,
    microprogram: Option<PathBuf>,
    config_modal_show: bool,
    config_modal_keywords: Vec<(String, String)>,
    msg_modal_text: Option<String>,
    value_format: ValueFormatType,
    scroll_mpc: Option<usize>,
    scroll_pc: Option<usize>,
    selected: usize,
    mem_view_index: usize,
    mem_goto: Option<MemGoto>,
    last_mem_goto: MemGoto,
    bottom_panel_tab: BottomPanelTab,
    instruction_table_tab: InstructionTableTab,
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Channels and signals
        for text in self.stdout_inbox.read(ui) {
            self.print_to_stdout(&text);
        }
        for task in self.task_inbox.read(ui) {
            match task.0 {
                Ok(task) => match task {
                    VMTask::Execute(res) => {
                        self.scroll_mpc = Some(res.mpc);
                        self.selected = res.mpc;
                        self.last_res = res;
                        if let Some(pc) = self.last_res.events.instruction_reads.iter().next() {
                            self.scroll_pc = Some(*pc as usize);
                        }
                    }
                    VMTask::AssembleMic(mics) => {
                        self.microinstructions = mics;
                        self.breaks_mic.fill(false);
                        self.breaks_mic.resize(self.microinstructions.len(), false);
                    }
                    VMTask::AssembleMac(ins) => {
                        self.instructions = ins;
                        self.breaks_mac.fill(false);
                        self.breaks_mac.resize(self.instructions.len(), false);
                    }
                },
                Err(err) => {
                    self.show_error_modal(err.clone());
                }
            }
            self.vm = Some(task.1);
            self.memory = Vec::from(self.vm.as_ref().unwrap().memory());
        }
        if let Some(input_request) = &self.vm_input_request {
            for input_validation in input_request.validation.read(ui) {
                match input_validation {
                    Ok(..) => {
                        self.input_modal_request = None;
                        self.input_modal_text.clear();
                        self.input_model_error.clear();
                    }
                    Err(err) => {
                        self.input_model_error = err;
                    }
                }
            }
            for input_request in input_request.request.read(ui) {
                self.input_modal_request = Some(input_request);
            }
        }

        // UI
        egui::Panel::right("right_panel")
            .resizable(true)
            .default_size(350.0)
            .show(ui, |ui| {
                self.side_panel_ui(ui);
            });
        egui::Panel::bottom("bottom_panel")
            .resizable(true)
            .default_size(440.0)
            .show(ui, |ui| {
                self.bottom_panel_ui(ui);
            });
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    egui::Grid::new("files_grid")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            if ui.button("🗁 Carregar arquivo Assembly").clicked()
                                && let Some(path) = rfd::FileDialog::new().pick_file()
                            {
                                debug!("Macroprograma: {}", path.display());
                                self.macroprogram = Some(path);
                            }
                            ui.label(
                                self.macroprogram
                                    .as_ref()
                                    .map(|p| p.to_string_lossy())
                                    .unwrap_or_default(),
                            );
                            ui.end_row();
                            if ui.button("🗁 Carregar arquivo MAL").clicked()
                                && let Some(path) = rfd::FileDialog::new().pick_file()
                            {
                                debug!("Microprograma: {}", path.display());
                                self.microprogram = Some(path);
                            }
                            ui.label(
                                self.microprogram
                                    .as_ref()
                                    .map(|p| p.to_string_lossy())
                                    .unwrap_or_default(),
                            );
                        });
                    ui.add_enabled_ui(self.vm.is_some(), |ui| {
                        ui.horizontal(|ui| {
                            if let Some(micro_path) = &self.microprogram
                                && ui.button("🔧 Montar Microprograma").clicked()
                            {
                                self.assemble_micro(micro_path.clone());
                            }
                            if let Some(macro_path) = &self.macroprogram
                                && ui.button("🔧 Montar Macroprograma").clicked()
                            {
                                self.assemble_macro(macro_path.clone());
                            }
                        });
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::LEFT), |ui| {
                    if ui.button("⚙").clicked() {
                        self.show_config_modal();
                    }
                });
            });
            ui.separator();
            self.instruction_table_ui(ui);
        });
        let mut invalid = false;
        if self.config_modal_show {
            let modal = egui::Modal::new(egui::Id::new("Config modal 1")).show(ui, |ui| {
                ui.set_width(400.0);
                ui.heading("Instruções (Assembly)");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut to_remove = None;
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            for (i, pair) in self.config_modal_keywords.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    let res1 = ui.add(
                                        egui::TextEdit::singleline(&mut pair.0)
                                            .desired_width(190.0),
                                    );
                                    let res2 = ui.add(
                                        egui::TextEdit::singleline(&mut pair.1)
                                            .desired_width(200.0),
                                    );
                                    if let Some(err) =
                                        KeywordMap::validate_pair(&pair.0.trim(), &pair.1.trim())
                                            .err()
                                            .map(|err| err.to_string())
                                    {
                                        invalid = true;
                                        ui.painter().rect_stroke(
                                            res1.rect,
                                            ui.style().visuals.widgets.hovered.corner_radius,
                                            egui::Stroke::new(2.0, egui::Color32::RED),
                                            egui::StrokeKind::Middle,
                                        );
                                        ui.painter().rect_stroke(
                                            res2.rect,
                                            ui.style().visuals.widgets.hovered.corner_radius,
                                            egui::Stroke::new(2.0, egui::Color32::RED),
                                            egui::StrokeKind::Middle,
                                        );
                                        res1.on_hover_text(&err);
                                        res2.on_hover_text(&err);
                                    }
                                    if ui.button("🗙").clicked() {
                                        to_remove = Some(i);
                                    }
                                });
                            }
                            if let Some(to_remove) = to_remove {
                                self.config_modal_keywords.remove(to_remove);
                            }
                            ui.centered_and_justified(|ui| {
                                if ui.button("+").clicked() {
                                    self.config_modal_keywords
                                        .push((String::new(), String::new()));
                                }
                            });
                            let available_width = ui.available_width();
                            egui::Grid::new("grid keywords import export")
                                .num_columns(2)
                                .min_col_width(available_width / 2.0)
                                .show(ui, |ui| {
                                    ui.centered_and_justified(|ui| {
                                        if ui.button("Importar").clicked()
                                            && let Some(path) = rfd::FileDialog::new().pick_file()
                                        {
                                            if let Err(err) = self.import_keywords(path) {
                                                self.show_error_modal(err.to_string());
                                            }
                                        }
                                    });
                                    ui.centered_and_justified(|ui| {
                                        ui.add_enabled_ui(!invalid, |ui| {
                                            if ui.button("Exportar").clicked()
                                                && let Some(path) =
                                                    rfd::FileDialog::new().save_file()
                                            {
                                                self.set_keywords_from_modal();
                                                if let Err(err) = self.export_keywords(path) {
                                                    self.show_error_modal(err.to_string());
                                                }
                                            }
                                        });
                                    });
                                });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::LEFT), |ui| {
                                ui.add_enabled_ui(!invalid, |ui| {
                                    if ui.button("Ok").clicked() && !invalid {
                                        ui.close();
                                    }
                                })
                            });
                        });
                        ui.add_space(10.0);
                    });
                });
            });
            if modal.should_close() && !invalid {
                self.config_modal_show = false;
                self.set_keywords_from_modal();
            }
        }
        if let Some(text) = &self.msg_modal_text {
            let modal = egui::Modal::new(egui::Id::new("Msg modal 1")).show(ui, |ui| {
                ui.set_width(300.0);
                ui.heading("Message");
                ui.monospace(text);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::LEFT), |ui| {
                    if ui.button("Ok").clicked() {
                        ui.close();
                    }
                });
            });
            if modal.should_close() {
                self.msg_modal_text = None;
            }
        }
        self.input_request_ui(ui);
    }
}

impl MyApp {
    pub fn new(
        microprogram: Option<PathBuf>,
        macroprogram: Option<PathBuf>,
        keywords: Option<KeywordMap>,
        no_info_print: bool,
    ) -> Self {
        let keywords = keywords.unwrap_or_default();
        let config_modal_keywords = keywords.str_values();
        let mut vm = VM::new();
        let stdout_inbox = UiInbox::new();
        let s_stdout = stdout_inbox.sender();
        vm.set_on_print(Box::new(move |text| {
            s_stdout.send(text).expect("Erro ao exibir saída")
        }));
        vm.set_info_print(!no_info_print);
        MyApp {
            vm: Some(vm),
            mem_goto: Some(MemGoto::Data),
            stdout_inbox,
            microprogram,
            macroprogram,
            keywords,
            config_modal_keywords,
            ..Default::default()
        }
    }
    fn assemble_micro(&mut self, path: PathBuf) {
        if let Some(mut vm) = self.vm.take() {
            self.task_inbox = UiInbox::new();
            let s_task = self.task_inbox.sender();
            std::thread::spawn(move || {
                s_task
                    .send((
                        (|vm: &mut VM| {
                            let source_map = SourceMap::from_filepath(path)
                                .map_err(|err| format!("Falha ao ler arquivo: {}", err))?;
                            let mics = vm
                                .assemble_mic(&source_map)
                                .map_err(|err| err.to_string())?;
                            Ok(VMTask::AssembleMic(mics))
                        })(&mut vm),
                        vm,
                    ))
                    .expect("Resposta assícrona falhou");
            });
        }
    }
    fn assemble_macro(&mut self, path: PathBuf) {
        if let Some(mut vm) = self.vm.take() {
            self.task_inbox = UiInbox::new();
            let s_task = self.task_inbox.sender();
            let path = path.clone();
            let keywords = self.keywords.clone();
            std::thread::spawn(move || {
                s_task
                    .send((
                        (|vm: &mut VM| {
                            let source_map = SourceMap::from_filepath(path)
                                .map_err(|err| format!("Falha ao ler arquivo: {}", err))?;
                            let ins = vm
                                .assemble_mac(&source_map, keywords)
                                .map_err(|err| err.to_string())?;
                            Ok(VMTask::AssembleMac(ins))
                        })(&mut vm),
                        vm,
                    ))
                    .expect("Resposta assícrona falhou");
            });
        }
    }
    fn reset_vm(&mut self) {
        if let Some(vm) = &mut self.vm {
            vm.reset();
            self.memory = vm.memory().into();
            self.selected = 0;
            self.last_res.mpc = 0;
            self.last_res.prev_mpc = 0;
        }
    }

    fn execute(&mut self, execution_type: VMExecutionType) {
        let request_inbox = UiInbox::new();
        let s_request = request_inbox.sender();
        let validation_inbox = UiInbox::new();
        let s_validation = validation_inbox.sender();
        self.vm_input_request = Some(UIInputInboxes {
            request: request_inbox,
            validation: validation_inbox,
        });
        let (s_pause, r_pause) = unbounded::<()>();
        self.vm_pauser = Some(s_pause);
        let breaks_mic = self
            .breaks_mic
            .iter()
            .enumerate()
            .filter(|(_, v)| **v)
            .map(|(i, _)| i)
            .collect();
        let breaks_mac = self
            .breaks_mac
            .iter()
            .enumerate()
            .filter(|(_, v)| **v)
            .map(|(i, _)| i)
            .collect();
        if let Some(mut vm) = self.vm.take() {
            self.task_inbox = UiInbox::new();
            let s_task = self.task_inbox.sender();
            std::thread::spawn(move || {
                let res = vm.execute(
                    execution_type,
                    VMExecutionInfo {
                        r_pause,
                        on_input_request: Box::new(move |req| {
                            s_request
                                .send(req)
                                .expect("Não foi possível requisitar entrada");
                        }),
                        on_validation: Box::new(move |val| {
                            s_validation
                                .send(val)
                                .expect("Não foi possível validar entrada");
                        }),
                        breaks_mic,
                        breaks_mac,
                    },
                );
                s_task
                    .send((Ok(VMTask::Execute(Box::new(res))), vm))
                    .expect("Resposta assícrona falhou");
            });
        }
    }
    fn pause(&mut self) {
        if let Some(pause_s) = &self.vm_pauser {
            pause_s.send(()).expect("Erro ao pausar o programa");
        }
    }

    fn send_input_response(&mut self, inp: VMInputResponse) {
        let request = self
            .input_modal_request
            .as_ref()
            .expect("Tentativa de enviar entrada não requisitada");
        request
            .sender
            .send(inp)
            .expect("Tentativa de enviar entrada não requisitada");
    }

    fn print_to_stdout(&mut self, text: &str) {
        self.stdout.push_str(text);
    }

    fn set_keywords_from_modal(&mut self) {
        match self.config_modal_keywords.clone().try_into() {
            Ok(keywords) => self.keywords = keywords,
            Err(err) => self.show_error_modal(format!(
                "Erro ao processar keyword {}: {}",
                err.0 + 1,
                err.1
            )),
        }
    }
    fn import_keywords(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let keywords = KeywordMap::from_filename(path).map_err(|err| {
            anyhow!(
                "Erro ao ler arquivo de instruções{}: {}",
                if let Some(l) = err.0 {
                    format!(" (linha {})", l + 1)
                } else {
                    String::new()
                },
                err.1
            )
        })?;
        self.config_modal_keywords = keywords.str_values();
        self.keywords = keywords;
        Ok(())
    }
    fn export_keywords(&self, path: PathBuf) -> anyhow::Result<()> {
        let mut contents = String::new();
        for (name, op) in self.keywords.str_values().iter() {
            contents.push_str(&name);
            contents.push(',');
            contents.push_str(&op);
            contents.push('\n');
        }
        fs::write(path, contents)?;
        Ok(())
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
        self.msg_modal_text = Some(msg);
    }

    fn show_config_modal(&mut self) {
        self.config_modal_show = true;
    }

    fn input_request_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(request_type) = self.input_modal_request.as_ref().map(|r| r.typ.clone()) {
            egui::Modal::new(egui::Id::new("Input modal")).show(ui, |ui| {
                ui.set_width(300.0);
                ui.heading("Entrada");
                ui.vertical(|ui| {
                    ui.label(match request_type {
                        VMInputRequestType::Int => "Digite um número:",
                        VMInputRequestType::Char => "Digite um caractere:",
                        VMInputRequestType::String => "Digite um texto:",
                    });
                    ui.add(egui::TextEdit::singleline(&mut self.input_modal_text));
                    ui.colored_label(egui::Color32::RED, self.input_model_error.clone());
                    if ui.button("Enviar").clicked() {
                        match request_type {
                            VMInputRequestType::Int => {
                                if let Ok(n) = self.input_modal_text.parse::<isize>() {
                                    self.send_input_response(VMInputResponse::Int(n));
                                } else {
                                    self.input_model_error = "Número inválido".to_string();
                                }
                            }
                            VMInputRequestType::Char => {
                                if self.input_modal_text.len() == 1
                                    && self.input_modal_text.is_ascii()
                                {
                                    self.send_input_response(VMInputResponse::Char(
                                        self.input_modal_text.chars().next().unwrap(),
                                    ));
                                } else {
                                    self.input_model_error =
                                        "Deve haver apenas um caractere ASCII".to_string();
                                }
                            }
                            VMInputRequestType::String => {
                                if self.input_modal_text.is_ascii() {
                                    self.send_input_response(VMInputResponse::String(
                                        self.input_modal_text.clone(),
                                    ));
                                } else {
                                    self.input_model_error =
                                        "Todos os carateres precisam ser do padrão ASCII"
                                            .to_string();
                                }
                            }
                        }
                    }
                });
            });
        }
    }

    fn side_panel_ui(&mut self, ui: &mut egui::Ui) {
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);
        let state = self
            .vm
            .as_ref()
            .map(|vm| vm.state().clone())
            .unwrap_or_else(|| self.last_res.state.clone());
        if !self.microinstructions.is_empty() {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("🔃︎ Resetar").clicked() {
                        self.reset_vm();
                    }
                    if self.vm.is_some() {
                        ui.add_enabled_ui(state != VMState::Halted, |ui| {
                            if ui.button("▶ Executar").clicked() {
                                self.execute(VMExecutionType::Run);
                            }
                        });
                    } else {
                        if ui.button("⏸ Pausar").clicked() {
                            self.pause();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(state != VMState::Halted, |ui| {
                        if ui.button("⬇️ ︎Próxima microinstrução").clicked() {
                            self.execute(VMExecutionType::Microinstruction);
                        }
                        if ui.button("⏭ Próxima macroinstrução").clicked() {
                            self.execute(VMExecutionType::Macroinstruction);
                        }
                    });
                });
            });
        };
        ui.separator();
        if !self.microinstructions.is_empty() {
            let mir = &self.microinstructions[self.selected].mir;
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
                    body.rows(text_height, 14, |mut row| {
                        let row_index = row.index();
                        row.col(|ui| {
                            ui.label(CONTROL_SIGNAL_NAMES[row_index]);
                        });
                        row.col(|ui| {
                            ui.label(mir_vals[row_index].to_string());
                        });
                    });
                });
        }
        ui.strong("Registradores:");
        let DataRegisters(mar, mbr, registers) = self.last_res.registers;
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
                    row.set_selected(self.last_res.events.mar_written.is_some());
                    row.col(|ui| {
                        ui.label("");
                    });
                    row.col(|ui| {
                        ui.label("mar");
                    });
                    row.col(|ui| {
                        let label = egui::Label::new(self.format_value(mar as usize));
                        if let Some(event) = &self.last_res.events.mar_written {
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
                    row.set_selected(self.last_res.events.mbr_written.is_some());
                    row.col(|ui| {
                        ui.label("");
                    });
                    row.col(|ui| {
                        ui.label("mbr");
                    });
                    row.col(|ui| {
                        let label = egui::Label::new(self.format_value(mbr as usize));
                        if let Some(event) = &self.last_res.events.mbr_written {
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
                    let reg_name = Register::NAMES.get(row_index).map_or("", |v| v);
                    if self
                        .last_res
                        .events
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
                        let mut hover: Option<&'static str> = None;
                        let label = egui::Label::new(match reg_name {
                            "ir" | "tir" | "amask" | "smask" => {
                                egui::RichText::new(format!("0b{:016b}", registers[row_index]))
                            }
                            "sp" => {
                                let v = registers[row_index] as usize;
                                let t = self.format_value(v);
                                if v < DATA_SEGMENT_START {
                                    hover = Some("Registrador sp dentro do segmento de instruções");
                                    egui::RichText::new(t).underline()
                                } else {
                                    egui::RichText::new(t)
                                }
                            }
                            _ => egui::RichText::new(
                                self.format_value(registers[row_index] as usize),
                            ),
                        });
                        if let Some(hover) = hover {
                            ui.add(label).on_hover_text(hover);
                        } else if let Some(event) =
                            &self.last_res.events.register_writes.get(&(row_index as u8))
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
            ui.monospace(&self.stdout);
        });
    }

    fn mem_table_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(goto) = self.mem_goto.take() {
            self.mem_view_index = goto.get_slot();
            self.last_mem_goto = goto;
        }
        let memory = &self.memory;
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
                                .last_res
                                .events
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

    fn instruction_table_ui(&mut self, ui: &mut egui::Ui) {
        if !self.microinstructions.is_empty() {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                self.instruction_table_tab == InstructionTableTab::Micro,
                                "MAL",
                            )
                            .clicked()
                        {
                            self.instruction_table_tab = InstructionTableTab::Micro;
                        }
                        if ui
                            .selectable_label(
                                self.instruction_table_tab == InstructionTableTab::Macro,
                                "Assembly",
                            )
                            .clicked()
                        {
                            self.instruction_table_tab = InstructionTableTab::Macro;
                        }
                    });
                    ui.separator();
                    match self.instruction_table_tab {
                        InstructionTableTab::Micro => self.mal_table_ui(ui),
                        InstructionTableTab::Macro => self.asm_table_ui(ui),
                    }
                });
        }
    }

    fn asm_table_ui(&mut self, ui: &mut egui::Ui) {
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);
        let available_height = ui.available_height();
        if !self.instructions.is_empty() {
            let mut asm_table = TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::remainder().clip(true))
                .min_scrolled_height(0.0)
                .max_scroll_height(available_height);
            if let Some(pc) = self.scroll_pc.take() {
                asm_table = asm_table.scroll_to_row(pc, None);
            }
            let ins = &self.instructions;
            asm_table.body(|body| {
                body.rows(text_height, ins.len(), |mut row| {
                    let row_index = row.index();
                    row.set_selected(row_index == self.last_res.pc);
                    row.col(|ui| {
                        ui.checkbox(&mut self.breaks_mac[row_index], "")
                            .on_hover_text("Breakpoint");
                    });
                    row.col(|ui| {
                        if row_index == self.last_res.pc {
                            ui.strong(row_index.to_string());
                        } else {
                            ui.label(row_index.to_string());
                        }
                    });
                    row.col(|ui| {
                        let (text, bin) = ins
                            .get(row_index)
                            .map(|i| (i.content.as_str(), i.bin.as_str()))
                            .unwrap_or(("", ""));
                        let rich_text = egui::RichText::new(text).monospace();
                        let hover_add = if row_index == self.last_res.pc {
                            Some("Próxima instrução")
                        } else if row_index == self.last_res.prev_pc {
                            Some("Instrução executada")
                        } else {
                            None
                        };
                        if let Some(hover_add) = hover_add {
                            ui.label(rich_text.strong())
                                .on_hover_text(format!("{} ({})", bin, hover_add));
                        } else {
                            ui.label(rich_text).on_hover_text(bin);
                        }
                    });
                });
            });
        }
    }

    fn mal_table_ui(&mut self, ui: &mut egui::Ui) {
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);
        let available_height = ui.available_height();
        if !self.microinstructions.is_empty() {
            let mut mal_table = TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::remainder().clip(true))
                .min_scrolled_height(0.0)
                .max_scroll_height(available_height)
                .sense(egui::Sense::click());
            if let Some(mpc) = self.scroll_mpc.take() {
                mal_table = mal_table.scroll_to_row(mpc, None);
            }
            let mics = &self.microinstructions;
            let mpc = self.last_res.mpc;
            let prev_mpc = self.last_res.prev_mpc;
            mal_table.body(|body| {
                body.rows(text_height, mics.len(), |mut row| {
                    let row_index = row.index();
                    row.set_selected(row_index == self.selected);
                    row.col(|ui| {
                        ui.checkbox(&mut self.breaks_mic[row_index], "")
                            .on_hover_text("Breakpoint");
                    });
                    row.col(|ui| {
                        if row_index == mpc {
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
                        if row_index == mpc {
                            ui.label(egui::RichText::new(text).monospace().strong())
                                .on_hover_text("Próxima microinstrução");
                        } else if row_index == prev_mpc {
                            ui.label(egui::RichText::new(text).monospace().strong())
                                .on_hover_text("Microinstrução executada");
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
    }
}

#[derive(Default, PartialEq, Eq)]
enum BottomPanelTab {
    #[default]
    MemTable,
    Stdout,
}

#[derive(Default, PartialEq, Eq)]
enum InstructionTableTab {
    #[default]
    Micro,
    Macro,
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

struct UIInputInboxes {
    request: UiInbox<VMInputRequest>,
    validation: UiInbox<Result<(), String>>,
}

#[derive(Debug)]
enum VMTask {
    // Usando Box pra reduzir o tamanho total do array
    Execute(Box<VMResponse>),
    AssembleMac(Vec<Instruction>),
    AssembleMic(Vec<Microinstruction>),
}
