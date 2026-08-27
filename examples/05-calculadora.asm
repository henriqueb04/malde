# Esse exemplo mostra uma calculadora de soma e adição
# São pedidos 2 valores do usuário e depois uma operação (+ ou -)
# O resultado da operação é exibido, eu uma mensagem de erro caso uma
# Operação inválida seja selecionada

.data
VAL1:   .word 0
VAL2:   .word 0
OP:     .ascii "+"
FRASE1: .asciz "Digite o primeiro valor: "
FRASE2: .asciz "Digite o segundo valor: "
FRASE3: .asciz "Digite a operação (+ ou -): "
FRASE4: .asciz "Operação inválida!"
FRASE5: .asciz "Resultado: "

.text
MAIN:
        LOCO FRASE1
        SWAPA
        LOCO 3
        ECALL
        LOCO 7
        ECALL
        STOD VAL1

        LOCO FRASE2
        SWAPA
        LOCO 3
        ECALL
        LOCO 7
        ECALL
        STOD VAL2

        LOCO FRASE3
        SWAPA
        LOCO 3
        ECALL
        LOCO 8
        ECALL
        STOD OP

        LOCO FRASE5
        SWAPA
        LOCO 3
        ECALL

        LOCO '+'
        SUBD OP
        JZER SOMA

        LOCO '-'
        SUBD OP
        JZER SUB

        LOCO FRASE4
        SWAPA
        LOCO 3
        ECALL
        LOCO '\n'
        SWAPA
        LOCO 2
        ECALL
        HALT

SOMA:   LODD VAL1
        ADDD VAL2
        SWAPA
        LOCO 1
        ECALL
        LOCO '\n'
        SWAPA
        LOCO 2
        ECALL
        HALT

SUB:    LODD VAL1
        SUBD VAL2
        SWAPA
        LOCO 1
        ECALL
        LOCO '\n'
        SWAPA
        LOCO 2
        ECALL
        HALT
