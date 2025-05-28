src=src/main.rs src/bus.rs src/dram.rs src/cpu.rs

build_apps:apps/test.c apps/fib.s apps/exception.s
	riscv64-unknown-elf-gcc -S apps/test.c
	riscv64-unknown-elf-gcc -Wl,-Ttext=0x0 -nostdlib -o apps/test apps/test.s
	riscv64-unknown-elf-objcopy -O binary apps/test apps/test.bin
	riscv64-unknown-elf-objdump -b binary --adjust-vma=0x0 -D -m riscv:rv64 apps/test.bin > apps/test.dump
	riscv64-unknown-elf-objdump -b binary --adjust-vma=0x0 -D -m riscv:rv64 apps/fib.bin > apps/fib.dump
	riscv64-unknown-elf-gcc -nostdlib -o apps/exception.elf apps/exception.s
	riscv64-unknown-elf-gcc -S -nostdlib -o apps/exception.S apps/exception.s
	riscv64-unknown-elf-objdump -D -m riscv:rv64 apps/exception.elf > apps/exception.dump

test:apps/test.c ${src}
	RUST_LOG=debug cargo run apps/test.bin -c 1000 -d 1 > log/output_test.log 2>&1

fib:apps/fib.s ${src}
	RUST_LOG=debug cargo run apps/fib.bin -c 1000 -d 1 > log/output.log 2>&1

exception:apps/exception.s ${src}
	RUST_LOG=debug cargo run apps/exception.elf --elf -c 100 -d 1 > log/output_exception.log 2>&1

xv6:apps/xv6-riscv/kernel/kernel ${src}
	cargo run --release apps/xv6-riscv/kernel/kernel --elf --base-addr 2147483648 --loop-on --dump 100000000

xv6-gdb:apps/xv6-riscv/kernel/kernel ${src}
	cargo run --release apps/xv6-riscv/kernel/kernel --elf --base-addr 2147483648 --loop-on --dump 100000000 --gdb

run:apps/test.bin apps/fib.bin ${src}
	RUST_LOG=debug cargo run apps/test.bin -c 1000 -d 100 -o log/output_test.log
	RUST_LOG=debug cargo run apps/fib.bin -c 1000 -d 100 -o log/output_fib.log
	# RUST_LOG=debug cargo run apps/csr --elf -c 1000 -d 100 -o log/output_csr.log
	RUST_LOG=debug cargo run apps/exception.elf --elf -c 1000 -d 100 -o log/output_exception.log

all:fib test