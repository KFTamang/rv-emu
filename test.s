	.file	"test.c"
	.option nopic
	.attribute arch, "rv64i2p0_m2p0_a2p0_f2p0_d2p0"
	.attribute unaligned_access, 0
	.attribute stack_align, 16
	.text
	.align	2
	.globl	main
	.type	main, @function
main:
	addi	sp,sp,-16
	sd	ra,8(sp)
	sd	s0,0(sp)
	addi	s0,sp,16
	li	a0,100
	call	fizzbuzz
	li	a5,0
	mv	a0,a5
	ld	ra,8(sp)
	ld	s0,0(sp)
	addi	sp,sp,16
	jr	ra
	.size	main, .-main
	.align	2
	.globl	fizzbuzz
	.type	fizzbuzz, @function
fizzbuzz:
	addi	sp,sp,-544
	sd	s0,536(sp)
	addi	s0,sp,544
	mv	a5,a0
	sw	a5,-532(s0)
	sw	zero,-20(s0)
	j	.L4
.L8:
	lw	a5,-20(s0)
	mv	a4,a5
	li	a5,3
	remw	a5,a4,a5
	sext.w	a5,a5
	bne	a5,zero,.L5
	lw	a5,-20(s0)
	mv	a4,a5
	li	a5,5
	remw	a5,a4,a5
	sext.w	a5,a5
	bne	a5,zero,.L5
	lw	a5,-20(s0)
	addi	a5,a5,-16
	add	a5,a5,s0
	li	a4,42
	sb	a4,-504(a5)
	j	.L4
.L5:
	lw	a5,-20(s0)
	mv	a4,a5
	li	a5,3
	remw	a5,a4,a5
	sext.w	a5,a5
	bne	a5,zero,.L6
	lw	a5,-20(s0)
	addi	a5,a5,-16
	add	a5,a5,s0
	li	a4,70
	sb	a4,-504(a5)
	j	.L4
.L6:
	lw	a5,-20(s0)
	mv	a4,a5
	li	a5,3
	remw	a5,a4,a5
	sext.w	a5,a5
	bne	a5,zero,.L7
	lw	a5,-20(s0)
	addi	a5,a5,-16
	add	a5,a5,s0
	li	a4,66
	sb	a4,-504(a5)
	j	.L4
.L7:
	lw	a5,-20(s0)
	andi	a4,a5,0xff
	lw	a5,-20(s0)
	addi	a5,a5,-16
	add	a5,a5,s0
	sb	a4,-504(a5)
.L4:
	lw	a5,-20(s0)
	mv	a4,a5
	lw	a5,-532(s0)
	sext.w	a4,a4
	sext.w	a5,a5
	blt	a4,a5,.L8
	nop
	nop
	ld	s0,536(sp)
	addi	sp,sp,544
	jr	ra
	.size	fizzbuzz, .-fizzbuzz
	.ident	"GCC: (g5964b5cd727) 11.1.0"
