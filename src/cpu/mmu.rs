use super::*;

impl Cpu {
    /// SV39 page-table walk + permission check + A/D handling + simple TLB keyed by (satp_ppn, asid, va_page).
    pub(crate) fn translate(&mut self, bus: &mut Bus, va: u64, acc: AccessMode) -> Result<u64, Exception> {
        const PAGESIZE: u64 = 4096;
        const PTESIZE: u64 = 8;

        if matches!(acc, AccessMode::Fetch) {
            if self.mode == M_MODE {
                return Ok(va);
            }
        }
        if matches!(acc, AccessMode::Load | AccessMode::Store) {
            if self.mode == M_MODE && self.csr.get_mstatus_bit(MASK_MPRV, BIT_MPRV) == 0 {
                return Ok(va);
            }
        }

        let satp = self.csr.load_csrs(SATP, self.cycle, &self.interrupt_list);
        let mode = (satp >> 60) & 0xF;
        let asid = (satp >> 44) & 0xFFFF;
        let satp_ppn = satp & ((1u64 << 44) - 1);

        if mode == 0 {
            return Ok(va);
        }
        if mode != 8 {
            return match acc {
                AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
                AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
            };
        }

        let sign = (va >> 38) & 1;
        let upper = va >> 39;
        if (sign == 0 && upper != 0) || (sign == 1 && upper != ((1u64 << 25) - 1)) {
            return match acc {
                AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
                AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
            };
        }

        let va_page = va >> 12;
        let tlb_key = (satp_ppn, asid, va_page);
        if let Some(&pa_page) = self.address_translation_cache.get(&tlb_key) {
            return Ok((pa_page << 12) | (va & 0xFFF));
        }

        let vpn0 = (va >> 12) & 0x1FF;
        let vpn1 = (va >> 21) & 0x1FF;
        let vpn2 = (va >> 30) & 0x1FF;
        let vpn = [vpn0, vpn1, vpn2];

        let mut a = satp_ppn * PAGESIZE;
        let mut level: i32 = 2;

        let mut pte_addr: u64 = 0;
        let mut pte: u64 = 0;

        loop {
            pte_addr = a + vpn[level as usize] * PTESIZE;
            pte = bus
                .load(pte_addr, 64)
                .map_err(|_| match acc {
                    AccessMode::Fetch => Exception::InstructionPageFault(va as u32),
                    AccessMode::Load => Exception::LoadPageFault(va as u32),
                    AccessMode::Store => Exception::StoreAMOPageFault(va as u32),
                })?;

            let v = bit(pte, 0);
            let r = bit(pte, 1);
            let w = bit(pte, 2);
            let x = bit(pte, 3);
            let u = bit(pte, 4);
            let a_bit = bit(pte, 6);
            let d_bit = bit(pte, 7);

            if v == 0 || (r == 0 && w == 1) {
                return match acc {
                    AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
                    AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                    AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
                };
            }

            let is_leaf = (r == 1) || (x == 1);
            if is_leaf {
                match acc {
                    AccessMode::Fetch => {
                        if x == 0 {
                            return Err(Exception::InstructionPageFault(va as u32));
                        }
                    }
                    AccessMode::Load => {
                        if r == 0 {
                            return Err(Exception::LoadPageFault(va as u32));
                        }
                    }
                    AccessMode::Store => {
                        if w == 0 {
                            return Err(Exception::StoreAMOPageFault(va as u32));
                        }
                    }
                }

                if self.mode == U_MODE && u == 0 {
                    return match acc {
                        AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
                        AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                        AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
                    };
                }

                let mut new_pte = pte;
                if a_bit == 0 {
                    new_pte |= 1 << 6;
                }
                if matches!(acc, AccessMode::Store) && d_bit == 0 {
                    new_pte |= 1 << 7;
                }
                if new_pte != pte {
                    bus.store(pte_addr, 64, new_pte)
                        .map_err(|_| match acc {
                            AccessMode::Fetch => Exception::InstructionPageFault(va as u32),
                            AccessMode::Load => Exception::LoadPageFault(va as u32),
                            AccessMode::Store => Exception::StoreAMOPageFault(va as u32),
                        })?;
                    pte = new_pte;
                }

                let ppn0 = (pte >> 10) & 0x1FF;
                let ppn1 = (pte >> 19) & 0x1FF;
                let ppn2 = (pte >> 28) & 0x3FF_FFFF;
                if level == 2 {
                    if ppn0 != 0 || ppn1 != 0 {
                        return match acc {
                            AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
                            AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                            AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
                        };
                    }
                } else if level == 1 {
                    if ppn0 != 0 {
                        return match acc {
                            AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
                            AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                            AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
                        };
                    }
                }

                let page_off = va & 0xFFF;
                let pa: u64 = match level {
                    0 => {
                        let ppn = (pte >> 10) & ((1u64 << 44) - 1);
                        (ppn << 12) | page_off
                    }
                    1 => {
                        let ppn = ((ppn2 << 18) | (ppn1 << 9) | vpn0) & ((1u64 << 44) - 1);
                        (ppn << 12) | page_off
                    }
                    2 => {
                        let ppn = ((ppn2 << 18) | (vpn1 << 9) | vpn0) & ((1u64 << 44) - 1);
                        (ppn << 12) | page_off
                    }
                    _ => unreachable!(),
                };

                self.address_translation_cache.insert(tlb_key, pa >> 12);
                return Ok(pa);
            }

            if level == 0 {
                return match acc {
                    AccessMode::Fetch => Err(Exception::InstructionPageFault(va as u32)),
                    AccessMode::Load => Err(Exception::LoadPageFault(va as u32)),
                    AccessMode::Store => Err(Exception::StoreAMOPageFault(va as u32)),
                };
            }

            let next_ppn = (pte >> 10) & ((1u64 << 44) - 1);
            a = next_ppn * PAGESIZE;
            level -= 1;
        }
    }
}
