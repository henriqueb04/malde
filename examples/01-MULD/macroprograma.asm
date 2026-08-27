.data
ZERO:   .word 0
DOIS:   .word 2
TRES:   .word 3
CINCO:  .word 5

.text
MAIN:
        LOCO 1

        MULD DOIS
        SWAPA
        LOCO 1
        ECALL

        SWAPA
        MULD TRES
        SWAPA
        ECALL

        SWAPA
        MULD CINCO
        SWAPA
        ECALL

        SWAPA
        MULD ZERO
        SWAPA
        ECALL

        SWAPA
        MULD DOIS
        SWAPA
        ECALL

