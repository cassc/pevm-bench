# Benchmark pevm

# How to run the test

Benchmark pevm by testing with provided transactions

``` bash
JEMALLOC_SYS_WITH_MALLOC_CONF="thp:always,metadata_thp:always" taskset -c -a 0-15 cargo run --release -- --bench
```

Note:
- pevm version used: 4e8ff98e54c76af2c52072ca19b0d96eb5c6b6e9
