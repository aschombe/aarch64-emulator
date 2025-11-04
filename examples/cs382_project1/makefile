# === AArch64 Assembly Project Makefile ===

# Tools
AS = aarch64-linux-gnu-as
LD = aarch64-linux-gnu-ld

# Flags
ASFLAGS = -g
LDFLAGS = 

# Files
SRC = main.s concat_array.s count_specs.s itoascii.s pringle.s
OBJ = $(SRC:.s=.o)
TARGET = program

# Default rule
all: $(TARGET)

# Linking
$(TARGET): $(OBJ)
	$(LD) $(LDFLAGS) -o $@ $(OBJ)

# Assembling
%.o: %.s
	$(AS) $(ASFLAGS) -o $@ $<

# Clean
clean:
	rm -f $(OBJ) $(TARGET)

# Rebuild
rebuild: clean all

