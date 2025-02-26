# Benchmark pevm

# How to run the test

Benchmark pevm by testing with provided transactions

``` bash
taskset -c -a 0-15 cargo run --release -- --bench
```

Note:
- pevm version used: 4e8ff98e54c76af2c52072ca19b0d96eb5c6b6e9

Caveat:
- Transaction nonce check is skipped by setting `tx.nonce = None`

# Benchmark result

Hardward information: Intel(R) Xeon(R) Gold 5317 CPU @ 3.00GHz / 24 cores


``` bash

```
