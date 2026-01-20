#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import argparse
import bisect
import re
import sys
from dataclasses import dataclass
from typing import List, Optional


# Disassembly function header example:
# 00000000800090b0 <userret>:
FUNC_HDR_RE = re.compile(r'^\s*([0-9A-Fa-f]+)\s+<([^>]+)>:\s*$')

# Log line example:
# [2026-01-18T09:33:55Z INFO  rv_emu::cpu] Block execution: 0x8000377c to 0x8000379c
LOG_RE = re.compile(
    r'^\[(?P<date>.+?)\s+INFO\s+rv_emu::cpu\]\s+Block execution:\s+'
    r'(?P<start>0x[0-9A-Fa-f]+)\s+to\s+(?P<end>0x[0-9A-Fa-f]+)\s*$'
)


@dataclass(frozen=True)
class FuncRange:
    name: str
    start: int
    end_exclusive: int  # end is exclusive; last function uses a huge sentinel


SENTINEL_END = (1 << 64)  # big enough for 64-bit address space


def build_function_ranges(disasm_path: str) -> List[FuncRange]:
    """
    Parse disassembly output and build ranges:
      func_i = [start_i, start_{i+1})  (end exclusive)
      last   = [start_last, SENTINEL_END)
    """
    funcs: List[tuple[int, str]] = []

    with open(disasm_path, "r", encoding="utf-8", errors="replace") as f:
        for line in f:
            m = FUNC_HDR_RE.match(line)
            if not m:
                continue
            addr_hex, name = m.group(1), m.group(2)
            start = int(addr_hex, 16)
            funcs.append((start, name))

    funcs.sort(key=lambda x: x[0])
    if not funcs:
        return []

    ranges: List[FuncRange] = []
    for i, (start, name) in enumerate(funcs):
        if i + 1 < len(funcs):
            end = funcs[i + 1][0]
            if end <= start:
                # Defensive: avoid non-positive ranges (duplicate/unsorted inputs)
                end = start + 1
        else:
            end = SENTINEL_END
        ranges.append(FuncRange(name=name, start=start, end_exclusive=end))

    return ranges


def find_function_name(func_ranges: List[FuncRange], addr: int) -> Optional[str]:
    """
    Find the function whose range contains addr.
    Uses binary search over start addresses.
    """
    starts = [fr.start for fr in func_ranges]
    i = bisect.bisect_right(starts, addr) - 1
    if i < 0:
        return None
    fr = func_ranges[i]
    if fr.start <= addr < fr.end_exclusive:
        return fr.name
    return None


def process_log(log_path: str, func_ranges: List[FuncRange]) -> None:
    """
    Extract matching log lines and append function name for start address.
    Output to stdout.
    """
    with open(log_path, "r", encoding="utf-8", errors="replace") as f:
        for raw in f:
            line = raw.rstrip("\n")
            m = LOG_RE.match(line)
            if not m:
                continue

            start_addr = int(m.group("start"), 16)
            func = find_function_name(func_ranges, start_addr) or "UNKNOWN"
            print(f"{line} {func}")


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Extract rv_emu block execution lines and append function name."
    )
    ap.add_argument("disasm", help="Disassembly output file (objdump-like).")
    ap.add_argument("log", help="Emulator execution log file.")
    args = ap.parse_args()

    func_ranges = build_function_ranges(args.disasm)
    if not func_ranges:
        print(
            "ERROR: Could not find any function headers in disassembly. "
            "Expected lines like: 00000000800090b0 <userret>:",
            file=sys.stderr,
        )
        return 2

    process_log(args.log, func_ranges)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
