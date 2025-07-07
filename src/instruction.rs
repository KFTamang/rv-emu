use log::{error, info};

pub enum DecodedInstr {
    Add { rd: usize, rs1: usize, rs2: usize },
    Sub {rd: usize, rs1: usize, rs2: usize},
    Sll {rd: usize, rs1: usize, rs2: usize},
    Slt {rd: usize, rs1: usize, rs2: usize},
    Sltu {rd: usize, rs1: usize, rs2: usize},
    Xor {rd: usize, rs1: usize, rs2: usize},
    Srl {rd: usize, rs1: usize, rs2: usize},
    Sra {rd: usize, rs1: usize, rs2: usize},
    Or {rd: usize, rs1: usize, rs2: usize},
    And {rd: usize, rs1: usize, rs2: usize},
    Mul {rd: usize, rs1: usize, rs2: usize},
    Mulh {rd: usize, rs1: usize, rs2: usize},
    Mulhsu {rd: usize, rs1: usize, rs2: usize},
    Mulhu {rd: usize, rs1: usize, rs2: usize},
    Div {rd: usize, rs1: usize, rs2: usize},
    Divu {rd: usize, rs1: usize, rs2: usize},
    Rem {rd: usize, rs1: usize, rs2: usize},
    Remu {rd: usize, rs1: usize, rs2: usize},
    Addi { rd: usize, rs1: usize, imm: u32 },
    Slti { rd: usize, rs1: usize, imm: u32 },
    Sltiu { rd: usize, rs1: usize, imm: u32 },
    Xori { rd: usize, rs1: usize, imm: u32 },
    Ori { rd: usize, rs1: usize, imm: u32 },
    Andi { rd: usize, rs1: usize, imm: u32 },
    Slli { rd: usize, rs1: usize, imm: u32 },
    Srli { rd: usize, rs1: usize, imm: u32 },
    Lb { rd: usize, rs1: usize, imm: u32 },
    Lh { rd: usize, rs1: usize, imm: u32 },
    Lw { rd: usize, rs1: usize, imm: u32 },
    Ld { rd: usize, rs1: usize, imm: u32 },
    Lbu { rd: usize, rs1: usize, imm: u32 },
    Lhu { rd: usize, rs1: usize, imm: u32 },
    Lwu { rd: usize, rs1: usize, imm: u32 },
    Sb { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Sh { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Sw { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Sd { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Jal { rd: usize, imm: u32 },
    Jalr { rd: usize, rs1: usize, imm: u32 },
    Addiw { rd: usize, rs1: usize, imm: u32 },
    Slliw { rd: usize, rs1: usize, imm: u32 },
    Srliw { rd: usize, rs1: usize, imm: u32 },
    Sraiw { rd: usize, rs1: usize, imm: u32 },
    Beq { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Bne { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Blt { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Bge { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Bltu { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Bgeu { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Addw { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Subw { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Sllw { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Srlw { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Sraw { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Mulw { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Divw { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Divuw { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Remw { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Remuw { rd: usize, rs1: usize, rs2: usize, imm: u32 },
    Lui { rd: usize, imm: i32 },
    Auipc { rd: usize, imm: i32 },
    Ecall,
    Ebreak,
    Sret,
    Wfi,
    Mret,
    Csrrw { rd: usize, rs1: usize, imm: usize },
    Csrrs { rd: usize, rs1: usize, imm: usize },
    Csrrc { rd: usize, rs1: usize, imm: usize },
    Csrrwi { rd: usize, rs1: usize, imm: usize },
    Csrrsi { rd: usize, rs1: usize, imm: usize },
    Csrrci { rd: usize, rs1: usize, imm: usize },
    Sfence,
    Fence,
    Amoswap { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amoadd { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amoxor { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amoand { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amoor { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amomin { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amomax { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amominu { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amomaxu { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amoswap { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amoadd { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amoxor { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amoand { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amoor { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amomin { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amomax { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amominu { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    Amomaxu { rd: usize, rs1: usize, rs2: usize, imm: i32 },
    IllegalInstruction { inst: u32 },
}

impl DecodedInstr {
    pub fn decode(inst: u32) -> Self {
                let opcode = inst & 0x7f;
        let rd = ((inst >> 7) & 0x1f) as usize;
        let rs1 = ((inst >> 15) & 0x1f) as usize;
        let rs2 = ((inst >> 20) & 0x1f) as usize;
        let funct3 = ((inst >> 12) & 0x7) as usize;
        let funct7 = ((inst >> 25) & 0x7f) as usize;

        match opcode {
            0x33 => {
                match (funct3, funct7) {
                    (0x0, 0x0) => {
                        DecodedInstr::Add { rd, rs1, rs2 }
                    }
                    (0x0, 0x20) => {
                        DecodedInstr::Sub { rd, rs1, rs2 }
                    }
                    (0x1, 0x0) => {
                        DecodedInstr::Sll { rd, rs1, rs2 }
                    }
                    (0x2, 0x0) => {
                        DecodedInstr::Slt { rd, rs1, rs2 }
                    }
                    (0x3, 0x0) => {
                        DecodedInstr::Sltu { rd, rs1, rs2 }
                    }
                    (0x4, 0x0) => {
                        DecodedInstr::Xor { rd, rs1, rs2 }
                    }
                    (0x5, 0x0) => {
                        DecodedInstr::Srl { rd, rs1, rs2 }
                    }
                    (0x5, 0x20) => {
                        DecodedInstr::Sra { rd, rs1, rs2 }
                    }
                    (0x6, 0x0) => {
                        DecodedInstr::Or { rd, rs1, rs2 }
                    }
                    (0x7, 0x0) => {
                        DecodedInstr::And { rd, rs1, rs2 }
                    }
                    (0x0, 0x1) => {
                        DecodedInstr::Mul { rd, rs1, rs2 }
                    }
                    (0x1, 0x1) => {
                        DecodedInstr::Mulh { rd, rs1, rs2 }
                    }
                    (0x2, 0x1) => {
                        DecodedInstr::Mulhsu { rd, rs1, rs2 }
                    }
                    (0x3, 0x1) => {
                        DecodedInstr::Mulhu { rd, rs1, rs2 }
                    }
                    (0x4, 0x1) => {
                        DecodedInstr::Div { rd, rs1, rs2 }
                    }
                    (0x5, 0x1) => {
                        DecodedInstr::Divu { rd, rs1, rs2 }
                    }
                    (0x6, 0x1) => {
                        DecodedInstr::Rem { rd, rs1, rs2 }
                    }
                    (0x7, 0x1) => {
                        DecodedInstr::Remu { rd, rs1, rs2 }
                    }
                    (_, _) => {
                        error!("This should not be reached!");
                        info!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        DecodedInstr::IllegalInstruction(inst)
                    }
                }
            },
            0x13 => {
                let imm = ((inst as i32 as i64) >> 20) as u32;
                let addr = self.regs[rs1].wrapping_add(imm);
                match funct3 {
                    0x0 => {
                        DecodedInstr::Lb { rd, rs1, imm}
                    }
                    0x1 => {
                        DecodedInstr::Lh { rd, rs1, imm }
                    }
                    0x2 => {
                        DecodedInstr::Lw { rd, rs1, imm }
                    }
                    0x3 => {
                        DecodedInstr::Ld { rd, rs1, imm }
                    }
                    0x4 => {
                        DecodedInstr::Lbu { rd, rs1, imm }
                    }
                    0x5 => {
                        DecodedInstr::Lhu { rd, rs1, imm }
                    }
                    0x6 => {
                        DecodedInstr::Lwu { rd, rs1, imm }
                    }
                    _ => {
                        error!("This should not be reached!");
                        error!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        DecodedInstr::IllegalInstruction{ inst }
                    }
                }
            },
            0x23 => {
                // store instructions
                let imm = (((inst & 0xfe000000) as i32 as i64 >> 20) as u32)
                    | ((inst >> 7) & 0x1f) as u32;
                match funct3 {
                    0x0 => {
                        DecodedInstr::Sb { rd, rs1, rs2, imm }
                    }
                    0x1 => {
                        DecodedInstr::Sb { rd, rs1, rs2, imm }
                    }
                    0x2 => {
                        DecodedInstr::Sb { rd, rs1, rs2, imm }
                    }
                    0x3 => {
                        DecodedInstr::Sb { rd, rs1, rs2, imm }
                    }
                    _ => {
                        error!("This should not be reached!");
                        info!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        DecodedInstr::IllegalInstruction{ inst }
                    }
                }
            }
            0x6f => {
                // jal
                let imm = ((inst & 0x80000000) as i32 >> 11) as u32
                    | ((inst & 0x7fe00000) as u32) >> 20
                    | ((inst & 0x100000) as u32) >> 9
                    | ((inst & 0xff000) as u32);
                // "jal"
                DecodedInstr::Jal { rd, imm }
            }
            0x67 => {
                match funct3 {
                    0x0 => {
                        let imm = ((inst as i32 as i64) >> 20) as u64;
                        return jalr;
                        let return_addr = self.pc.wrapping_add(4);
                        let next_pc = self.regs[rs1].wrapping_add(imm).wrapping_sub(4);
                        // subtract 4 because 4 will be added
                        self.regs[rd] = return_addr;
                        self.pc = next_pc;
                    }
                    _ => {
                        error!("This should not be reached!");
                        error!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            0x1b => {
                match (funct3, funct7) {
                    (0x0, _) => {
                        // addiw
                        // I-type format
                        let imm = (inst as i32) >> 20;
                        return addiw;
                        let src = self.regs[rs1] as i32;
                        let val = src.wrapping_add(imm);
                        self.regs[rd] = val as i64 as u64;
                    }
                    (0x1, 0x0) => {
                        // slliw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        return slliw;
                        let src = self.regs[rs1] as u32;
                        let val = src << shamt;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    (0x5, 0x0) => {
                        // srliw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        return srliw;
                        let src = self.regs[rs1] as u32;
                        let val = src >> shamt;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    (0x5, 0x20) => {
                        // sraiw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        return sraiw;
                        let src = self.regs[rs1] as i32;
                        let val = src >> shamt;
                        self.regs[rd] = val as i64 as u64;
                    }
                    _ => {
                        error!("This should not be reached!");
                        error!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(Exception::IllegalInstruction(inst));
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
                        return beq;
                        if self.regs[rs1] == self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x1 => {
                        return bne;
                        if self.regs[rs1] != self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x4 => {
                        return blt;
                        if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x5 => {
                        return bge;
                        if (self.regs[rs1] as i64) >= (self.regs[rs2] as i64) {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x6 => {
                        return bltu;
                        if self.regs[rs1] < self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x7 => {
                        return bgeu;
                        if self.regs[rs1] >= self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    _ => {
                        error!("This should not be reached!");
                        error!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            0x3b => {
                match (funct3, funct7) {
                    (0x0, 0x0) => {
                        return addw;
                        let add_val = (self.regs[rs1] as i32).wrapping_add(self.regs[rs2] as i32);
                        self.regs[rd] = add_val as i64 as u64;
                    }
                    (0x0, 0x20) => {
                        return subw;
                        let add_val = (self.regs[rs1] as i32).wrapping_sub(self.regs[rs2] as i32);
                        self.regs[rd] = add_val as i64 as u64;
                    }
                    (0x1, 0x0) => {
                        return sllw;
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as u32) << shamt) as u64;
                    }
                    (0x5, 0x0) => {
                        return srlw;
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as u32) >> shamt) as u64;
                    }
                    (0x5, 0x20) => {
                        return sraw;
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as i32) >> shamt) as i64 as u64;
                    }
                    (0x0, 0x1) => {
                        return mulw;
                        let mul = (self.regs[rs2] as u32) * (self.regs[rs2] as u32);
                        self.regs[rd] = mul as i32 as i64 as u64;
                    }
                    (0x4, 0x1) => {
                        return divw;
                        let rem = (self.regs[rs2] as u32) / (self.regs[rs2] as u32);
                        self.regs[rd] = rem as u64;
                    }
                    (0x5, 0x1) => {
                        return divuw;
                        let rem = (self.regs[rs2] as i32) / (self.regs[rs2] as i32);
                        self.regs[rd] = rem as i64 as u64;
                    }
                    (0x6, 0x1) => {
                        return remw;
                        let rem = (self.regs[rs2] as i32) % (self.regs[rs2] as i32);
                        self.regs[rd] = rem as i64 as u64;
                    }
                    (0x7, 0x1) => {
                        return remuw;
                        let rem = (self.regs[rs2] as u32) % (self.regs[rs2] as u32);
                        self.regs[rd] = rem as u64;
                    }
                    _ => {
                        error!("This should not be reached!");
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            0x37 => {
                let imm = (inst & 0xfffff000) as i32 as i64 as u64;
                // "lui"
                self.regs[rd] = imm;
                self.mark_as_dest(rd);
                Ok(())
            }
            0x17 => {
                let imm = inst & 0xfffff000;
                // "auipc"
                self.regs[rd] = imm.wrapping_add(self.pc as u32) as u64;
                self.mark_as_dest(rd);
                Ok(())
            }
            0x73 => {
                let csr = ((inst as u32) >> 20) as usize;
                let uimm = ((inst & 0xf8000) as u32) >> 15;
                let imm = (inst as i32 as i64 >> 20) as u64;
                match (funct3, funct7, rs2) {
                    (0x0, 0x0, 0x0) => {
                        return ecall;
                        Exception::EnvironmentalCallFromMMode.take_trap(self);
                    }
                    (0x0, 0x0, 0x1) => {
                        return ebreak;
                    }
                    (0x0, 0x8, 0x2) => {
                        return sret;
                        self.return_from_trap();
                    }
                    (0x0, 0x8, 0x5) => {
                        return wfi;
                        self.wait_for_interrupt();
                    }
                    (0x0, 0x18, 0x2) => {
                        return mret;
                        self.return_from_trap();
                    }
                    (0x1, _, _) => {
                        return csrrw;
                        if rd != 0 {
                            self.regs[rd] = self.csr.load_csrs(csr) as u64;
                        }
                        self.csr.store_csrs(csr, self.regs[rs1]);
                    }
                    (0x2, _, _) => {
                        return csrrs;
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, self.regs[rs1] | old_val);
                        }
                    }
                    (0x3, _, _) => {
                        return csrrc;
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, self.regs[rs1] & !old_val);
                        }
                    }
                    (0x5, _, _) => {
                        return csrrwi;
                        if rd != 0 {
                            self.regs[rd] = self.csr.load_csrs(csr);
                        }
                        self.csr.store_csrs(csr, uimm as u64);
                    }
                    (0x6, _, _) => {
                        return csrrsi;
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, uimm as u64 | old_val);
                        }
                    }
                    (0x7, _, _) => {
                        return csrrci;
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, uimm as u64 & !old_val);
                        }
                    }
                    (0x0, 0x9, _) => {
                        return sfence;
                        self.address_translation_cache.clear();
                    }
                    (_, _, _) => {
                        error!("Unsupported CSR instruction!");
                        error!("pc = 0x{:x}, funct3:{}, funct7:{}", self.pc, funct3, funct7);
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                Ok(())
            }
            0x0f => {
                self.inst_string = format!("pc=0x{:x}\nfence(do nothing)\n", self.pc);
                Ok(())
            }
            0x2f => {
                // Atomic Operation instructions
                let funct5 = funct7 >> 2;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                match (funct3, funct5) {
                    (0x2, 0x1) => {
                        return amoswap;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: swap
                        self.regs[rs2] = loaded_value;
                        let result = src_value;
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x0, 0x1) => {
                        return amoadd;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: add
                        let result = loaded_value.wrapping_add(src_value);
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x4, 0x1) => {
                        return amoxor;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: xor
                        let result = loaded_value ^ src_value;
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0xc, 0x1) => {
                        return amoand;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: and
                        let result = loaded_value & src_value;
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x8, 0x1) => {
                        return amoor;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: or
                        let result = loaded_value | src_value;
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x10, 0x1) => {
                        return amomin;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: singed min
                        let result = cmp::min(loaded_value as i64, src_value as i64) as u64;
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x14, 0x1) => {
                        return amomax;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: signed max
                        let result = cmp::max(loaded_value as i64, src_value as i64) as u64;
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x18, 0x1) => {
                        return amominu;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: unsigned min
                        let result = cmp::min(loaded_value, src_value);
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x1c, 0x1) => {
                        return amomaxu;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 32)? as i32 as i64 as u64;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: unsigned max
                        let result = cmp::max(loaded_value, src_value);
                        // store operation result
                        self.store(addr, 32, result)?;
                    }
                    (0x2, 0x3) => {
                        return amoswap;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: swap
                        self.regs[rs2] = loaded_value;
                        let result = src_value;
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x0, 0x3) => {
                        return amoadd;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: add
                        let result = loaded_value.wrapping_add(src_value);
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x4, 0x3) => {
                        return amoxor;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: xor
                        let result = loaded_value ^ src_value;
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0xc, 0x3) => {
                        return amoand;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: and
                        let result = loaded_value & src_value;
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x8, 0x3) => {
                        return amoor;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: or
                        let result = loaded_value | src_value;
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x10, 0x3) => {
                        return amomin;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: signed min
                        let result = cmp::min(loaded_value as i64, src_value as i64) as u64;
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x14, 0x3) => {
                        return amomax;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: signed max
                        let result = cmp::max(loaded_value as i64, src_value as i64) as u64;
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x18, 0x3) => {
                        return amominu;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: unsigned min
                        let result = cmp::min(loaded_value, src_value);
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    (0x1c, 0x3) => {
                        return amomaxu;
                        let addr = self.regs[rs1];
                        let loaded_value = self.load(addr, 64)?;
                        let src_value = self.regs[rs2];
                        // store loaded value to dest register
                        self.regs[rd] = loaded_value;
                        // binary operation: unsigned max
                        let result = cmp::max(loaded_value, src_value);
                        // store operation result
                        self.store(addr, 64, result)?;
                    }
                    _ => {
                        DecodedInstr::IllegalInstruction{inst}
                    }
                }
            }
            _ => {
                error!("not implemented yet!");
                // error!("pc=0x{:x}", self.pc);
                error!("inst:{inst:b}");
                return DecodedInstr::IllegalInstruction{inst};
            }
        }
    }
}