src=src/main.rs src/bus.rs src/dram.rs src/cpu.rs

test:test.c ${src}
	riscv64-unknown-elf-gcc -S test.c
	riscv64-unknown-elf-gcc -Wl,-Ttext=0x0 -nostdlib -o test test.s
	riscv64-unknown-elf-objcopy -O binary test test.bin
	riscv64-unknown-elf-objdump -b binary --adjust-vma=0x0 -D -m riscv:rv64 test.bin > test.dump
	cargo run test.bin -c 1000 -d > output_test.log

fib:fib.s ${src}
	riscv64-unknown-elf-objdump -b binary --adjust-vma=0x0 -D -m riscv:rv64 fib.bin > fib.dump
	cargo run fib.bin -c 1000 -d > output.log

run:test.bin fib.bin ${src}
	cargo run test.bin -c 1000 -d > output_test.log
	cargo run fib.bin -c 1000 -d > output.log
	cargo run csr.bin -c 1000 -d > output_csr.log

all:fib test