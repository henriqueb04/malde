## Multiplicação por somas sucessivas

Esse exemplo demonstra como customizar o microprograma e mudar o conjunto de instruções padrão.

A instrução `ADDL` é substituída por `MULD`, que multiplica o valor do registrador `ac` por um número em um espaço de memória.

Uso no macroprograma:

```asm
.data
X: .word 2

.text
MAIN:   LOCO 5
        MULD X
        # ac agora é 5 * 2
```
