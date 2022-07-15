src=src/main.rs src/bus.rs src/dram.rs src/cpu.rs

test:apps/test.c ${src}
	riscv64-unknown-elf-gcc -S apps/test.c
	riscv64-unknown-elf-gcc -Wl,-Ttext=0x0 -nostdlib -o apps/test apps/test.s
	riscv64-unknown-elf-objcopy -O binary apps/test apps/test.bin
	riscv64-unknown-elf-objdump -b binary --adjust-vma=0x0 -D -m riscv:rv64 apps/test.bin > apps/test.dump
	cargo run apps/test.bin -c 1000 -d > log/output_test.log

fib:apps/fib.s ${src}
	riscv64-unknown-elf-objdump -b binary --adjust-vma=0x0 -D -m riscv:rv64 apps/fib.bin > apps/fib.dump
	cargo run apps/fib.bin -c 1000 -d > log/output.log

run:apps/test.bin apps/fib.bin ${src}
	cargo run apps/test.bin -c 1000 -d > log/output_test.log
	cargo run apps/fib.bin -c 1000 -d > log/output_fib.log
	cargo run apps/csr --elf -c 1000 -d > log/output_csr.log

all:fib test