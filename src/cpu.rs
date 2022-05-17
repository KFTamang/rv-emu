// dram memory size, 128MB
pub const DRAM_SIZE: u64 = 1024 * 1024 * 128;

pub struct Cpu {
    pub regs: [u64; 32],
    pub pc: u64,
    pub dram: Vec<u8>,
}

impl Cpu {
    pub fn new(code: Vec<u8>) -> Self {
        let mut regs = [0; 32];
        regs[2] = DRAM_SIZE;
        Self {
            regs,
            pc: 0,
            dram: code,
        }
    }

    pub fn fetch(&self) -> u32 {
        let index = self.pc as usize;
        return ((self.dram[index + 0] as u32) << 0)
            | ((self.dram[index + 1] as u32) << 8)
            | ((self.dram[index + 2] as u32) << 16)
            | ((self.dram[index + 3] as u32) << 24);
    }
    pub fn execute(&mut self, inst: u32) {
        let opcode = inst & 0x7f;
        let rd = ((inst >> 7) & 0x1f) as usize;
        let rs1 = ((inst >> 15) & 0x1f) as usize;
        let rs2 = ((inst >> 20) & 0x1f) as usize;

        match opcode {
            0x33 => {
                // add
                println!(
                    "opcode:{}({}), rd:{}, rs1:{}, rs2:{}",
                    opcode, "add", rd, rs1, rs2
                );
                self.regs[rd] = self.regs[rs1].wrapping_add(self.regs[rs2]);
            }
            0x13 => {
                // addi
                println!(
                    "opcode:{}({}), rd:{}, rs1:{}, rs2:{}",
                    opcode, "addi", rd, rs1, rs2
                );
                let imm = ((inst >> 20) & 0xfff) as u64;
                self.regs[rd] = self.regs[rs1].wrapping_add(imm);
            }
            _ => {
                dbg!("not implemented yet!");
            }
        }
    }

    pub fn dump_registers(&self) {
        let abi = [
            "zero", " ra ", " sp ", " gp ", " tp ", " t0 ", " t1 ", " t2 ", " s0 ", " s1 ", " a0 ",
            " a1 ", " a2 ", " a3 ", " a4 ", " a5 ", " a6 ", " a7 ", " s2 ", " s3 ", " s4 ", " s5 ",
            " s6 ", " s7 ", " s8 ", " s9 ", " s10", " s11", " t3 ", " t4 ", " t5 ", " t6 ",
        ];
        let mut output = format!("pc={:>#18x}", self.pc);
        for i in (0..32).step_by(4) {
            output = format!(
                "{}\n{}",
                output,
                format!(
                    "x{:02}({})={:>#18x}, x{:02}({})={:>#18x}, x{:02}({})={:>#18x}, x{:02}({})={:>#18x}",
                    i,
                    abi[i],
                    self.regs[i],
                    i+1,
                    abi[i+1],
                    self.regs[i+1],
                    i+2,
                    abi[i+2],
                    self.regs[i+2],
                    i+3,
                    abi[i+3],
                    self.regs[i+3]
                )
            )
        }
        println!("{}", output);
    }
}
