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

    pub fn fetch(&self) -> Result<u64, ()> {
        let index = self.pc as usize;
        match self.bus.load(index as u64, 32) {
            Ok(inst) => Ok(inst),
            Err(_) => Err(()),
        }
    }
    pub fn execute(&mut self, inst: u32) -> Result<(), ()> {
        let opcode = inst & 0x7f;
        let rd = ((inst >> 7) & 0x1f) as usize;
        let rs1 = ((inst >> 15) & 0x1f) as usize;
        let rs2 = ((inst >> 20) & 0x1f) as usize;
        let funct3 = ((inst >> 12) & 0x7) as usize;
        let funct7 = ((inst >> 25) & 0x7f) as usize;

        match opcode {
            0x33 => {
                // add
                println!(
                    "{:>#x} : {:>#2x}({}), dest:{}, src1:{}, src2:{}",
                    self.pc, opcode, "add", rd, rs1, rs2
                );
                self.regs[rd] = self.regs[rs1].wrapping_add(self.regs[rs2]);
                Ok(())
            }
            0x13 => {
                let imm = (inst as i32 as i64 >> 20) as u64;
                match funct3 {
                    0x0 => {
                        // addi
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, src:{}, imm:{}",
                            self.pc, opcode, "addi", rd, rs1, imm as i64
                        );
                        self.regs[rd] = self.regs[rs1].wrapping_add(imm);
                    }
                    0x2 => {
                        // slti
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, src:{}, imm:{}",
                            self.pc, opcode, "slti", rd, rs1, imm
                        );
                        let result = if (self.regs[rs1] as i32 as i64) < (imm as i64) {
                            1
                        } else {
                            0
                        };
                        self.regs[rd] = result;
                    }
                    0x3 => {
                        // sltiu
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, src:{}, imm:{}",
                            self.pc, opcode, "sltiu", rd, rs1, imm
                        );
                        let result = if (self.regs[rs1] as i32 as i64 as u64) < imm {
                            1
                        } else {
                            0
                        };
                        self.regs[rd] = result;
                    }
                    0x4 => {
                        // xori
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, src:{}, imm:{}",
                            self.pc, opcode, "xori", rd, rs1, imm
                        );
                        let val = ((self.regs[rs1] as i32) ^ (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x6 => {
                        // ori
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, src:{}, imm:{}",
                            self.pc, opcode, "ori", rd, rs1, imm
                        );
                        let val = ((self.regs[rs1] as i32) | (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x7 => {
                        // andi
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, src:{}, imm:{}",
                            self.pc, opcode, "andi", rd, rs1, imm
                        );
                        let val = ((self.regs[rs1] as i32) & (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x1 => {
                        // slli
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, src:{}, imm:{}",
                            self.pc, opcode, "slli", rd, rs1, imm
                        );
                        let shamt = self.regs[rs2] as u64;
                        self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                    }
                    0x5 => {
                        // srli
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, src:{}, imm:{}",
                            self.pc, opcode, "srli", rd, rs1, imm
                        );
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
            0x03 => {
                // load instructions
                // load a value stored at addr, where addr is RS1 + imm
                let imm = ((inst as i32 as i64) >> 20) as u64;
                let addr = self.regs[rs1].wrapping_add(imm);
                match funct3 {
                    0x0 => {
                        // lb
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, base:{}, imm:{}",
                            self.pc, opcode, "lb", rd, rs1, imm as i64
                        );
                        let val = self.bus.load(addr, 8)?;
                        self.regs[rd] = val as i8 as i64 as u64;
                    }
                    0x1 => {
                        // lh
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, base:{}, imm:{}",
                            self.pc, opcode, "lh", rd, rs1, imm as i64
                        );
                        let val = self.bus.load(addr, 16)?;
                        self.regs[rd] = val as i16 as i64 as u64;
                    }
                    0x2 => {
                        // lw
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, base:{}, imm:{}",
                            self.pc, opcode, "lw", rd, rs1, imm as i64
                        );
                        let val = self.bus.load(addr, 32)?;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    0x3 => {
                        // ld
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, base:{}, imm:{}",
                            self.pc, opcode, "ld", rd, rs1, imm as i64
                        );
                        let val = self.bus.load(addr, 64)?;
                        self.regs[rd] = val;
                    }
                    0x4 => {
                        // lbu
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, base:{}, imm:{}",
                            self.pc, opcode, "lbu", rd, rs1, imm as i64
                        );
                        let val = self.bus.load(addr, 8)?;
                        self.regs[rd] = val;
                    }
                    0x5 => {
                        // lhu
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, base:{}, imm:{}",
                            self.pc, opcode, "lhu", rd, rs1, imm as i64
                        );
                        let val = self.bus.load(addr, 16)?;
                        self.regs[rd] = val;
                    }
                    0x6 => {
                        // lwu
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, base:{}, imm:{}",
                            self.pc, opcode, "lwu", rd, rs1, imm as i64
                        );
                        let val = self.bus.load(addr, 32)?;
                        self.regs[rd] = val;
                    }
                    _ => {}
                }
                Ok(())
            }
            0x23 => {
                // store instructions
                let imm = (((inst & 0xfe000000) as i32 as i64 >> 20) as u64)
                    | ((inst >> 7) & 0x1f) as u64;
                let addr = self.regs[rs1].wrapping_add(imm);
                println!(
                    "{:>#x} : {:>#2x}({}), offset:{}, base:{}, src:{}",
                    self.pc, opcode, "s?", imm as i64, rs1, rs2
                );
                match funct3 {
                    0x0 => self.bus.store(addr, 8, self.regs[rs2])?,
                    0x1 => self.bus.store(addr, 16, self.regs[rs2])?,
                    0x2 => self.bus.store(addr, 32, self.regs[rs2])?,
                    0x3 => self.bus.store(addr, 64, self.regs[rs2])?,
                    _ => {}
                }
                Ok(())
            }
            0x6f => {
                // jal
                let imm = ((inst & 0x80000000) as i32 as i64 >> 11) as u64
                    | ((inst & 0x7fe00000) as u64) >> 20
                    | ((inst & 0x100000) as u64) >> 9
                    | ((inst & 0xff000) as u64);
                println!(
                    "{:>#x} : {:>#2x}({}), dest:{}, offset:{}({:>#x})",
                    self.pc, opcode, "jal", rd, imm as i64, imm as i64
                );
                self.regs[rd] = self.pc.wrapping_add(4);
                self.pc = self.pc.wrapping_add(imm).wrapping_sub(4); // subtract 4 because 4 will be added
                Ok(())
            }
            0x67 => {
                // jalr
                match funct3 {
                    0x0 => {
                        let imm = ((inst as i32 as i64) >> 20) as u64;
                        println!(
                            "{:>#x} : {:>#2x}({}), dest:{}, base:{}, offset:{}({:>#x})",
                            self.pc, opcode, "jalr", rd, rs1, imm, imm
                        );
                        self.regs[rd] = self.pc.wrapping_add(4);
                        self.pc = self.regs[rs1].wrapping_add(imm).wrapping_sub(4);
                        // subtract 4 because 4 will be added
                    }
                    _ => {}
                }
                Ok(())
            }
            0x1b => {
                match funct3 {
                    0x0 => {
                        // addiw
                        // I-type format
                        let imm = (inst as i32) >> 20;
                        println!(
                            "{:>#x} : {:>#2x}({}), rd:{}, rs1:{}, imm:{}({:>#x})",
                            self.pc, opcode, "addiw", rd, rs1, imm, imm
                        );
                        let src = self.regs[rs1] as i32;
                        let val = src.wrapping_add(imm);
                        self.regs[rd] = val as i64 as u64;
                    }
                    _ => {}
                }
                Ok(())
            }
            0x63 => {
                // branch instructions
                let imm = ((inst & 0x80000000) as i32 as i64 >> 19) as u64
                    | ((inst & 0x7e000000) as u64) >> 20
                    | ((inst & 0xf00) as u64) >> 7
                    | ((inst & 0x80) as u64) << 5;
                match funct3 {
                    0x0 => {
                        // beq
                        println!(
                            "{:>#x} : {:>#2x}({}), rs1:{}, rs2:{}, imm:{}({:>#x})",
                            self.pc, opcode, "beq", rs1, rs2, imm as i32, imm as i32
                        );
                        if self.regs[rs1] == self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x1 => {
                        // bne
                        println!(
                            "{:>#x} : {:>#2x}({}), rs1:{}, rs2:{}, imm:{}({:>#x})",
                            self.pc, opcode, "bne", rs1, rs2, imm as i32, imm as i32
                        );
                        if self.regs[rs1] != self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x4 => {
                        // blt
                        println!(
                            "{:>#x} : {:>#2x}({}), rs1:{}, rs2:{}, imm:{}({:>#x})",
                            self.pc, opcode, "blt", rs1, rs2, imm as i32, imm as i32
                        );
                        if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x5 => {
                        // bge
                        println!(
                            "{:>#x} : {:>#2x}({}), rs1:{}, rs2:{}, imm:{}({:>#x})",
                            self.pc, opcode, "bge", rs1, rs2, imm as i32, imm as i32
                        );
                        if (self.regs[rs1] as i64) >= (self.regs[rs2] as i64) {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x6 => {
                        // bltu
                        println!(
                            "{:>#x} : {:>#2x}({}), rs1:{}, rs2:{}, imm:{}({:>#x})",
                            self.pc, opcode, "bltu", rs1, rs2, imm as i32, imm as i32
                        );
                        if self.regs[rs1] < self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x7 => {
                        // bgeu
                        println!(
                            "{:>#x} : {:>#2x}({}), rs1:{}, rs2:{}, imm:{}({:>#x})",
                            self.pc, opcode, "bgeu", rs1, rs2, imm as i32, imm as i32
                        );
                        if self.regs[rs1] >= self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    _ => {}
                }
                Ok(())
            }
            0x3b => {
                match funct3 {
                    0x0 => {
                        if funct7 == 0x0 {
                            // addw
                            println!(
                                "{:>#x} : {:>#2x}({}), rd:{}, rs1:{}, rs2:{}",
                                self.pc, opcode, "addw", rd, rs1, rs2
                            );
                            let add_val = (self.regs[rs1] as i32).wrapping_add(self.regs[rs2] as i32);
                            self.regs[rd] = add_val as i64 as u64;
                        }else if funct7 == 0x20 {
                            // subw
                            println!(
                                "{:>#x} : {:>#2x}({}), rd:{}, rs1:{}, rs2:{}",
                                self.pc, opcode, "subw", rd, rs1, rs2
                            );
                            let add_val = (self.regs[rs1] as i32).wrapping_sub(self.regs[rs2] as i32);
                            self.regs[rd] = add_val as i64 as u64;
                        }else{
                            println!("This should not be reached!");
                            return Err(());
                        }
                    }
                    0x1 => {
                        if funct7 == 0x0 {
                            // sllw
                            println!(
                                "{:>#x} : {:>#2x}({}), dest:{}, rs1:{}, rs2:{}",
                                self.pc, opcode, "sllw", rd, rs1, rs2
                            );
                            let shamt = (self.regs[rs2] as u64) & 0x1f;
                            self.regs[rd] = ((self.regs[rs1] as u32) << shamt) as u64;
                        } else {
                            println!("This should not be reached!");
                            return Err(());
                        }
                    }
                    0x5 => {
                        if funct7 == 0x0 {
                            // srlw
                            println!(
                                "{:>#x} : {:>#2x}({}), dest:{}, rs1:{}, rs2:{}",
                                self.pc, opcode, "srlw", rd, rs1, rs2
                            );
                            let shamt = (self.regs[rs2] as u64) & 0x1f;
                            self.regs[rd] = ((self.regs[rs1] as u32) >> shamt) as u64;
                        }else if funct7 == 0x20 {
                            // sraw
                            println!(
                                "{:>#x} : {:>#2x}({}), dest:{}, rs1:{}, rs2:{}",
                                self.pc, opcode, "sraw", rd, rs1, rs2
                            );
                            let shamt = (self.regs[rs2] as u64) & 0x1f;
                            self.regs[rd] = ((self.regs[rs1] as i32) >> shamt) as i64 as u64;
                        }else{
                            println!("This should not be reached!");
                            return Err(());
                        }
                    }
                        _ => {}
                }
                Ok(())
            }
            _ => {
                println!("not implemented yet!");
                println!("pc=0x{:x}", self.pc);
                println!("inst:{inst:b}");
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
        println!("----");
    }
}
