use log::{error, info};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DecodedInstr {
    Add { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Sub { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Sll { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Slt { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Sltu { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Xor { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Srl { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Sra { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Or { raw: u32, rd: usize, rs1: usize, rs2: usize},
    And { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Mul { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Mulh { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Mulhsu { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Mulhu { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Div { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Divu { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Rem { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Remu { raw: u32, rd: usize, rs1: usize, rs2: usize},
    Addi { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Slti { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Sltiu { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Xori { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Ori { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Andi { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Slli { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Srli { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Lb { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Lh { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Lw { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Ld { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Lbu { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Lhu { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Lwu { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Sb { raw: u32, rs1: usize, rs2: usize, imm: u64 },
    Sh { raw: u32, rs1: usize, rs2: usize, imm: u64 },
    Sw { raw: u32, rs1: usize, rs2: usize, imm: u64 },
    Sd { raw: u32, rs1: usize, rs2: usize, imm: u64 },
    Jal { raw: u32, rd: usize, imm: u64 },
    Jalr { raw: u32, rd: usize, rs1: usize, imm: u64 },
    Addiw { raw: u32, rd: usize, rs1: usize, imm: i32 },
    Slliw { raw: u32, rd: usize, rs1: usize, shamt: u32 },
    Srliw { raw: u32, rd: usize, rs1: usize, shamt: u32 },
    Sraiw { raw: u32, rd: usize, rs1: usize, shamt: u32 },
    Beq { raw: u32, rs1: usize, rs2: usize, imm: u64 },
    Bne { raw: u32, rs1: usize, rs2: usize, imm: u64 },
    Blt { raw: u32, rs1: usize, rs2: usize, imm: u64 },
    Bge { raw: u32, rs1: usize, rs2: usize, imm: u64 },
    Bltu { raw: u32, rs1: usize, rs2: usize, imm: u64 },
    Bgeu { raw: u32, rs1: usize, rs2: usize, imm: u64 },
    Addw { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Subw { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Sllw { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Srlw { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Sraw { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Mulw { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Divw { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Divuw { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Remw { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Remuw { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Lui { raw: u32, rd: usize, imm: u64 },
    Auipc { raw: u32, rd: usize, imm: u64 },
    Ecall { raw: u32 },
    Ebreak { raw: u32 },
    Sret { raw: u32 },
    Wfi { raw: u32 },
    Mret { raw: u32 },
    Csrrw { raw: u32, rd: usize, rs1: usize, csr: usize },
    Csrrs { raw: u32, rd: usize, rs1: usize, csr: usize },
    Csrrc { raw: u32, rd: usize, rs1: usize, csr: usize },
    Csrrwi { raw: u32, rd: usize, rs1: usize, csr: usize, uimm: u32 },
    Csrrsi { raw: u32, rd: usize, rs1: usize, csr: usize, uimm: u32 },
    Csrrci { raw: u32, rd: usize, rs1: usize, csr: usize, uimm: u32 },
    Sfence { raw: u32 },
    Fence { raw: u32 },
    Amoswap { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Amoadd { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Amoxor { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Amoand { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Amoor { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Amomin { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Amomax { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Amominu { raw: u32, rd: usize, rs1: usize, rs2: usize },
    Amomaxu { raw: u32, rd: usize, rs1: usize, rs2: usize },
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
                        DecodedInstr::Add {raw: inst, rd, rs1, rs2 }
                    }
                    (0x0, 0x20) => {
                        DecodedInstr::Sub {raw: inst, rd, rs1, rs2 }
                    }
                    (0x1, 0x0) => {
                        DecodedInstr::Sll {raw: inst, rd, rs1, rs2 }
                    }
                    (0x2, 0x0) => {
                        DecodedInstr::Slt {raw: inst, rd, rs1, rs2 }
                    }
                    (0x3, 0x0) => {
                        DecodedInstr::Sltu {raw: inst, rd, rs1, rs2 }
                    }
                    (0x4, 0x0) => {
                        DecodedInstr::Xor {raw: inst, rd, rs1, rs2 }
                    }
                    (0x5, 0x0) => {
                        DecodedInstr::Srl {raw: inst, rd, rs1, rs2 }
                    }
                    (0x5, 0x20) => {
                        DecodedInstr::Sra {raw: inst, rd, rs1, rs2 }
                    }
                    (0x6, 0x0) => {
                        DecodedInstr::Or {raw: inst, rd, rs1, rs2 }
                    }
                    (0x7, 0x0) => {
                        DecodedInstr::And {raw: inst, rd, rs1, rs2 }
                    }
                    (0x0, 0x1) => {
                        DecodedInstr::Mul {raw: inst, rd, rs1, rs2 }
                    }
                    (0x1, 0x1) => {
                        DecodedInstr::Mulh {raw: inst, rd, rs1, rs2 }
                    }
                    (0x2, 0x1) => {
                        DecodedInstr::Mulhsu {raw: inst, rd, rs1, rs2 }
                    }
                    (0x3, 0x1) => {
                        DecodedInstr::Mulhu {raw: inst, rd, rs1, rs2 }
                    }
                    (0x4, 0x1) => {
                        DecodedInstr::Div {raw: inst, rd, rs1, rs2 }
                    }
                    (0x5, 0x1) => {
                        DecodedInstr::Divu {raw: inst, rd, rs1, rs2 }
                    }
                    (0x6, 0x1) => {
                        DecodedInstr::Rem {raw: inst, rd, rs1, rs2 }
                    }
                    (0x7, 0x1) => {
                        DecodedInstr::Remu {raw: inst, rd, rs1, rs2 }
                    }
                    (_, _) => {
                        error!("This should not be reached!");
                        info!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        DecodedInstr::IllegalInstruction{ inst }
                    }
                }
            },
            0x13 => {
                let imm = (inst as i32 >> 20) as i64 as u64;
                match funct3 {
                    0x0 => {
                        DecodedInstr::Addi{raw: inst, rd, rs1, imm }
                    }
                    0x2 => {
                        DecodedInstr::Slti{raw: inst, rd, rs1, imm }
                    }
                    0x3 => {
                        DecodedInstr::Sltiu{raw: inst, rd, rs1, imm }
                    }
                    0x4 => {
                        DecodedInstr::Xori{raw: inst, rd, rs1, imm }
                    }
                    0x6 => {
                        DecodedInstr::Ori{raw: inst, rd, rs1, imm }
                    }
                    0x7 => {
                        DecodedInstr::Andi{raw: inst, rd, rs1, imm }
                    }
                    0x1 => {
                        DecodedInstr::Slli{raw: inst, rd, rs1, imm }
                    }
                    0x5 => {
                        DecodedInstr::Srli{raw: inst, rd, rs1, imm }
                    }
                    _ => {
                        error!("This should not be reached!");
                        error!("funct3 = {:>#x}, funct7 = {:>#x}", funct3, funct7);
                        DecodedInstr::IllegalInstruction{ inst }
                    }
                }
            }
            0x03 => {
                let imm = ((inst as i32 as i64) >> 20) as u64;
                match funct3 {
                    0x0 => {
                        DecodedInstr::Lb {raw: inst, rd, rs1, imm}
                    }
                    0x1 => {
                        DecodedInstr::Lh {raw: inst, rd, rs1, imm }
                    }
                    0x2 => {
                        DecodedInstr::Lw {raw: inst, rd, rs1, imm }
                    }
                    0x3 => {
                        DecodedInstr::Ld {raw: inst, rd, rs1, imm }
                    }
                    0x4 => {
                        DecodedInstr::Lbu {raw: inst, rd, rs1, imm }
                    }
                    0x5 => {
                        DecodedInstr::Lhu {raw: inst, rd, rs1, imm }
                    }
                    0x6 => {
                        DecodedInstr::Lwu {raw: inst, rd, rs1, imm }
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
                let imm = (((inst & 0xfe000000) as i32 as i64 >> 20) as u64)
                    | ((inst >> 7) & 0x1f) as u64;
                match funct3 {
                    0x0 => {
                        DecodedInstr::Sb {raw: inst, rs1, rs2, imm }
                    }
                    0x1 => {
                        DecodedInstr::Sh {raw: inst, rs1, rs2, imm }
                    }
                    0x2 => {
                        DecodedInstr::Sw {raw: inst, rs1, rs2, imm }
                    }
                    0x3 => {
                        DecodedInstr::Sd {raw: inst, rs1, rs2, imm }
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
                let imm = ((inst & 0x80000000) as i32 as i64 >> 11) as u64
                    | ((inst & 0x7fe00000) as u64) >> 20
                    | ((inst & 0x100000) as u64) >> 9
                    | ((inst & 0xff000) as u64);
                DecodedInstr::Jal { raw: inst, rd, imm }
            }
            0x67 => {
                match funct3 {
                    0x0 => {
                        let imm = ((inst as i32 as i64) >> 20) as u64;
                        DecodedInstr::Jalr {raw: inst, rd, rs1, imm }
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
                        DecodedInstr::Addiw {raw: inst, rd, rs1, imm }
                    }
                    (0x1, 0x0) => {
                        // slliw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        DecodedInstr::Slliw {raw: inst, rd, rs1, shamt }
                    }
                    (0x5, 0x0) => {
                        // srliw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        DecodedInstr::Srliw {raw: inst, rd, rs1, shamt }
                    }
                    (0x5, 0x20) => {
                        // sraiw
                        // I-type format
                        let shamt = ((inst as u32) >> 20) & 0x1f;
                        DecodedInstr::Sraiw {raw: inst, rd, rs1, shamt }
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
                let imm = ((inst & 0x80000000) as i32 as i64 >> 19) as u64
                    | ((inst & 0x7e000000) as u64) >> 20
                    | ((inst & 0xf00) as u64) >> 7
                    | ((inst & 0x80) as u64) << 4;
                match funct3 {
                    0x0 => {
                        DecodedInstr::Beq {raw: inst, rs1, rs2, imm }
                    }
                    0x1 => {
                        DecodedInstr::Bne {raw: inst, rs1, rs2, imm }
                    }
                    0x4 => {
                        DecodedInstr::Blt {raw: inst, rs1, rs2, imm }
                    }
                    0x5 => {
                        DecodedInstr::Bge {raw: inst, rs1, rs2, imm }
                    }
                    0x6 => {
                        DecodedInstr::Bltu {raw: inst, rs1, rs2, imm }
                    }
                    0x7 => {
                        DecodedInstr::Bgeu {raw: inst, rs1, rs2, imm }
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
                        DecodedInstr::Addw {raw: inst, rd, rs1, rs2 }
                    }
                    (0x0, 0x20) => {
                        DecodedInstr::Subw {raw: inst, rd, rs1, rs2 }
                    }
                    (0x1, 0x0) => {
                        DecodedInstr::Sllw {raw: inst, rd, rs1, rs2 }
                    }
                    (0x5, 0x0) => {
                        DecodedInstr::Srlw {raw: inst, rd, rs1, rs2 }
                    }
                    (0x5, 0x20) => {
                        DecodedInstr::Sraw {raw: inst, rd, rs1, rs2 }
                    }
                    (0x0, 0x1) => {
                        DecodedInstr::Mulw {raw: inst, rd, rs1, rs2 }
                    }
                    (0x4, 0x1) => {
                        DecodedInstr::Divw {raw: inst, rd, rs1, rs2 }
                    }
                    (0x5, 0x1) => {
                        DecodedInstr::Divuw {raw: inst, rd, rs1, rs2 }
                    }
                    (0x6, 0x1) => {
                        DecodedInstr::Remw {raw: inst, rd, rs1, rs2 }
                    }
                    (0x7, 0x1) => {
                        DecodedInstr::Remuw {raw: inst, rd, rs1, rs2 }
                    }
                    _ => {
                        error!("This should not be reached!");
                        DecodedInstr::IllegalInstruction{ inst }
                    }
                }
            }
            0x37 => {
                // "lui"
                let imm = (inst & 0xfffff000) as i32 as i64 as u64;
                DecodedInstr::Lui { raw: inst, rd, imm }
            }
            0x17 => {
                // "auipc"
                let imm = (inst & 0xfffff000) as i32 as i64 as u64;
                DecodedInstr::Auipc { raw: inst, rd, imm }
            }
            0x73 => {
                let csr = ((inst as u32) >> 20) as usize;
                let uimm = ((inst & 0xf8000) as u32) >> 15;
                match (funct3, funct7, rs2) {
                    (0x0, 0x0, 0x0) => {
                        DecodedInstr::Ecall{raw: inst}
                    }
                    (0x0, 0x0, 0x1) => {
                        DecodedInstr::Ebreak{raw: inst}
                    }
                    (0x0, 0x8, 0x2) => {
                        DecodedInstr::Sret{raw: inst}
                    }
                    (0x0, 0x8, 0x5) => {
                        DecodedInstr::Wfi{raw: inst}
                    }
                    (0x0, 0x18, 0x2) => {
                        DecodedInstr::Mret{raw: inst}
                    }
                    (0x1, _, _) => {
                        DecodedInstr::Csrrw {raw: inst,
                            rd, rs1, csr
                        }
                    }
                    (0x2, _, _) => {
                        DecodedInstr::Csrrs{raw: inst, 
                            rd, rs1, csr
                        }
                    }
                    (0x3, _, _) => {
                        DecodedInstr::Csrrc{raw: inst, 
                            rd, rs1, csr
                        }
                    }
                    (0x5, _, _) => {
                        DecodedInstr::Csrrwi{raw: inst, 
                            rd, rs1, csr, uimm
                        }
                    }
                    (0x6, _, _) => {
                        DecodedInstr::Csrrsi{raw: inst, 
                            rd, rs1, csr, uimm
                        }
                    }
                    (0x7, _, _) => {
                        DecodedInstr::Csrrci{raw: inst, 
                            rd, rs1, csr, uimm
                        }
                    }
                    (0x0, 0x9, _) => {
                        DecodedInstr::Sfence{raw: inst}
                    }
                    (_, _, _) => {
                        error!("Unsupported CSR instruction!");
                        error!("funct3:{}, funct7:{}", funct3, funct7);
                        DecodedInstr::IllegalInstruction{ inst }
                    }
                }
            }
            0x0f => {
                DecodedInstr::Fence{ raw: inst }
            }
            0x2f => {
                // Atomic Operation instructions
                let funct5 = funct7 >> 2;
                match (funct3, funct5) {
                    (0x2, 0x1) => {
                        DecodedInstr::Amoswap{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x0, 0x1) => {
                        DecodedInstr::Amoadd{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x4, 0x1) => {
                        DecodedInstr::Amoxor{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0xc, 0x1) => {
                        DecodedInstr::Amoand{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x8, 0x1) => {
                        DecodedInstr::Amoor{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x10, 0x1) => {
                        DecodedInstr::Amomin{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x14, 0x1) => {
                        DecodedInstr::Amomax{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x18, 0x1) => {
                        DecodedInstr::Amominu{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x1c, 0x1) => {
                        DecodedInstr::Amomaxu{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x2, 0x3) => {
                        DecodedInstr::Amoswap{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x0, 0x3) => {
                        DecodedInstr::Amoadd{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x4, 0x3) => {
                        DecodedInstr::Amoxor{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0xc, 0x3) => {
                        DecodedInstr::Amoand{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x8, 0x3) => {
                        DecodedInstr::Amoor{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x10, 0x3) => {
                        DecodedInstr::Amomin{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x14, 0x3) => {
                        DecodedInstr::Amomax{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x18, 0x3) => {
                        DecodedInstr::Amominu{raw: inst,
                            rd, rs1, rs2
                        }
                    }
                    (0x1c, 0x3) => {
                        DecodedInstr::Amomaxu{raw: inst,
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

    pub fn is_branch(&self) -> bool {
        matches!(self, DecodedInstr::Beq { .. }
            | DecodedInstr::Bne { .. }
            | DecodedInstr::Blt { .. }
            | DecodedInstr::Bge { .. }
            | DecodedInstr::Bltu { .. }
            | DecodedInstr::Bgeu { .. })
    }

    pub fn is_jump(&self) -> bool {
        matches!(self, DecodedInstr::Jal { .. }
            | DecodedInstr::Jalr { .. }
            | DecodedInstr::Ecall{..}
            | DecodedInstr::Ebreak{..}
            | DecodedInstr::Sret{..}
            | DecodedInstr::Wfi{..}
            | DecodedInstr::Mret{..}
        )
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BasicBlock {
    pub start_pc: u64,
    pub end_pc: u64,
    pub instrs: Vec<DecodedInstr>,
}