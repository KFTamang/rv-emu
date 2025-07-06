pub enum DecodedInstr {
    Addi { rd: u8, rs1: u8, imm: i32 },
}

impl DecodedInstr {
    pub fn decode(inst: u32) -> Self {
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
                        // "add"
                        self.regs[rd] = self.regs[rs1].wrapping_add(self.regs[rs2]);
                    }
                    (0x0, 0x20) => {
                        // "sub"
                        self.regs[rd] = self.regs[rs1].wrapping_sub(self.regs[rs2]);
                    }
                    (0x1, 0x0) => {
                        // "sll"
                        let shamt = self.regs[rs2] & 0x1f;
                        self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                    }
                    (0x2, 0x0) => {
                        // "slt"
                        self.regs[rd] = if (rs1 as i64) < (rs2 as i64) { 1 } else { 0 }
                    }
                    (0x3, 0x0) => {
                        // "sltu"
                        self.regs[rd] = if (rs1 as u64) < (rs2 as u64) { 1 } else { 0 }
                    }
                    (0x4, 0x0) => {
                        // "xor"
                        self.regs[rd] = self.regs[rs1] ^ self.regs[rs2];
                    }
                    (0x5, 0x0) => {
                        // "srl"
                        let shamt = self.regs[rs2] & 0x1f;
                        self.regs[rd] = self.regs[rs1] as u64 >> shamt;
                    }
                    (0x5, 0x20) => {
                        // "sra"
                        let shamt = self.regs[rs2] & 0x1f;
                        self.regs[rd] = (self.regs[rs1] as i64 as u64) >> shamt;
                    }
                    (0x6, 0x0) => {
                        // "or"
                        self.regs[rd] = self.regs[rs1] | self.regs[rs2];
                    }
                    (0x7, 0x0) => {
                        // "and"
                        self.regs[rd] = self.regs[rs1] & self.regs[rs2];
                    }
                    (0x0, 0x1) => {
                        // "mul"
                        self.regs[rd] = self.regs[rs1].wrapping_mul(self.regs[rs2]);
                    }
                    (0x1, 0x1) => {
                        // "mulh"
                        let mul = (self.regs[rs1] as i64 as i128)
                            .wrapping_mul(self.regs[rs2] as i64 as i128);
                        self.regs[rd] = (mul >> 64) as u64;
                    }
                    (0x2, 0x1) => {
                        // "mulhsu"
                        let mul = (self.regs[rs1] as i64 as i128)
                            .wrapping_mul(self.regs[rs2] as u128 as i128);
                        self.regs[rd] = (mul >> 64) as u64;
                    }
                    (0x3, 0x1) => {
                        // "mulhu"
                        let mul = (self.regs[rs1] as u128).wrapping_mul(self.regs[rs2] as u128);
                        self.regs[rd] = (mul >> 64) as u64;
                    }
                    (0x4, 0x1) => {
                        // "div"
                        self.regs[rd] = self.regs[rs1] / self.regs[rs2];
                    }
                    (0x5, 0x1) => {
                        // "divu"
                        self.regs[rd] = ((self.regs[rs1] as i64) / (self.regs[rs2] as i64)) as u64;
                    }
                    (0x6, 0x1) => {
                        // "rem"
                        self.regs[rd] = self.regs[rs1] % self.regs[rs2];
                    }
                    (0x7, 0x1) => {
                        // "remu"
                        self.regs[rd] = ((self.regs[rs1] as i64) % (self.regs[rs2] as i64)) as u64;
                    }
                    (_, _) => {
                        error!("This should not be reached!");
                        info!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(Exception::IllegalInstruction(inst));
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
                        // "addi"
                        self.regs[rd] = self.regs[rs1].wrapping_add(imm);
                    }
                    0x2 => {
                        // "slti"
                        let result = if (self.regs[rs1] as i32 as i64) < (imm as i64) {
                            1
                        } else {
                            0
                        };
                        self.regs[rd] = result;
                    }
                    0x3 => {
                        // "sltiu"
                        let result = if (self.regs[rs1] as i32 as i64 as u64) < imm {
                            1
                        } else {
                            0
                        };
                        self.regs[rd] = result;
                    }
                    0x4 => {
                        // "xori"
                        let val = ((self.regs[rs1] as i32) ^ (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x6 => {
                        // "ori"
                        let val = ((self.regs[rs1] as i32) | (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x7 => {
                        // "andi"
                        let val = ((self.regs[rs1] as i32) & (imm as i32)) as u64;
                        self.regs[rd] = val;
                    }
                    0x1 => {
                        // "slli"
                        let shamt = (imm & 0x3f) as u64;
                        self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                    }
                    0x5 => {
                        // "srli/
                        let shamt = (imm & 0x3f) as u64;
                        let logical_shift = imm >> 5;
                        if logical_shift == 0 {
                            self.regs[rd] = (self.regs[rs1] as u64) >> shamt;
                        } else {
                            self.regs[rd] = ((self.regs[rs1] as i64) >> shamt) as u64;
                        }
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
            0x03 => {
                // load instructions
                // load a value stored at addr, where addr is RS1 + imm
                let imm = ((inst as i32 as i64) >> 20) as u64;
                let addr = self.regs[rs1].wrapping_add(imm);
                match funct3 {
                    0x0 => {
                        // "lb"
                        let val = self.load(addr, 8)?;
                        self.regs[rd] = val as i8 as i64 as u64;
                    }
                    0x1 => {
                        // "lh"
                        let val = self.load(addr, 16)?;
                        self.regs[rd] = val as i16 as i64 as u64;
                    }
                    0x2 => {
                        // "lw"
                        let val = self.load(addr, 32)?;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    0x3 => {
                        // "ld"
                        let val = self.load(addr, 64)?;
                        self.regs[rd] = val;
                    }
                    0x4 => {
                        // "lbu"
                        let val = self.load(addr, 8)?;
                        self.regs[rd] = val;
                    }
                    0x5 => {
                        // "lhu"
                        let val = self.load(addr, 16)?;
                        self.regs[rd] = val;
                    }
                    0x6 => {
                        // "lwu"
                        let val = self.load(addr, 32)?;
                        self.regs[rd] = val;
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
            0x23 => {
                // store instructions
                let imm = (((inst & 0xfe000000) as i32 as i64 >> 20) as u64)
                    | ((inst >> 7) & 0x1f) as u64;
                let addr = self.regs[rs1].wrapping_add(imm);
                // "s?",
                match funct3 {
                    0x0 => self.store(addr, 8, self.regs[rs2])?,
                    0x1 => self.store(addr, 16, self.regs[rs2])?,
                    0x2 => self.store(addr, 32, self.regs[rs2])?,
                    0x3 => self.store(addr, 64, self.regs[rs2])?,
                    _ => {
                        error!("This should not be reached!");
                        info!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        return Err(Exception::IllegalInstruction(inst));
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
                // "jal"
                self.regs[rd] = self.pc.wrapping_add(4);
                self.pc = self.pc.wrapping_add(imm).wrapping_sub(4); // subtract 4 because 4 will be added
                self.mark_as_dest(rd);
                Ok(())
            }
            0x67 => {
                match funct3 {
                    0x0 => {
                        let imm = ((inst as i32 as i64) >> 20) as u64;
                        // "jalr"
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
                        // "addiw"
                        let src = self.regs[rs1] as i32;
                        let val = src.wrapping_add(imm);
                        self.regs[rd] = val as i64 as u64;
                    }
                    (0x1, 0x0) => {
                        // slliw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        // "slliw"
                        let src = self.regs[rs1] as u32;
                        let val = src << shamt;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    (0x5, 0x0) => {
                        // srliw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        // "srliw"
                        let src = self.regs[rs1] as u32;
                        let val = src >> shamt;
                        self.regs[rd] = val as i32 as i64 as u64;
                    }
                    (0x5, 0x20) => {
                        // sraiw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        // "sraiw"
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
                        // "beq"
                        if self.regs[rs1] == self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x1 => {
                        // "bne"
                        if self.regs[rs1] != self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x4 => {
                        // "blt"
                        if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x5 => {
                        // "bge"
                        if (self.regs[rs1] as i64) >= (self.regs[rs2] as i64) {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x6 => {
                        // "bltu"
                        if self.regs[rs1] < self.regs[rs2] {
                            self.pc = self.pc.wrapping_add(imm).wrapping_sub(4);
                        }
                    }
                    0x7 => {
                        // "bgeu"
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
                        // "addw"
                        let add_val = (self.regs[rs1] as i32).wrapping_add(self.regs[rs2] as i32);
                        self.regs[rd] = add_val as i64 as u64;
                    }
                    (0x0, 0x20) => {
                        // "subw"
                        let add_val = (self.regs[rs1] as i32).wrapping_sub(self.regs[rs2] as i32);
                        self.regs[rd] = add_val as i64 as u64;
                    }
                    (0x1, 0x0) => {
                        // "sllw"
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as u32) << shamt) as u64;
                    }
                    (0x5, 0x0) => {
                        // "srlw"
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as u32) >> shamt) as u64;
                    }
                    (0x5, 0x20) => {
                        // "sraw"
                        let shamt = (self.regs[rs2] as u64) & 0x1f;
                        self.regs[rd] = ((self.regs[rs1] as i32) >> shamt) as i64 as u64;
                    }
                    (0x0, 0x1) => {
                        // "mulw"
                        let mul = (self.regs[rs2] as u32) * (self.regs[rs2] as u32);
                        self.regs[rd] = mul as i32 as i64 as u64;
                    }
                    (0x4, 0x1) => {
                        // "divw"
                        let rem = (self.regs[rs2] as u32) / (self.regs[rs2] as u32);
                        self.regs[rd] = rem as u64;
                    }
                    (0x5, 0x1) => {
                        // "divuw"
                        let rem = (self.regs[rs2] as i32) / (self.regs[rs2] as i32);
                        self.regs[rd] = rem as i64 as u64;
                    }
                    (0x6, 0x1) => {
                        // "remw"
                        let rem = (self.regs[rs2] as i32) % (self.regs[rs2] as i32);
                        self.regs[rd] = rem as i64 as u64;
                    }
                    (0x7, 0x1) => {
                        // "remuw"
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
                        // "ecall"
                        Exception::EnvironmentalCallFromMMode.take_trap(self);
                    }
                    (0x0, 0x0, 0x1) => {
                        // "ebreak"
                    }
                    (0x0, 0x8, 0x2) => {
                        // "sret"
                        self.return_from_trap();
                    }
                    (0x0, 0x8, 0x5) => {
                        // "wfi"
                        self.wait_for_interrupt();
                    }
                    (0x0, 0x18, 0x2) => {
                        // "mret"
                        self.return_from_trap();
                    }
                    (0x1, _, _) => {
                        // "csrrw"
                        if rd != 0 {
                            self.regs[rd] = self.csr.load_csrs(csr) as u64;
                        }
                        self.csr.store_csrs(csr, self.regs[rs1]);
                    }
                    (0x2, _, _) => {
                        // "csrrs"
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, self.regs[rs1] | old_val);
                        }
                    }
                    (0x3, _, _) => {
                        // "csrrc"
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, self.regs[rs1] & !old_val);
                        }
                    }
                    (0x5, _, _) => {
                        // "csrrwi"
                        if rd != 0 {
                            self.regs[rd] = self.csr.load_csrs(csr);
                        }
                        self.csr.store_csrs(csr, uimm as u64);
                    }
                    (0x6, _, _) => {
                        // "csrrsi"
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, uimm as u64 | old_val);
                        }
                    }
                    (0x7, _, _) => {
                        // "csrrci"
                        let old_val = self.csr.load_csrs(csr) as u64;
                        self.regs[rd] = old_val;
                        if rs1 != 0 {
                            self.csr.store_csrs(csr, uimm as u64 & !old_val);
                        }
                    }
                    (0x0, 0x9, _) => {
                        // "sfence.
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
                        // "amoswap.
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
                        // "amoadd.
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
                        // "amoxor.
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
                        // "amoand.
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
                        // "amoor.
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
                        // "amomin.
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
                        // "amomax.
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
                        // "amominu.
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
                        // "amomaxu.
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
                        // "amoswap.
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
                        // "amoadd.
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
                        // "amoxor.
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
                        // "amoand.
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
                        // "amoor.
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
                        // "amomin.
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
                        // "amomax.
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
                        // "amominu.
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
                        // "amomaxu.
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
                        return Err(Exception::IllegalInstruction(inst));
                    }
                }
                Ok(())
            }
            _ => {
                error!("not implemented yet!");
                error!("pc=0x{:x}", self.pc);
                error!("inst:{inst:b}");
                return Err(Exception::IllegalInstruction(inst));
            }
        }
    }

    pub fn decode(&self, inst: u32) -> Result<DecodedInstruction, Exception> {
        // Decode the instruction and return a DecodedInstruction struct
        // This is a placeholder implementation
        // In a real implementation, this would parse the instruction bits
        // and return the appropriate DecodedInstruction variant
        Ok(DecodedInstruction::from(inst))
    }
}