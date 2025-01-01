FROM rust:1.67

WORKDIR /usr/src/rv-emu
COPY . .

RUN cargo install --path .

# RUN cargo run --release apps/xv6/kernel --elf --base-addr 2147483648 --loop-on -c 100000 -o log/output_kernel.log

# CMD ["rv-emu apps/xv6/kernel --elf --base-addr 2147483648 --loop-on -c 100000 -o log/output_kernel.log"]
