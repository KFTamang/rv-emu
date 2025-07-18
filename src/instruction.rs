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
    Addw { rd: usize, rs1: usize, rs2: usize },
    Subw { rd: usize, rs1: usize, rs2: usize },
    Sllw { rd: usize, rs1: usize, rs2: usize },
    Srlw { rd: usize, rs1: usize, rs2: usize },
    Sraw { rd: usize, rs1: usize, rs2: usize },
    Mulw { rd: usize, rs1: usize, rs2: usize },
    Divw { rd: usize, rs1: usize, rs2: usize },
    Divuw { rd: usize, rs1: usize, rs2: usize },
    Remw { rd: usize, rs1: usize, rs2: usize },
    Remuw { rd: usize, rs1: usize, rs2: usize },
    Lui { rd: usize, imm: u32 },
    Auipc { rd: usize, imm: u32 },
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
    Amoswap { rd: usize, rs1: usize, rs2: usize },
    Amoadd { rd: usize, rs1: usize, rs2: usize },
    Amoxor { rd: usize, rs1: usize, rs2: usize },
    Amoand { rd: usize, rs1: usize, rs2: usize },
    Amoor { rd: usize, rs1: usize, rs2: usize },
    Amomin { rd: usize, rs1: usize, rs2: usize },
    Amomax { rd: usize, rs1: usize, rs2: usize },
    Amominu { rd: usize, rs1: usize, rs2: usize },
    Amomaxu { rd: usize, rs1: usize, rs2: usize },
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
                        DecodedInstr::IllegalInstruction{ inst }
                    }
                }
            },
            0x13 => {
                let imm = ((inst as i32 as i64) >> 20) as u32;
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
                        let imm = ((inst as i32) >> 20) as u32;
                        DecodedInstr::Jalr { rd, rs1, imm }
                    }
                    _ => {
                        error!("This should not be reached!");
                        error!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        DecodedInstr::IllegalInstruction{ inst }
                    }
                }
            }
            0x1b => {
                match (funct3, funct7) {
                    (0x0, _) => {
                        // addiw
                        // I-type format
                        let imm = (inst as i32) >> 20;
                        DecodedInstr::Addiw { rd, rs1, imm: imm as u32 }
                    }
                    (0x1, 0x0) => {
                        // slliw
                        // I-type format
                        let imm = ((inst as u32) >> 20) & 0x1f;
                        DecodedInstr::Slliw { rd, rs1, imm}
                    }
                    (0x5, 0x0) => {
                        // srliw
                        // I-type format
                        let imm = ((inst as u32) >> 20) & 0x1f;
                        DecodedInstr::Srliw { rd, rs1, imm }
                    }
                    (0x5, 0x20) => {
                        // sraiw
                        // I-type format
                        let imm = ((inst as u32) >> 20) & 0x1f;
                        DecodedInstr::Sraiw { rd, rs1, imm }
                    }
                    _ => {
                        error!("This should not be reached!");
                        error!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        DecodedInstr::IllegalInstruction{ inst }
                    }
                }
            }
            0x63 => {
                // branch instructions
                let imm = ((inst & 0x80000000) as i32 >> 19) as u32
                    | ((inst & 0x7e000000) as u32) >> 20
                    | ((inst & 0xf00) as u32) >> 7
                    | ((inst & 0x80) as u32) << 4;
                match funct3 {
                    0x0 => {
                        DecodedInstr::Beq { rd, rs1, rs2, imm }
                    }
                    0x1 => {
                        DecodedInstr::Bne { rd, rs1, rs2, imm }
                    }
                    0x4 => {
                        DecodedInstr::Blt { rd, rs1, rs2, imm }
                    }
                    0x5 => {
                        DecodedInstr::Bge { rd, rs1, rs2, imm }
                    }
                    0x6 => {
                        DecodedInstr::Bltu { rd, rs1, rs2, imm }
                    }
                    0x7 => {
                        DecodedInstr::Bgeu { rd, rs1, rs2, imm }
                    }
                    _ => {
                        error!("This should not be reached!");
                        error!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        DecodedInstr::IllegalInstruction{ inst }
                    }
                }
            }
            0x3b => {
                match (funct3, funct7) {
                    (0x0, 0x0) => {
                        DecodedInstr::Addw { rd, rs1, rs2 }
                    }
                    (0x0, 0x20) => {
                        DecodedInstr::Subw { rd, rs1, rs2 }
                    }
                    (0x1, 0x0) => {
                        DecodedInstr::Sllw { rd, rs1, rs2 }
                    }
                    (0x5, 0x0) => {
                        DecodedInstr::Srlw { rd, rs1, rs2 }
                    }
                    (0x5, 0x20) => {
                        DecodedInstr::Sraw { rd, rs1, rs2 }
                    }
                    (0x0, 0x1) => {
                        DecodedInstr::Mulw { rd, rs1, rs2 }
                    }
                    (0x4, 0x1) => {
                        DecodedInstr::Divw { rd, rs1, rs2 }
                    }
                    (0x5, 0x1) => {
                        DecodedInstr::Divuw { rd, rs1, rs2 }
                    }
                    (0x6, 0x1) => {
                        DecodedInstr::Remw { rd, rs1, rs2 }
                    }
                    (0x7, 0x1) => {
                        DecodedInstr::Remuw { rd, rs1, rs2 }
                    }
                    _ => {
                        error!("This should not be reached!");
                        DecodedInstr::IllegalInstruction{ inst }
                    }
                }
            }
            0x37 => {
                let imm = (inst & 0xfffff000) as i32 as u32;
                // "lui"
                DecodedInstr::Lui { rd, imm }
            }
            0x17 => {
                let imm = inst & 0xfffff000;
                // "auipc"
                DecodedInstr::Auipc { rd, imm }
            }
            0x73 => {
                let csr = ((inst as u32) >> 20) as usize;
                let uimm = ((inst & 0xf8000) as u32) >> 15;
                let imm = (inst as i32 as i64 >> 20) as u64;
                match (funct3, funct7, rs2) {
                    (0x0, 0x0, 0x0) => {
                        DecodedInstr::Ecall
                    }
                    (0x0, 0x0, 0x1) => {
                        DecodedInstr::Ebreak
                    }
                    (0x0, 0x8, 0x2) => {
                        DecodedInstr::Sret
                    }
                    (0x0, 0x8, 0x5) => {
                        DecodedInstr::Wfi
                    }
                    (0x0, 0x18, 0x2) => {
                        DecodedInstr::Mret
                    }
                    (0x1, _, _) => {
                        DecodedInstr::Csrrw {
                            rd, rs1, imm: csr
                        }
                    }
                    (0x2, _, _) => {
                        DecodedInstr::Csrrs{ 
                            rd, rs1, imm: csr
                        }
                    }
                    (0x3, _, _) => {
                        DecodedInstr::Csrrc{ 
                            rd, rs1, imm: csr
                        }
                    }
                    (0x5, _, _) => {
                        DecodedInstr::Csrrwi{ 
                            rd, rs1, imm: csr
                        }
                    }
                    (0x6, _, _) => {
                        DecodedInstr::Csrrsi{ 
                            rd, rs1, imm: csr
                        }
                    }
                    (0x7, _, _) => {
                        DecodedInstr::Csrrci{ 
                            rd, rs1, imm: csr
                        }
                    }
                    (0x0, 0x9, _) => {
                        DecodedInstr::Sfence
                    }
                    (_, _, _) => {
                        error!("Unsupported CSR instruction!");
                        error!("funct3:{}, funct7:{}", funct3, funct7);
                        DecodedInstr::IllegalInstruction{ inst }
                    }
                }
            }
            0x0f => {
                DecodedInstr::Fence
            }
            0x2f => {
                // Atomic Operation instructions
                let funct5 = funct7 >> 2;
                match (funct3, funct5) {
                    (0x2, 0x1) => {
                        DecodedInstr::Amoswap{
                            rd, rs1, rs2
                        }
                    }
                    (0x0, 0x1) => {
                        DecodedInstr::Amoadd{
                            rd, rs1, rs2
                        }
                    }
                    (0x4, 0x1) => {
                        DecodedInstr::Amoxor{
                            rd, rs1, rs2
                        }
                    }
                    (0xc, 0x1) => {
                        DecodedInstr::Amoand{
                            rd, rs1, rs2
                        }
                    }
                    (0x8, 0x1) => {
                        DecodedInstr::Amoor{
                            rd, rs1, rs2
                        }
                    }
                    (0x10, 0x1) => {
                        DecodedInstr::Amomin{
                            rd, rs1, rs2
                        }
                    }
                    (0x14, 0x1) => {
                        DecodedInstr::Amomax{
                            rd, rs1, rs2
                        }
                    }
                    (0x18, 0x1) => {
                        DecodedInstr::Amominu{
                            rd, rs1, rs2
                        }
                    }
                    (0x1c, 0x1) => {
                        DecodedInstr::Amomaxu{
                            rd, rs1, rs2
                        }
                    }
                    (0x2, 0x3) => {
                        DecodedInstr::Amoswap{
                            rd, rs1, rs2
                        }
                    }
                    (0x0, 0x3) => {
                        DecodedInstr::Amoadd{
                            rd, rs1, rs2
                        }
                    }
                    (0x4, 0x3) => {
                        DecodedInstr::Amoxor{
                            rd, rs1, rs2
                        }
                    }
                    (0xc, 0x3) => {
                        DecodedInstr::Amoand{
                            rd, rs1, rs2
                        }
                    }
                    (0x8, 0x3) => {
                        DecodedInstr::Amoor{
                            rd, rs1, rs2
                        }
                    }
                    (0x10, 0x3) => {
                        DecodedInstr::Amomin{
                            rd, rs1, rs2
                        }
                    }
                    (0x14, 0x3) => {
                        DecodedInstr::Amomax{
                            rd, rs1, rs2
                        }
                    }
                    (0x18, 0x3) => {
                        DecodedInstr::Amominu{
                            rd, rs1, rs2
                        }
                    }
                    (0x1c, 0x3) => {
                        DecodedInstr::Amomaxu{
                            rd, rs1, rs2
                        }
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