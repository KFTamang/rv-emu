use crate::bus::*;
use crate::dram::*;

pub struct Cpu {
    pub regs: [u64; 32],
    pub pc: u64,
    pub bus: Bus,
}

impl Cpu {
    pub fn new(binary: Vec<u8>) -> Self {
        let mut regs = [0; 32];
        regs[2] = DRAM_SIZE;
        Self {
            regs,
            pc: DRAM_BASE,
            bus: Bus::new(binary),
        }
    }

    pub fn fetch(&self) -> Result<u64,()> {
        let index = self.pc as usize;
        match self.bus.load(index as u64, 32) {
            Ok(inst) => Ok(inst),
            Err(_) => Err(()),
        }
    }
    pub fn execute(&mut self, inst: u32) -> Result<(),()> {
        let opcode = inst & 0x7f;
        let rd = ((inst >> 7) & 0x1f) as usize;
        let rs1 = ((inst >> 15) & 0x1f) as usize;
        let rs2 = ((inst >> 20) & 0x1f) as usize;
        let funct3 = ((inst >> 12) & 0x7) as usize;

        match opcode {
            0x33 => {
                // add
                println!(
                    "opcode:{}({}), rd:{}, rs1:{}, rs2:{}",
                    opcode, "add", rd, rs1, rs2
                );
                self.regs[rd] = self.regs[rs1].wrapping_add(self.regs[rs2]);
                Ok(())
            }
            0x13 => {
                let imm = ((inst as i32 as i64 >> 20) & 0xfff) as u64;
                match funct3 {
                    0x0 => {
                        // addi
                        println!(
                            "opcode:{}({}), rd:{}, rs1:{}, rs2:{}",
                            opcode, "addi", rd, rs1, rs2
                        );
                        self.regs[rd] = self.regs[rs1].wrapping_add(imm);
                    }
                    0x2 => {
                        // slti
                        let result = if (self.regs[rs1] as i32 as i64) < (imm as i64) {1} else {0};
                        self.regs[rd] = result;
                    }
                    0x3 => {
                        // sltiu
                        let result = if (self.regs[rs1] as i32 as i64 as u64) < imm {1} else {0};
                        self.regs[rd] = result;
                    }
                    0x4 => {
                        // xori
                        let val = ((self.regs[rs1] as i32) ^ (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x6 => {
                        // ori
                        let val = ((self.regs[rs1] as i32) | (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }                    
                    0x7 => {
                        // andi
                        let val = ((self.regs[rs1] as i32) & (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x1 => {
                        // slli
                        let shamt = self.regs[rs2] as u64;
                        self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                    }
                    0x5 => {
                        // srli
                        let shamt = self.regs[rs2] as u64;
                        let logical_shift = (imm >> 10) & 0x1;
                        if logical_shift != 0 {
                            self.regs[rd] = (self.regs[rs1] as u64) >> shamt;
                        } else {
                            self.regs[rd] = ((self.regs[rs1] as i64) >> shamt) as u64;
                        }
                    }
                    _ => {}
                }
                Ok(())
            }
            0x03 =>{
                // load instructions
                // load a value stored at addr, where addr is RS1 + imm
                let imm = ((inst as i32 as i64) >> 20) as u64;
                let addr = self.regs[rs1].wrapping_add(imm); 
                match funct3 {
                    0x0 => { // lb
                        let val = self.bus.load(addr, 8)?;
                        self.regs[rd] = val as i8 as i64 as u64;
                    }
                    0x1 => { // lh
                        let val = self.bus.load(addr, 16)?;
                        self.regs[rd] = val as i16 as i64 as u64;
                    }
                    0x2 => { // lw
                        let val = self.bus.load(addr, 32)?;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    0x3 => { // lw
                        let val = self.bus.load(addr, 64)?;
                        self.regs[rd] = val;
                    }
                    0x4 => { // lbu
                        let val = self.bus.load(addr, 8)?;
                        self.regs[rd] = val;
                    }
                    0x5 => { // lhu
                        let val = self.bus.load(addr, 16)?;
                        self.regs[rd] = val;
                    }
                    0x6 => { // lwu
                        let val = self.bus.load(addr, 32)?;
                        self.regs[rd] = val;
                    }
                    _ => {}
                }
                Ok(())
            }
            0x23 => {
                // store instructions
                let imm = (((inst & 0xfe000000) as i32 as i64 >> 20) as u64 ) | ((inst >> 7) & 0x1f) as u64;
                let addr = self.regs[rs1].wrapping_add(imm);
                match funct3 {
                    0x0 => self.bus.store(addr,  8, self.regs[rs2])?,
                    0x1 => self.bus.store(addr, 16, self.regs[rs2])?,
                    0x2 => self.bus.store(addr, 32, self.regs[rs2])?,
                    0x3 => self.bus.store(addr, 64, self.regs[rs2])?,
                    _ => {}
                }
                Ok(())
            }
            0x6f => {
                // jal
                let tmp = inst as u64;
                let imm = ((tmp >> 11) & 0x100000) | ((tmp >> 20) & 0x7fe) | ((tmp >> 9) & 0x800) | (tmp & 0xff000);
                self.regs[rd] = self.pc;
                self.pc = self.pc.wrapping_add(imm);
                Ok(())
            }
            0x67 => {
                // jalr
                match funct3 {
                    0x0 => {
                        let imm = ((inst as i32 as i64) >> 20) as u64;
                        let offset = self.regs[rs1].wrapping_add(imm);
                        self.regs[rd] = self.pc;
                        self.pc = self.pc.wrapping_add(offset);
                    }
                    _ => {}
                }
                Ok(())
            }
            _ => {
                dbg!("not implemented yet!");
                Err(())
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
