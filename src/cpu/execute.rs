use super::*;

impl Cpu {
    pub fn execute(&mut self, bus: &mut Bus, inst: &DecodedInstr) -> Result<(), Exception> {
        self.clear_reg_marks();
        match *inst {
            DecodedInstr::Add {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                self.regs[rd] = self.regs[rs1].wrapping_add(self.regs[rs2]);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sub {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                self.regs[rd] = self.regs[rs1].wrapping_sub(self.regs[rs2]);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sll {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let shamt = self.regs[rs2] & 0x1f;
                self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Slt {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                self.regs[rd] = if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) {
                    1
                } else {
                    0
                };
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sltu {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                self.regs[rd] = if self.regs[rs1] < self.regs[rs2] {
                    1
                } else {
                    0
                };
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Xor {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                self.regs[rd] = self.regs[rs1] ^ self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Srl {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let shamt = self.regs[rs2] & 0x1f;
                self.regs[rd] = self.regs[rs1] as u64 >> shamt;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sra {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let shamt = self.regs[rs2] & 0x1f;
                self.regs[rd] = (self.regs[rs1] as i64 as u64) >> shamt;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Or {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                self.regs[rd] = self.regs[rs1] | self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::And {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                self.regs[rd] = self.regs[rs1] & self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mul {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                self.regs[rd] = self.regs[rs1].wrapping_mul(self.regs[rs2]);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mulh {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let mul =
                    (self.regs[rs1] as i64 as i128).wrapping_mul(self.regs[rs2] as i64 as i128);
                self.regs[rd] = (mul >> 64) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mulhsu {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let mul =
                    (self.regs[rs1] as i64 as i128).wrapping_mul(self.regs[rs2] as u128 as i128);
                self.regs[rd] = (mul >> 64) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mulhu {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let mul = (self.regs[rs1] as u128).wrapping_mul(self.regs[rs2] as u128);
                self.regs[rd] = (mul >> 64) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Div {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                self.regs[rd] = self.regs[rs1] / self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Divu {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                self.regs[rd] = ((self.regs[rs1] as i64) / (self.regs[rs2] as i64)) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Rem {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                self.regs[rd] = self.regs[rs1] % self.regs[rs2];
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Remu {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                self.regs[rd] = ((self.regs[rs1] as i64) % (self.regs[rs2] as i64)) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Addi {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                self.regs[rd] = self.regs[rs1].wrapping_add(imm);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Slti {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let result = if (self.regs[rs1] as i32 as i64) < (imm as i64) {
                    1
                } else {
                    0
                };
                self.regs[rd] = result;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Sltiu {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let result = if (self.regs[rs1] as i32 as i64 as u64) < imm {
                    1
                } else {
                    0
                };
                self.regs[rd] = result;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Xori {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let val = ((self.regs[rs1] as i32) ^ (imm as i32)) as u64;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Ori {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let val = ((self.regs[rs1] as i32) | (imm as i32)) as u64;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Andi {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let val = ((self.regs[rs1] as i32) & (imm as i32)) as u64;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Slli {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let shamt = (imm & 0x3f) as u64;
                self.regs[rd] = (self.regs[rs1] as u64) << shamt;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Srli {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let shamt = (imm & 0x3f) as u64;
                let logical_shift = imm >> 5;
                if logical_shift == 0 {
                    self.regs[rd] = (self.regs[rs1] as u64) >> shamt;
                } else {
                    self.regs[rd] = ((self.regs[rs1] as i64) >> shamt) as u64;
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lb {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(bus, addr, 8)?;
                self.regs[rd] = val as i8 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lh {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(bus, addr, 16)?;
                self.regs[rd] = val as i16 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lw {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(bus, addr, 32)?;
                self.regs[rd] = val as i32 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Ld {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(bus, addr, 64)?;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lbu {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(bus, addr, 8)?;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lhu {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(bus, addr, 16)?;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Lwu {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let addr = self.regs[rs1].wrapping_add(imm as u64);
                let val = self.load(bus, addr, 32)?;
                self.regs[rd] = val;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Sb {
                raw: _,
                rs1,
                rs2,
                imm,
            } => {
                let addr = self.regs[rs1].wrapping_add(imm as i32 as i64 as u64);
                self.store(bus, addr, 8, self.regs[rs2])?;
                self.mark_as_src1(rs1);
                self.mark_as_dest(rs2);
                Ok(())
            }
            DecodedInstr::Sh {
                raw: _,
                rs1,
                rs2,
                imm,
            } => {
                let addr = self.regs[rs1].wrapping_add(imm as i32 as i64 as u64);
                self.store(bus, addr, 16, self.regs[rs2])?;
                self.mark_as_src1(rs1);
                self.mark_as_dest(rs2);
                Ok(())
            }
            DecodedInstr::Sw {
                raw: _,
                rs1,
                rs2,
                imm,
            } => {
                let addr = self.regs[rs1].wrapping_add(imm as i32 as i64 as u64);
                self.store(bus, addr, 32, self.regs[rs2])?;
                self.mark_as_src1(rs1);
                self.mark_as_dest(rs2);
                Ok(())
            }
            DecodedInstr::Sd {
                raw: _,
                rs1,
                rs2,
                imm,
            } => {
                let addr = self.regs[rs1].wrapping_add(imm as i32 as i64 as u64);
                self.store(bus, addr, 64, self.regs[rs2])?;
                self.mark_as_src1(rs1);
                self.mark_as_dest(rs2);
                Ok(())
            }
            DecodedInstr::Jal { raw: _, rd, imm } => {
                self.regs[rd] = self.pc.wrapping_add(4);
                self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                self.mark_as_dest(rd);
                Ok(())
            }
            DecodedInstr::Jalr {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let return_addr = self.pc.wrapping_add(4);
                let next_pc = self.regs[rs1].wrapping_add(imm as u64).wrapping_sub(4);
                self.regs[rd] = return_addr;
                self.pc = next_pc;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Addiw {
                raw: _,
                rd,
                rs1,
                imm,
            } => {
                let src = self.regs[rs1] as i32;
                let val = src.wrapping_add(imm as i32);
                self.regs[rd] = val as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Slliw {
                raw: _,
                rd,
                rs1,
                shamt,
            } => {
                let src = self.regs[rs1] as u32;
                let val = src << shamt;
                self.regs[rd] = val as i32 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Srliw {
                raw: _,
                rd,
                rs1,
                shamt,
            } => {
                let src = self.regs[rs1] as u32;
                let val = src >> shamt;
                self.regs[rd] = val as i32 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Sraiw {
                raw: _,
                rd,
                rs1,
                shamt,
            } => {
                let src = self.regs[rs1] as i32;
                let val = src >> shamt;
                self.regs[rd] = val as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Beq {
                raw: _,
                rs1,
                rs2,
                imm,
            } => {
                if self.regs[rs1] == self.regs[rs2] {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Bne {
                raw: _,
                rs1,
                rs2,
                imm,
            } => {
                if self.regs[rs1] != self.regs[rs2] {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Blt {
                raw: _,
                rs1,
                rs2,
                imm,
            } => {
                if (self.regs[rs1] as i64) < (self.regs[rs2] as i64) {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Bge {
                raw: _,
                rs1,
                rs2,
                imm,
            } => {
                if (self.regs[rs1] as i64) >= (self.regs[rs2] as i64) {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Bltu {
                raw: _,
                rs1,
                rs2,
                imm,
            } => {
                if self.regs[rs1] < self.regs[rs2] {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Bgeu {
                raw: _,
                rs1,
                rs2,
                imm,
            } => {
                if self.regs[rs1] >= self.regs[rs2] {
                    self.pc = self.pc.wrapping_add(imm as u64).wrapping_sub(4);
                }
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Addw {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let add_val = (self.regs[rs1] as i32).wrapping_add(self.regs[rs2] as i32);
                self.regs[rd] = add_val as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Subw {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let add_val = (self.regs[rs1] as i32).wrapping_sub(self.regs[rs2] as i32);
                self.regs[rd] = add_val as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sllw {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let shamt = (self.regs[rs2] as u64) & 0x1f;
                self.regs[rd] = ((self.regs[rs1] as u32) << shamt) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Srlw {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let shamt = (self.regs[rs2] as u64) & 0x1f;
                self.regs[rd] = ((self.regs[rs1] as u32) >> shamt) as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Sraw {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let shamt = (self.regs[rs2] as u64) & 0x1f;
                self.regs[rd] = ((self.regs[rs1] as i32) >> shamt) as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Mulw {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let mul = (self.regs[rs2] as u32) * (self.regs[rs2] as u32);
                self.regs[rd] = mul as i32 as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Divw {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let rem = (self.regs[rs2] as u32) / (self.regs[rs2] as u32);
                self.regs[rd] = rem as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Divuw {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let rem = (self.regs[rs2] as i32) / (self.regs[rs2] as i32);
                self.regs[rd] = rem as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Remw {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let rem = (self.regs[rs2] as i32) % (self.regs[rs2] as i32);
                self.regs[rd] = rem as i64 as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Remuw {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let rem = (self.regs[rs2] as u32) % (self.regs[rs2] as u32);
                self.regs[rd] = rem as u64;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Lui { raw: _, rd, imm } => {
                self.regs[rd] = imm as u64;
                self.mark_as_dest(rd);
                Ok(())
            }
            DecodedInstr::Auipc { raw: _, rd, imm } => {
                self.regs[rd] = self.pc + imm;
                self.mark_as_dest(rd);
                Ok(())
            }
            DecodedInstr::Ecall { raw: _ } => {
                info!("ecall instruction from mode {}", self.mode);
                match self.mode {
                    M_MODE => Exception::EnvironmentalCallFromMMode.take_trap(self),
                    S_MODE => Exception::EnvironmentalCallFromSMode.take_trap(self),
                    U_MODE => Exception::EnvironmentalCallFromUMode.take_trap(self),
                    _ => panic!("ecall is executed with mode: {}", self.mode),
                }
                Ok(())
            }
            DecodedInstr::Ebreak { raw: _ } => Ok(()),
            DecodedInstr::Sret { raw } => {
                if self.mode < S_MODE {
                    return Err(Exception::IllegalInstruction(raw));
                }
                self.return_from_supervisor_trap();
                Ok(())
            }
            DecodedInstr::Mret { raw } => {
                if self.mode < M_MODE {
                    return Err(Exception::IllegalInstruction(raw));
                }
                self.return_from_machine_trap();
                Ok(())
            }
            DecodedInstr::Wfi { raw: _ } => Ok(()),
            DecodedInstr::Csrrw {
                raw: _,
                rd,
                rs1,
                csr,
            } => {
                if rd != 0 {
                    self.regs[rd] = self.csr.load_csrs(csr, self.cycle, &self.interrupt_list);
                }
                self.csr.store_csrs(csr, self.regs[rs1]);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Csrrs {
                raw: _,
                rd,
                rs1,
                csr,
            } => {
                let old = self.csr.load_csrs(csr, self.cycle, &self.interrupt_list);
                self.regs[rd] = old;
                if rs1 != 0 {
                    self.csr.store_csrs(csr, self.regs[rs1] | old);
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Csrrc {
                raw: _,
                rd,
                rs1,
                csr,
            } => {
                let old = self.csr.load_csrs(csr, self.cycle, &self.interrupt_list);
                self.regs[rd] = old;
                if rs1 != 0 {
                    self.csr.store_csrs(csr, self.regs[rs1] & !old);
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Csrrwi {
                raw: _,
                rd,
                rs1,
                csr,
                uimm,
            } => {
                if rd != 0 {
                    self.regs[rd] = self.csr.load_csrs(csr, self.cycle, &self.interrupt_list);
                }
                self.csr.store_csrs(csr, uimm as u64);
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Csrrsi {
                raw: _,
                rd,
                rs1,
                csr,
                uimm,
            } => {
                let old_val = self.csr.load_csrs(csr, self.cycle, &self.interrupt_list);
                self.regs[rd] = old_val;
                if rs1 != 0 {
                    self.csr.store_csrs(csr, uimm as u64 | old_val);
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Csrrci {
                raw: _,
                rd,
                rs1,
                csr,
                uimm,
            } => {
                let old_val = self.csr.load_csrs(csr, self.cycle, &self.interrupt_list);
                self.regs[rd] = old_val;
                if rs1 != 0 {
                    self.csr.store_csrs(csr, uimm as u64 & !old_val);
                }
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                Ok(())
            }
            DecodedInstr::Sfence { raw: _ } => {
                self.address_translation_cache.clear();
                self.block_cache.clear();
                Ok(())
            }
            DecodedInstr::Fence { raw: _ } => {
                self.address_translation_cache.clear();
                self.block_cache.clear();
                Ok(())
            }
            DecodedInstr::Amoswap {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let addr = self.regs[rs1];
                let val = self.load(bus, addr, 32)?;
                let src = self.regs[rs2];
                self.regs[rd] = val;
                self.store(bus, addr, 32, src)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Amoadd {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let addr = self.regs[rs1];
                let val = self.load(bus, addr, 32)?;
                let result = val.wrapping_add(self.regs[rs2]);
                self.regs[rd] = val;
                self.store(bus, addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Amoxor {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let addr = self.regs[rs1];
                let val = self.load(bus, addr, 32)?;
                let result = val ^ self.regs[rs2];
                self.regs[rd] = val;
                self.store(bus, addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Amoand {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let addr = self.regs[rs1];
                let val = self.load(bus, addr, 32)?;
                let result = val & self.regs[rs2];
                self.regs[rd] = val;
                self.store(bus, addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Amoor {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let addr = self.regs[rs1];
                let val = self.load(bus, addr, 32)?;
                let result = val | self.regs[rs2];
                self.regs[rd] = val;
                self.store(bus, addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Amomin {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let addr = self.regs[rs1];
                let loaded_value = self.load(bus, addr, 32)? as i32 as i64 as u64;
                let src_value = self.regs[rs2];
                self.regs[rd] = loaded_value;
                let result = cmp::min(loaded_value as i64, src_value as i64) as u64;
                self.store(bus, addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Amomax {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let addr = self.regs[rs1];
                let loaded_value = self.load(bus, addr, 32)? as i32 as i64 as u64;
                let src_value = self.regs[rs2];
                self.regs[rd] = loaded_value;
                let result = cmp::max(loaded_value as i64, src_value as i64) as u64;
                self.store(bus, addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Amominu {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let addr = self.regs[rs1];
                let loaded_value = self.load(bus, addr, 32)? as i32 as i64 as u64;
                let src_value = self.regs[rs2];
                self.regs[rd] = loaded_value;
                let result = cmp::min(loaded_value, src_value);
                self.store(bus, addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::Amomaxu {
                raw: _,
                rd,
                rs1,
                rs2,
            } => {
                let addr = self.regs[rs1];
                let loaded_value = self.load(bus, addr, 32)? as i32 as i64 as u64;
                let src_value = self.regs[rs2];
                self.regs[rd] = loaded_value;
                let result = cmp::max(loaded_value, src_value);
                self.store(bus, addr, 32, result)?;
                self.mark_as_dest(rd);
                self.mark_as_src1(rs1);
                self.mark_as_src2(rs2);
                Ok(())
            }
            DecodedInstr::IllegalInstruction { inst } => Err(Exception::IllegalInstruction(inst)),
        }
    }
}
