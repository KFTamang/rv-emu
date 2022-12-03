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

exception:apps/exception.s ${src}
	riscv64-unknown-elf-gcc -nostdlib -o apps/exception.elf apps/exception.s
	riscv64-unknown-elf-gcc -S -nostdlib -o apps/exception.S apps/exception.s
	riscv64-unknown-elf-objdump -D -m riscv:rv64 apps/exception.elf > apps/exception.dump
	cargo run apps/exception.elf --elf -c 100 -d > log/output_exception.log

xv6:apps/xv6/kernel ${src}
	cargo run apps/xv6/kernel --elf -c 1000 -d > log/output_kernel.log

run:apps/test.bin apps/fib.bin ${src}
	cargo run apps/test.bin -c 1000 -d > log/output_test.log
	cargo run apps/fib.bin -c 1000 -d > log/output_fib.log
	cargo run apps/csr --elf -c 1000 -d > log/output_csr.log
	cargo run apps/exception.elf --elf -c 1000 -d > log/output_exception.log

all:fib test