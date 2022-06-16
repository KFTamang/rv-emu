src=src/main.rs src/bus.rs src/dram.rs src/cpu.rs

test:test.c ${src}
	riscv64-unknown-elf-gcc -S test.c
	riscv64-unknown-elf-gcc -Wl,-Ttext=0x0 -nostdlib -o test test.s
	riscv64-unknown-elf-objcopy -O binary test test.bin
	cargo run test.bin -c 1000 -d > output_test.log

fib:fib.s ${src}
	cargo run fib.bin -c 1000 -d > output.log

all:fib test