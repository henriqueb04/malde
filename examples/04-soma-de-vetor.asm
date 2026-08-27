# Esse exemplo soma todos os elementos de VETOR, armazenando na variável SUM
# Depois, imprime o valor de SUM

.data
VETOR:  .word 1, 2, 3, 5, 9, 10
SIZE:   .word 6
SUM:    .word 0
CONT:   .word 0

.text
MAIN:   LOCO VETOR
        SWAP
        LOCO 0
LOOP:   LODD CONT
        SUBD SIZE
        JNEG PROX
        JUMP END
PROX:   LODL 0
        ADDD SUM
        STOD SUM
        INSP 1
        LOCO 1
        ADDD CONT
        STOD CONT
        JUMP LOOP

END:    LODD SUM
        SWAPA
        LOCO 1
        ECALL
        HALT
