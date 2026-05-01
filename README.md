# MALDE: simulador de arquitetura MIC1
Malde é um simulador da Micro Assembly Language da arquitetura MIC1 Andrew Tanenbaum, com parsers para a linguagem MAL e assembly MAC1.

- **Microprograma** (_MAL_): carregado na micro-memória da unidade de controle
- **Macroprograma** (_ASM_): carregado na meméria principal

## Instalação
Pré-requisitos: `libGL`, `libxkbcommon`, `wayland`, `zenity`

1. Instale o rust

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Baixe os arquivos (clone o repositório)

```sh
git clone https://github.com/henriqueb04/malde
```

3. Instale

```sh
cargo install
```

## Aviso
Esse projeto ainda é apenas um protótipo. Embora tenha funcionalidade básica, ainda não é estável e faltam funções essenciais.

## Exemplo
Esse exemplo percorre uma string, carregando todos os seus caracteres até chegar ao nulo:

```asm
.data
VALOR1: .word 25
FRASE: .asciz "hello, world!"
UM: .word 1
.text
MAIN:   LOCO FRASE
        SUBD UM
        SWAP
LOOP:   INSP 1
        LODL 0
        JNZE LOOP
        HALT
```
