-- Hook called before each instruction execution
function on_pre_exec()
	print("[Lua Plugin] Before instruction execution")
	return false -- false means "do not skip instruction"
end

-- Hook called after each instruction execution
function on_post_exec()
	print("[Lua Plugin] After instruction execution")
end

-- Hook called before syscall execution
function on_pre_syscall()
	print("[Lua Plugin] Before syscall execution")
	return false -- false means "do not skip syscall"
end

-- Hook called after syscall execution
function on_post_syscall()
	print("[Lua Plugin] After syscall execution")
end
