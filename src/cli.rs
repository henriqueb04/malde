use std::{
    collections::HashSet,
    io::{self, Write},
};

use anyhow::{Context, Error, Result, bail};
use crossbeam_channel::{select_biased, unbounded};

use crate::{
    parsers::{asm::KeywordMap, source_map::SourceMap},
    virtual_machine::{
        VM, VMExecutionInfo, VMExecutionType, VMInputRequest, VMInputRequestType, VMInputResponse,
    },
};

pub fn execute(
    microprogram: String,
    macroprogram: String,
    keywords: Option<KeywordMap>,
    no_info_print: bool,
) -> Result<()> {
    let mut vm = VM::new();
    let source_map1 = SourceMap::from_filepath(&microprogram)
        .with_context(|| "Erro ao ler arquivo de microprograma.")?;
    let source_map2 = SourceMap::from_filepath(&macroprogram)
        .with_context(|| "Erro ao ler arquivo de macroprograma.")?;
    vm.assemble_mac(&source_map2, keywords.unwrap_or_default())?;
    vm.assemble_mic(&source_map1)?;

    let (_s_dummy_pause, r_dummy_pauser) = unbounded();
    let (s_validation, r_validation) = unbounded();
    let (s_request, r_request) = unbounded();
    vm.set_info_print(!no_info_print);
    vm.set_on_print(Box::new(|msg| {
        print!("{}", msg);
        io::stdout().flush().expect("Erro ao mostrar saída");
    }));

    std::thread::spawn(move || {
        vm.execute(
            VMExecutionType::Run,
            VMExecutionInfo {
                r_pause: r_dummy_pauser,
                on_input_request: Box::new(move |req| {
                    s_request
                        .send(req)
                        .expect("Não foi possível requisitar entrada!");
                }),
                on_validation: Box::new(move |req| {
                    s_validation
                        .send(req)
                        .expect("Não foi possível validar entrada!");
                }),
                breaks_mic: HashSet::new(),
                breaks_mac: HashSet::new(),
            },
        );
    });

    loop {
        select_biased! {
            recv(r_validation) -> msg => match msg {
                Ok(Ok(())) => {},
                Ok(Err(reason)) => return Err(Error::msg(reason)),
                Err(_) => break,
            },
            recv(r_request) -> msg => match msg {
                Ok(request) => read_input(request, no_info_print)?,
                Err(_) => break,
            },
        }
    }
    Ok(())
}

fn read_input(request: VMInputRequest, no_info_print: bool) -> Result<()> {
    if !no_info_print {
        match request.typ {
            VMInputRequestType::Int => {
                print!("Digite um número: ");
            }
            VMInputRequestType::Char => {
                print!("Digite um caractere: ");
            }
            VMInputRequestType::String => {
                print!("Digite um texto: ");
            }
        }
        io::stdout().flush().expect("Erro ao mostrar saída");
    }
    let mut user_input = String::new();
    io::stdin()
        .read_line(&mut user_input)
        .expect("Falha ao ler entrada do usuário");
    user_input.retain(|c| c != '\n');
    match request.typ {
        VMInputRequestType::Int => {
            if let Ok(n) = user_input.parse::<isize>() {
                request.sender.send(VMInputResponse::Int(n))
            } else {
                bail!("Número inválido");
            }
        }
        VMInputRequestType::Char => {
            if user_input.len() == 1 && user_input.is_ascii() {
                request
                    .sender
                    .send(VMInputResponse::Char(user_input.chars().next().unwrap()))
            } else {
                bail!("Deve haver apenas um caractere ASCII");
            }
        }
        VMInputRequestType::String => {
            if user_input.is_ascii() {
                request
                    .sender
                    .send(VMInputResponse::String(user_input.clone()))
            } else {
                bail!("Todos os carateres precisam ser do padrão ASCII");
            }
        }
    }
    .with_context(|| "Erro ao enviar entrada")
}
