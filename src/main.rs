#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod architecture;
mod cli;
mod parsers;
mod ui;
mod virtual_machine;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use eframe::egui;
use ui::MyApp;

use crate::parsers::asm::KeywordMap;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Executar programa sem interface gráfica
    #[arg(short, long)]
    pub run: bool,

    /// Arquivo com microprograma em MAL para carregar na micro-memória.
    #[arg(short = 'm', long, value_name = "ARQUIVO")]
    pub microprogram: Option<PathBuf>,

    /// Arquivo com macroprograma em Assembly para carregar na memória principal.
    #[arg(short = 'a', long, value_name = "ARQUIVO")]
    pub macroprogram: Option<PathBuf>,

    /// Arquivo de keywords para serem usadas no Assembly. Usa o formato COMANDO,OPCODE para cada linha.
    #[arg(short, long, value_name = "ARQUIVO")]
    pub keywords: Option<PathBuf>,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    if args.run {
        // CLI execution
        let mic = args
            .microprogram
            .context("É necessário prover um arquivo MAL usando --microprogram")?;
        let mac = args
            .macroprogram
            .context("É necessário prover um arquivo Assembly usando --macroprogram")?;
        cli::execute(
            mic.display().to_string(),
            mac.display().to_string(),
            Some(KeywordMap::default()),
        )?;
        Ok(())
    } else {
        // UI execution
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 240.0]),
            ..Default::default()
        };
        eframe::run_native(
            "MALDE: Simulador de linguagem MAL",
            options,
            Box::new(|_cc| {
                Ok(Box::new(MyApp::new(
                    args.microprogram.map(|f| f.display().to_string()),
                    args.macroprogram.map(|f| f.display().to_string()),
                    None,
                )))
            }),
        )
        .with_context(|| "Erro ao iniciar interface")
    }
}
