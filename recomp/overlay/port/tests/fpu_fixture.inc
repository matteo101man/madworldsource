// Synthetic instructions from the production emitter.
case 0: { c.f[1].ps0 = c.f[2].ps0 + c.f[3].ps0; break; } // fadd
case 1: { c.f[1].ps0 = c.f[2].ps0 - c.f[3].ps0; break; } // fsub
case 2: { c.f[1].ps0 = c.f[2].ps0 / c.f[3].ps0; break; } // fdiv
case 3: { c.f[1].ps0 = c.f[2].ps0 * c.f[4].ps0; break; } // fmul
case 4: { c.f[1].ps0 = std::fma(c.f[2].ps0, c.f[4].ps0, c.f[3].ps0); break; } // fmadd
case 5: { c.f[1].ps0 = std::fma(c.f[2].ps0, c.f[4].ps0, -c.f[3].ps0); break; } // fmsub
case 6: { c.f[1].ps0 = -std::fma(c.f[2].ps0, c.f[4].ps0, c.f[3].ps0); break; } // fnmadd
case 7: { c.f[1].ps0 = -std::fma(c.f[2].ps0, c.f[4].ps0, -c.f[3].ps0); break; } // fnmsub
case 8: { { double r = ppc_round_single(c.f[2].ps0 + c.f[3].ps0); c.f[1].ps0 = r; c.f[1].ps1 = r; } break; } // fadds
case 9: { { double r = ppc_round_single(c.f[2].ps0 - c.f[3].ps0); c.f[1].ps0 = r; c.f[1].ps1 = r; } break; } // fsubs
case 10: { { double r = ppc_round_single(c.f[2].ps0 / c.f[3].ps0); c.f[1].ps0 = r; c.f[1].ps1 = r; } break; } // fdivs
case 11: { { double r = ppc_round_single(c.f[2].ps0 * c.f[4].ps0); c.f[1].ps0 = r; c.f[1].ps1 = r; } break; } // fmuls
case 12: { { double r = ppc_round_single(std::fma(c.f[2].ps0, c.f[4].ps0, c.f[3].ps0)); c.f[1].ps0 = r; c.f[1].ps1 = r; } break; } // fmadds
case 13: { { double r = ppc_round_single(std::fma(c.f[2].ps0, c.f[4].ps0, -c.f[3].ps0)); c.f[1].ps0 = r; c.f[1].ps1 = r; } break; } // fmsubs
case 14: { { double r = ppc_round_single(-std::fma(c.f[2].ps0, c.f[4].ps0, c.f[3].ps0)); c.f[1].ps0 = r; c.f[1].ps1 = r; } break; } // fnmadds
case 15: { { double r = ppc_round_single(-std::fma(c.f[2].ps0, c.f[4].ps0, -c.f[3].ps0)); c.f[1].ps0 = r; c.f[1].ps1 = r; } break; } // fnmsubs
case 16: { { double r = ppc_round_single(c.f[3].ps0); c.f[1].ps0 = r; c.f[1].ps1 = r; } break; } // frsp
case 17: { c.f[1].u0 = ppc_fctiw(c, c.f[3].ps0); break; } // fctiw
case 18: { c.f[1].u0 = ppc_fctiwz(c, c.f[3].ps0); break; } // fctiwz
case 19: { c.f[1].u0 = c.f[3].u0; break; } // fmr
case 20: { c.f[1].u0 = c.f[3].u0 ^ 0x8000000000000000ull; break; } // fneg
case 21: { c.f[1].u0 = c.f[3].u0 & 0x7FFFFFFFFFFFFFFFull; break; } // fabs
case 22: { c.f[1].u0 = c.f[3].u0 | 0x8000000000000000ull; break; } // fnabs
