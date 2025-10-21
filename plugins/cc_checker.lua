--[[
Calling convention:
X0-X7: Argument registers (These become callee-saved if used)
X8-X17: Callee-saved registers
X18-X29: Caller-saved registers
X30: Link register
SP: Stack pointer

General idea for checking CC:
- First and foremost, intial and final SP must match
- Callee-saved registers must be before and after procedure call
- Link register must be properly set before and after procedure call
--]]

-- Global variable for tracking the stack pointer
local sp = 0x0000000040000000

function on_plugin_load()
	-- grab the initial stack pointer value
	sp = cpu:get_reg(32) -- Index 32 corresponds to SP
	cpu:log("Stack pointer initialized to 0x" .. string.format("%X", sp))
end

function on_plugin_unload()
	-- if the current stack pointer doesn't match the saved one, log a warning
	local current_sp = cpu:get_reg(32)
	if current_sp ~= sp then
		cpu:log(
			string.format(
				"[CC Checker] Warning: Stack pointer mismatch! Initial SP: 0x%X, Final SP: 0x%X",
				sp,
				current_sp
			)
		)
	end

	print(
		"[CC Checker] on_plugin_unload: initial SP: 0x"
			.. string.format("%X", sp)
			.. " current SP: 0x"
			.. string.format("%X", current_sp)
	)

	local x1 = cpu:get_reg(1)
	print("[CC Checker] Final value of X1: 0x" .. string.format("%X", x1))
end

-- function on_pre_exec()
-- 	-- print("[Lua Plugin] Before instruction execution")
-- 	return false -- false means "do not skip instruction"
-- end
--
-- Hook called after each instruction execution
function on_post_exec()
	local sp = cpu:get_reg(32)
	local x1 = cpu:get_reg(1)
	cpu:log(string.format("SP: 0x%X, X1: 0x%X", sp, x1))
end
--
-- -- Hook called before syscall execution
-- function on_pre_syscall()
-- 	-- print("[Lua Plugin] Before syscall execution")
-- 	return false -- false means "do not skip syscall"
-- end
--
-- -- Hook called after syscall execution
-- function on_post_syscall()
-- 	-- print("[Lua Plugin] After syscall execution")
-- end
