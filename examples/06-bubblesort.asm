.data
VETOR:  .word 6, 1,2,3,4
SIZE:   .word 5
END:    .word 0
I:      .word 0
OK:     .word 0
AUX:    .word 0
UM:     .word 1
MENOSUM:.word -1

.text
        LODD SIZE
        SUBD UM
        STOD END        # end = size - 1
LOOPO:  LODD END
        JNEG PRINT      # if end < 0 then goto PRINT
        LOCO 1
        STOD OK         # ok = 1
        LODD MENOSUM
        STOD I          # i = -1
        LOCO VETOR
        ADDD MENOSUM
        SWAP            # sp = vetor - 1

LOOPI:  INSP 1          # sp++
        LODD I
        ADDD UM
        STOD I          # i++
        SUBD END
        JPOS CHECK      # if i >= end then goto CHECK
        LODL 1
        SUBL 0
        JNEG SWAP       # if vetor[i] > vetor[i+1] then goto SWAP
        JUMP LOOPI      # else goto LOOPI
SWAP:   LODL 0
        STOD AUX
        LODL 1
        STOL 0
        LODD AUX
        STOL 1
        LOCO 0
        STOD OK         # ok = 0
        JUMP LOOPI

CHECK:  LODD OK
        JNZE PRINT      # if ok then goto PRINT
        LODD END
        SUBD UM
        STOD END        # end--
        JUMP LOOPO

PRINT:  LOCO VETOR
        SWAP            # sp = vetor
        LOCO 0
        STOD I          # i = 0
LOOPP:  SUBD SIZE
        JPOS HALT       # if i >= size then goto HALT
        LODL 0
        SWAPA
        LOCO 1
        ECALL           # print vetor[i]
        LOCO ' '
        SWAPA
        LOCO 2
        ECALL           # print ' '
        LODD I
        ADDD UM
        STOD I          # i++
        INSP 1          # sp++
        JUMP LOOPP

HALT:   HALT

