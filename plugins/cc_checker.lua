local callee_saved_regs = { 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29 }
local caller_saved_regs = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18 }
local lr_reg = 30
local sp_reg = 32

local call_stack = {}
local depth = 0
local cc_violations = {} -- Global registry

local initial_sp = nil
local sp_before_instruction = nil

local function snapshot(regs)
	local s = {}
	for _, r in ipairs(regs) do
		s[r] = cpu:get_reg(r)
	end
	return s
end

local function record_violation(reg, depth, reg_type, detail)
	if not cc_violations[reg] then
		cc_violations[reg] = { count = 0, entries = {} }
	end
	local entry = string.format("Depth %d: %s - %s", depth, reg_type, detail)
	table.insert(cc_violations[reg].entries, entry)
	cc_violations[reg].count = cc_violations[reg].count + 1
end

local function diff(before, after, regs, label, depth_override)
	local use_depth = depth_override or depth
	for _, r in ipairs(regs) do
		if before[r] ~= after[r] then
			local msg =
				string.format("[Depth %d] %s x%d changed (0x%X → 0x%X)", use_depth, label, r, before[r], after[r])
			cpu:log(msg)
			record_violation(r, use_depth, label, msg)
		end
	end
end

function on_plugin_load()
	depth = 0
	call_stack = {}
	cc_violations = {}
	initial_sp = cpu:get_reg(sp_reg)
	cpu:log("CC Checker initialized with detailed result tracking.")
end

function pre_pc_increment()
	sp_before_instruction = cpu:get_reg(sp_reg)
	return false
end

function post_pc_increment()
	local sp_after = cpu:get_reg(sp_reg)
	if sp_after ~= sp_before_instruction then
		local msg = string.format(
			"Stack pointer changed during instruction! Before: 0x%X After: 0x%X",
			sp_before_instruction,
			sp_after
		)
		cpu:log("[INSTR] " .. msg)
		record_violation("SP", depth, "Stack-pointer", msg)
	end
end

function pre_bl()
	depth = depth + 1
	call_stack[depth] = {
		lr_in = cpu:get_reg(lr_reg),
		callee_saved_in = snapshot(callee_saved_regs),
		caller_saved_in = snapshot(caller_saved_regs),
	}
	return false
end

function post_bl()
	local frame = call_stack[depth]
	if not frame then
		return
	end
	local caller = call_stack[depth - 1]

	if caller then
		local caller_after = snapshot(caller_saved_regs)
		diff(caller.caller_saved_in, caller_after, caller_saved_regs, "Caller-saved", depth - 1)
	end
end

function pre_ret()
	local frame = call_stack[depth]
	if not frame then
		return
	end
	frame.callee_saved_pre_ret = snapshot(callee_saved_regs)
	return false
end

function post_ret()
	local frame = call_stack[depth]
	if not frame then
		return
	end

	local callee_after = snapshot(callee_saved_regs)
	local lr_now = cpu:get_reg(lr_reg)

	diff(frame.callee_saved_in, callee_after, callee_saved_regs, "Callee-saved", depth)

	if lr_now ~= frame.lr_in then
		local msg = string.format("[Depth %d] LR mismatch on RET (0x%X → 0x%X)", depth, frame.lr_in, lr_now)
		cpu:log(msg)
		record_violation(lr_reg, depth, "Link-register", msg)
	end

	call_stack[depth] = nil
	depth = depth - 1
end

function on_plugin_unload()
	local current_sp = cpu:get_reg(sp_reg)
	if current_sp ~= initial_sp then
		-- local msg =
		-- 	string.format("Stack pointer mismatch at unload! Initial: 0x%X Current: 0x%X", initial_sp, current_sp)
		-- cpu:log(msg)
		record_violation("SP", 0, "Stack-pointer", msg)
	end

	cpu:log("=== Calling Convention Check Summary ===")
	local any = false
	for reg, data in pairs(cc_violations) do
		any = true
		-- cpu:log(string.format("x%-2s had %d violations", tostring(reg), data.count))
		-- if the register is SP or LR, then don't put an 'x' before initial_reg
		local initial_reg = tostring(reg)
		if reg ~= "SP" and reg ~= tostring(lr_reg) then
			initial_reg = "x" .. initial_reg
		end
		cpu:log(string.format("%-4s had %d violations", initial_reg, data.count))
		for _, details in ipairs(data.entries) do
			cpu:log("   → " .. details)
		end
	end
	if not any then
		cpu:log("No violations detected. All preserved registers maintained correctly.")
	else
		cpu:log("=== End of Report (Review violations above) ===")
	end
end
