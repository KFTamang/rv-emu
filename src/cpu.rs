use crate::bus::*;
use crate::csr::*;
use crate::dram::*;
use crate::interrupt::*;

const REG_NUM: usize = 32;
const PRIV_M: u32 = 3;

pub struct Cpu {
    pub regs: [u64; 32],
    pub pc: u64,
    pub bus: Bus,
    csr: Csr,
    dest: usize,
    src1: usize,
    src2: usize,
    priv_level: u32,
    interrupt: Interrupt,
}

impl Cpu {
    pub fn new(binary: Vec<u8>) -> Self {
        let mut regs = [0; 32];
        regs[2] = DRAM_SIZE;
        Self {
            regs,
            pc: DRAM_BASE,
            bus: Bus::new(binary),
            csr: Csr::new(),
            dest: REG_NUM,
            src1: REG_NUM,
            src2: REG_NUM,
            priv_level: PRIV_M,
            interrupt: Interrupt::new(),
        }
    }

    pub fn fetch(&self) -> Result<u64, ()> {
        let index = self.pc as usize;
        match self.bus.load(index as u64, 32) {
            Ok(inst) => Ok(inst),
            Err(_) => Err(()),
        }
    }

    fn print_inst_r(&self, name: &str, rd: usize, rs1: usize, rs2: usize) {
        println!(
            "{:>#x} : {}, dest:{}, rs1:{}, rs2:{}",
            self.pc, name, rd, rs1, rs2
        );
    }

    fn print_inst_i(&self, name: &str, rd: usize, rs1: usize, imm: u64) {
        println!(
            "{:>#x} : {}, rd:{}, rs1:{}, imm:{}({:>#x})",
            self.pc, name, rd, rs1, imm as i32, imm as i32
        );
    }

    fn print_inst_s(&self, name: &str, rs1: usize, rs2: usize, imm: u64) {
        println!(
            "{:>#x} : {}, offset:{}, base:{}, src:{}",
            self.pc, name, imm as i64, rs1, rs2
        );
    }

    fn print_inst_b(&self, name: &str, rs1: usize, rs2: usize, imm: u64) {
        println!(
            "{:>#x} : {}, rs1:{}, rs2:{}, offset:{}",
            self.pc, name, rs1, rs2, imm as i64
        );
    }

    fn print_inst_j(&self, name: &str, rd: usize, imm: u64) {
        println!(
            "{:>#x} : {}, dest:{}, offset:{}({:>#x})",
            self.pc, name, rd, imm as i64, imm as i64
        );
    }

    fn print_inst_csr(&self, name: &str, rd: usize, rs1: usize, csr: u64) {
        println!(
            "{:>#x} : {}, dest:{}, rs1:{}, csr:{}({:>#x})",
            self.pc, name, rd, rs1, csr, csr
        );
    }

    fn print_inst_csri(&self, name: &str, rd: usize, csr: u64, uimm: u64) {
        println!(
            "{:>#x} : {}, dest:{}, csr:{}({:>#x}), uimm:{}({:>#x})",
            self.pc, name, rd, csr, csr, uimm, uimm
        );
    }

    fn mark_as_dest(&mut self, reg: usize) {
        self.dest = reg;
    }

    fn mark_as_src1(&mut self, reg: usize) {
        self.src1 = reg;
    }

    fn mark_as_src2(&mut self, reg: usize) {
        self.src2 = reg;
    }

    fn clear_reg_marks(&mut self) {
        self.dest = REG_NUM;
        self.src1 = REG_NUM;
        self.src2 = REG_NUM;
    }

    pub fn process_interrupt(&self) {
        if self.interrupt.is_pending {
            if ((self.priv_level == PRIV_M) && self.csr.mie()) || (self.priv_level < PRIV_M) {
                self.trap();   
            }
        }
    }

    fn trap(&self) {
        // trap process here
    }

    pub fn execute(&mut self, inst: u32) -> Result<(), ()> {
        let opcode = inst & 0x7f;
        let rd = ((inst >> 7) & 0x1f) as usize;
        let rs1 = ((inst >> 15) & 0x1f) as usize;
        let rs2 = ((inst >> 20) & 0x1f) as usize;
        let funct3 = ((inst >> 12) & 0x7) as usize;
        let funct7 = ((inst >> 25) & 0x7f) as usize;

        self.clear_reg_marks();

        match opcode {
            0x33 => {
                match (funct3, funct7) {
                    (0x0, 0x0) => {
                        self.print_inst_r("add", rd, rs1, rs2);
                        self.regs[rd] = self.regs[rs1].wrapping_add(self.regs[rs2]);
                    }
                    (0x0, 0x20) => {
                        self.print_inst_r("sub", rd, rs1, rs2);
                        self.regs[rd] = self.regs[rs1].wrapping_sub(self.regs[rs2]);
                    }
                    (0x1, 0x0) => {
                        self.print_inst_r("sll", rd, rs1, rs2);
                        let shamt = self.regs[rs2] & 0x1f;
                        self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                    }
                    (0x2, 0x0) => {
                        self.print_inst_r("slt", rd, rs1, rs2);
                        self.regs[rd] = if (rs1 as i64) < (rs2 as i64) { 1 } else { 0 }
                    }
                    (0x3, 0x0) => {
                        self.print_inst_r("sltu", rd, rs1, rs2);
                        self.regs[rd] = if (rs1 as u64) < (rs2 as u64) { 1 } else { 0 }
                    }
                    (0x4, 0x0) => {
                        self.print_inst_r("xor", rd, rs1, rs2);
                        self.regs[rd] = self.regs[rs1] ^ self.regs[rs2];
                    }
                    (0x5, 0x0) => {
                        self.print_inst_r("srl", rd, rs1, rs2);
                        let shamt = self.regs[rs2] & 0x1f;
                        self.regs[rd] = self.regs[rs1] as u64 >> shamt;
                    }
                    (0x5, 0x20) => {
                        self.print_inst_r("sra", rd, rs1, rs2);
                        let shamt = self.regs[rs2] & 0x1f;
                        self.regs[rd] = (self.regs[rs1] as i64 as u64) >> shamt;
                    }
                    (0x6, 0x0) => {
                        self.print_inst_r("or", rd, rs1, rs2);
                        self.regs[rd] = self.regs[rs1] | self.regs[rs2];
                    }
                    (0x7, 0x0) => {
                        self.print_inst_r("and", rd, rs1, rs2);
                        self.regs[rd] = self.regs[rs1] & self.regs[rs2];
                    }
                    (_, _) => {
                        println!("This should not be reached!");
                        println!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(());
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            0x13 => {
                let imm = (inst as i32 as i64 >> 20) as u64;
                match funct3 {
                    0x0 => {
                        self.print_inst_i("addi", rd, rs1, imm);
                        self.regs[rd] = self.regs[rs1].wrapping_add(imm);
                    }
                    0x2 => {
                        self.print_inst_i("slti", rd, rs1, imm);
                        let result = if (self.regs[rs1] as i32 as i64) < (imm as i64) {
                            1
                        } else {
                            0
                        };
                        self.regs[rd] = result;
                    }
                    0x3 => {
                        self.print_inst_i("sltiu", rd, rs1, imm);
                        let result = if (self.regs[rs1] as i32 as i64 as u64) < imm {
                            1
                        } else {
                            0
                        };
                        self.regs[rd] = result;
                    }
                    0x4 => {
                        self.print_inst_i("xori", rd, rs1, imm);
                        let val = ((self.regs[rs1] as i32) ^ (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x6 => {
                        self.print_inst_i("ori", rd, rs1, imm);
                        let val = ((self.regs[rs1] as i32) | (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x7 => {
                        self.print_inst_i("andi", rd, rs1, imm);
                        let val = ((self.regs[rs1] as i32) & (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x1 => {
                        self.print_inst_i("slli", rd, rs1, imm);
                        let shamt = self.regs[rs2] as u64;
                        self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                    }
                    0x5 => {
                        self.print_inst_i("srli", rd, rs1, imm);
                        let shamt = self.regs[rs2] as u64;
                        let logical_shift = (imm >> 10) & 0x1;
                        if logical_shift != 0 {
                            self.regs[rd] = (self.regs[rs1] as u64) >> shamt;
                        } else {
                            self.regs[rd] = ((self.regs[rs1] as i64) >> shamt) as u64;
                        }
                    }
                    _ => {
                        println!("This should not be reached!");
                        println!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(());
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            0x03 => {
                // load instructions
                // load a value stored at addr, where addr is RS1 + imm
                let imm = ((inst as i32 as i64) >> 20) as u64;
                let addr = self.regs[rs1].wrapping_add(imm);
                match funct3 {
                    0x0 => {
                        self.print_inst_i("lb", rd, rs1, imm);
                        let val = self.bus.load(addr, 8)?;
                        self.regs[rd] = val as i8 as i64 as u64;
                    }
                    0x1 => {
                        self.print_inst_i("lh", rd, rs1, imm);
                        let val = self.bus.load(addr, 16)?;
                        self.regs[rd] = val as i16 as i64 as u64;
                    }
                    0x2 => {
                        self.print_inst_i("lw", rd, rs1, imm);
                        let val = self.bus.load(addr, 32)?;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    0x3 => {
                        self.print_inst_i("ld", rd, rs1, imm);
                        let val = self.bus.load(addr, 64)?;
                        self.regs[rd] = val;
                    }
                    0x4 => {
                        self.print_inst_i("lbu", rd, rs1, imm);
                        let val = self.bus.load(addr, 8)?;
                        self.regs[rd] = val;
                    }
                    0x5 => {
                        self.print_inst_i("lhu", rd, rs1, imm);
                        let val = self.bus.load(addr, 16)?;
                        self.regs[rd] = val;
                    }
                    0x6 => {
                        self.print_inst_i("lwu", rd, rs1, imm);
                        let val = self.bus.load(addr, 32)?;
                        self.regs[rd] = val;
                    }
                    _ => {
                        println!("This should not be reached!");
                        println!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(());
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            0x23 => {
                // store instructions
                let imm = (((inst & 0xfe000000) as i32 as i64 >> 20) as u64)
                    | ((inst >> 7) & 0x1f) as u64;
                let addr = self.regs[rs1].wrapping_add(imm);
                self.print_inst_s("s?", rs1, rs2, imm);
                match funct3 {
                    0x0 => self.bus.store(addr, 8, self.regs[rs2])?,
                    0x1 => self.bus.store(addr, 16, self.regs[rs2])?,
                    0x2 => self.bus.store(addr, 32, self.regs[rs2])?,
                    0x3 => self.bus.store(addr, 64, self.regs[rs2])?,
                    _ => {
                        println!("This should not be reached!");
                        println!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(());
                    }
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            0x6f => {
                // jal
                let imm = ((inst & 0x80000000) as i32 as i64 >> 11) as u64
                    | ((inst & 0x7fe00000) as u64) >> 20
                    | ((inst & 0x100000) as u64) >> 9
                    | ((inst & 0xff000) as u64);
                self.print_inst_j("jal", rd, imm);
                self.regs[rd] = self.pc.wrapping_add(4);
                self.pc = self.pc.wrapping_add(imm).wrapping_sub(4); // subtract 4 because 4 will be added
                self.mark_as_dest(rd);
                Ok(())
            }
            0x67 => {
                match funct3 {
                    0x0 => {
                        let imm = ((inst as i32 as i64) >> 20) as u64;
                        self.print_inst_i("jalr", rd, rs1, imm);
                        self.regs[rd] = self.pc.wrapping_add(4);
                        self.pc = self.regs[rs1].wrapping_add(imm).wrapping_sub(4);
                        // subtract 4 because 4 will be added
                    }
                    _ => {
                        println!("This should not be reached!");
                        println!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(());
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            0x1b => {
                match funct3 {
                    0x0 => {
                        // addiw
                        // I-type format
                        let imm = (inst as i32) >> 20;
                        self.print_inst_i("addiw", rd, rs1, imm as u32 as u64);
                        let src = self.regs[rs1] as i32;
                        let val = src.wrapping_add(imm);
                        self.regs[rd] = val as i64 as u64;
                    }
                    _ => {
                        println!("This should not be reached!");
                        println!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(());
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            0x63 => {
                // branch instructions
                let imm = ((inst & 0x80000000) as i32 as i64 >> 19) as u64
                    | ((inst & 0x7e000000) as u64) >> 20
                    | ((inst & 0xf00) as u64) >> 7
                    | ((inst & 0x80) as u64) << 4;
                match funct3 {
                    0x0 => {
                        self.print_inst_b("beq", rs1, rs2, imm);
                        if self.regs[rs1] == self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x1 => {
                        self.print_inst_b("bne", rs1, rs2, imm);
                        if self.regs[rs1] != self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x4 => {
                        self.print_inst_b("blt", rs1, rs2, imm);
                        if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x5 => {
                        self.print_inst_b("bge", rs1, rs2, imm);
                        if (self.regs[rs1] as i64) >= (self.regs[rs2] as i64) {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x6 => {
                        self.print_inst_b("bltu", rs1, rs2, imm);
                        if self.regs[rs1] < self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x7 => {
                        self.print_inst_b("bgeu", rs1, rs2, imm);
                        if self.regs[rs1] >= self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    _ => {
                        println!("This should not be reached!");
                        println!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(());
                    }
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            0x3b => {
                match (funct3, funct7) {
                    (0x0, 0x0) => {
                        self.print_inst_r("addw", rd, rs1, rs2);
                        let add_val = (self.regs[rs1] as i32).wrapping_add(self.regs[rs2] as i32);
                        self.regs[rd] = add_val as i64 as u64;
                    }
                    (0x0, 0x20) => {
                        self.print_inst_r("subw", rd, rs1, rs2);
                        let add_val = (self.regs[rs1] as i32).wrapping_sub(self.regs[rs2] as i32);
                        self.regs[rd] = add_val as i64 as u64;
                    }
                    (0x1, 0x0) => {
                        self.print_inst_r("sllw", rd, rs1, rs2);
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as u32) << shamt) as u64;
                    }
                    (0x5, 0x0) => {
                        self.print_inst_r("srlw", rd, rs1, rs2);
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as u32) >> shamt) as u64;
                    }
                    (0x5, 0x20) => {
                        self.print_inst_r("sraw", rd, rs1, rs2);
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as i32) >> shamt) as i64 as u64;
                    }
                    (0x4, 0x1) => {
                        self.print_inst_r("divw", rd, rs1, rs2);
                        let rem = (self.regs[rs2] as u32) / (self.regs[rs2] as u32);
                        self.regs[rd] = rem as u64;
                    }
                    (0x5, 0x1) => {
                        self.print_inst_r("divuw", rd, rs1, rs2);
                        let rem = (self.regs[rs2] as i32) / (self.regs[rs2] as i32);
                        self.regs[rd] = rem as i64 as u64;
                    }
                    (0x6, 0x1) => {
                        self.print_inst_r("remw", rd, rs1, rs2);
                        let rem = (self.regs[rs2] as i32) % (self.regs[rs2] as i32);
                        self.regs[rd] = rem as i64 as u64;
                    }
                    (0x7, 0x1) => {
                        self.print_inst_r("remuw", rd, rs1, rs2);
                        let rem = (self.regs[rs2] as u32) % (self.regs[rs2] as u32);
                        self.regs[rd] = rem as u64;
                    }
                    _ => {
                        println!("This should not be reached!");
                        return Err(());
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            0x37 => {
                let imm = inst & 0xfffff000;
                self.print_inst_j("lui", rd, imm as u64);
                self.regs[rd] = imm as u64;
                self.mark_as_dest(rd);
                Ok(())
            }
            0x27 => {
                let imm = inst & 0xfffff000;
                self.print_inst_j("lui", rd, imm as u64);
                self.regs[rd] = imm.wrapping_add(self.pc as u32) as u64;
                self.mark_as_dest(rd);
                Ok(())
            }
            0x73 => {
                let csr = ((inst as u32) >> 20) as usize;
                let uimm = ((inst & 0xf8000) as u32) >> 15;
                match funct3 {
                    0x1 => {
                        self.print_inst_csr("csrrw", rd, rs1, csr as u64);
                        if rd != 0 {
                            self.regs[rd] = self.csr.load_csrs(csr) as u64;
                        }
                        self.csr.store_csrs(csr, self.regs[rs1]);
                    }
                    0x2 => {
                        self.print_inst_csr("csrrs", rd, rs1, csr as u64);
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, self.regs[rs1] | old_val);
                        }
                    }
                    0x3 => {
                        self.print_inst_csr("csrrc", rd, rs1, csr as u64);
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, self.regs[rs1] & !old_val);
                        }
                    }
                    0x5 => {
                        self.print_inst_csri("csrrwi", rd, csr as u64, uimm as u64);
                        if rd != 0 {
                            self.regs[rd] = self.csr.load_csrs(csr);
                        }
                        self.csr.store_csrs(csr, uimm as u64);
                    }
                    0x6 => {
                        self.print_inst_csri("csrrsi", rd, csr as u64, uimm as u64);
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, uimm as u64 | old_val);
                        }
                    }
                    0x7 => {
                        self.print_inst_csri("csrrci", rd, csr as u64, uimm as u64);
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, uimm as u64 & !old_val);
                        }
                    }
                    _ => {
                        println!("Unsupported CSR instruction!");
                        println!("funct3:{}, funct7:{}", funct3, funct7);
                        return Err(());
                    }
                }
                Ok(())
            }
            0x0f => {
                println!("pc=0x{:x}", self.pc);
                println!("fence(do nothing)");
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
        let mut output = format!("pc={:>#18x}\n", self.pc);
        const SEQ_RED: &str = "\x1b[91m";
        const SEQ_GREEN: &str = "\x1b[92m";
        const SEQ_CLEAR: &str = "\x1b[0m";
        for i in 0..32 {
            output = format!(
                "{}{}",
                output,
                format!(
                    "{}x{:02}({})={:>#18x}{}{}",
                    if i == self.dest {
                        SEQ_RED
                    } else if (i == self.src1) || (i == self.src2) {
                        SEQ_GREEN
                    } else {
                        ""
                    },
                    i,
                    abi[i],
                    self.regs[i],
                    if (i == self.dest) || (i == self.src1) || (i == self.src2) {
                        SEQ_CLEAR
                    } else {
                        ""
                    },
                    if i % 4 == 3 { "\n" } else { ", " }
                )
            )
        }
        print!("{}", output);
        println!("----");
    }
}
