.data
VAL1:   .word 0
VAL2:   .word 0
RES:    .word 0
FRASE1: .asciz "Digite o primeiro valor: "
FRASE2: .asciz "Digite o segundo valor: "
FRASE3: .asciz "Valor 1  : "
FRASE4: .asciz "Valor 2  : "
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

        LODD VAL1
        SWAPA
        LOCO 5
        ECALL

        LOCO '\n'
        SWAPA
        LOCO 2
        ECALL

        LOCO FRASE4
        SWAPA
        LOCO 3
        ECALL

        LODD VAL2
        SWAPA
        LOCO 5
        ECALL

        LOCO '\n'
        SWAPA
        LOCO 2
        ECALL

        LOCO FRASE5
        SWAPA
        LOCO 3
        ECALL

        LODD VAL1
        XORD VAL2
        SWAPA
        LOCO 5
        ECALL
        HALT

