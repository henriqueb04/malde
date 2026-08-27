# Esse exemplo imprime a frase "Hello, world!",
# Depois pergunta qual é o nome do usuário e cumprimenta ele com base na resposta

.data
FRASE1: .asciz "Hello, world!\n"
FRASE2: .asciz "Qual eh seu nome? "
FRASE3: .asciz "Ola, "
NOME:   .space 100

.text
        LOCO FRASE1
        SWAPA
        LOCO 3
        ECALL

        LOCO FRASE2
        SWAPA
        LOCO 3
        ECALL

        LOCO NOME
        SWAPA
        LOCO 50
        SWAPB
        LOCO 9
        ECALL

        LOCO FRASE3
        SWAPA
        LOCO 3
        ECALL

        LOCO NOME
        SWAPA
        LOCO 3
        ECALL

        HALT
