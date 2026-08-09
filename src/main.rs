#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod architecture;
mod cli;
mod parsers;
mod ui;
mod virtual_machine;

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
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

    /// Arquivo de instruções para serem usadas no Assembly. O arquivo usa o formato NOME,OPCODE para cada linha.
    #[arg(short, long, value_name = "ARQUIVO")]
    pub instructions: Option<PathBuf>,

    /// Não exibe mesagens de informação quando o programa encerra, nem mostra mensagens de requisição de entrada (ex.: "Digite um número: ")
    #[arg(short, long)]
    pub no_info_print: bool,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    let keymaps = args
        .instructions
        .map(|p| KeywordMap::from_filename(p))
        .transpose()
        .map_err(|err| {
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
    if args.run {
        // CLI execution
        let mic = args
            .microprogram
            .context("É necessário prover um arquivo MAL usando --microprogram")?;
        let mac = args
            .macroprogram
            .context("É necessário prover um arquivo Assembly usando --macroprogram")?;
        cli::execute(
            mic,
            mac,
            keymaps,
            args.no_info_print,
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
                    args.microprogram,
                    args.macroprogram,
                    keymaps,
                    args.no_info_print,
                )))
            }),
        )
        .with_context(|| "Erro ao iniciar interface")
    }
}
