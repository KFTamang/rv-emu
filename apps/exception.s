	.file	"exception.c"
	.option nopic
	.attribute arch, "rv64i2p0_m2p0_a2p0_f2p0_d2p0"
	.attribute unaligned_access, 0
	.attribute stack_align, 16
	.text
	.align	2
	.globl	_start
	.type	_start, @function
_start:
  addi t0, zero, 1
  la t1, handler
  csrrw zero, mstatus, t0
  csrrs zero, mtvec, t1
  csrrw zero, mepc, t2
  csrrc t2, mepc, zero
  csrrwi zero, sstatus, 4
  csrrsi zero, stvec, 5
  csrrwi zero, sepc, 6
  csrrci zero, sepc, 0
  ecall
  ret
handler:
  csrrs t2, mepc, zero
  addi t2, t2, 4
  csrrw zero, mepc, t2
  mret