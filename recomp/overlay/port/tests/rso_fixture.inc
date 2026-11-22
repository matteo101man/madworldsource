// Synthetic fixture from the production RSO emitter.
// synthetic_export_with_private_constructor  0x810000F8-0x81000110
static void body_810000F8(PPCContext& c, uint8_t* m, uint32_t entry) {
	PPC_ENTER(entry);
	switch (entry) { case 0x81000108u: goto L_81000108; default: break; }
	c.r[3] = 0u + ((uint32_t)(int32_t)(int16_t)ld16(m, mw_rso_runtime_pc(m, 0x810000FAu)) << 16);
	c.r[3] = c.r[3] + (uint32_t)(int32_t)(int16_t)ld16(m, mw_rso_runtime_pc(m, 0x810000FEu));
	c.r[4] = ld32(m, (c.r[3] + (uint32_t)(int32_t)(int16_t)ld16(m, mw_rso_runtime_pc(m, 0x81000102u))));
	return;
L_81000108:
	c.lr = mw_rso_runtime_pc(m, 0x8100010Cu); ppc_call_indirect(c, m, mw_rso_branch_target(m, 0x81000108u));
	return;
}
PPC_FUNC(f_810000F8) { body_810000F8(c, m, 0x810000F8u); }
PPC_FUNC(f_81000108) { body_810000F8(c, m, 0x81000108u); }