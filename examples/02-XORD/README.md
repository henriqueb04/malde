## XOR bit a bit

Esse exemplo demonstra como customizar o microprograma e mudar o conjunto de instruções padrão.

A instrução `ADDL` é substituída por `XORD`, que aplica a operação xor ao valor de `ac` e um espaço de memória.

Uso no macroprograma:

```asm
.data
X: .word 0b0110111010011001

.text
MAIN:   LOCO 0b0010011000001000  # ac é       0b0010011000001000
        XORD X                   # M[X] é     0b0110111010011001
                                 # ac agora é 0b0100100010010001
```
