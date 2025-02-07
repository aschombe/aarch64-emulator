#include "core/Machine.h"
#include "core/Error.h"

#include <stdio.h>

uint64_t mapMemoryAddress(uint64_t addr) {
    CMachine* pMachine = CMachine::GetCurrentMachine();
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddr = 0;

    if (pMachine == NULL) {
        pErrorHandler->SetError(emulator_err_t::NO_MACHINE_ERR);
        return -1;
    }

    uMappedAddr = addr - pMachine->GetMemBottom();
    if ((pMachine->GetMemBottom() > addr) || uMappedAddr >= MEM_SIZE) {
        pErrorHandler->SetError(emulator_err_t::INVALID_ADDRESS_ERR);
        return -1;
    }

    return uMappedAddr;
}

uint64_t CMachine::ReadQuadwordAt(uint64_t address) {
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddress = mapMemoryAddress(address);
    // TODO: IMPLEMENT
    return -1;
}

uint32_t CMachine::ReadWordAt(uint64_t address) {
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddress = mapMemoryAddress(address);
    // TODO: IMPLEMENT
    return -1;
}

uint16_t CMachine::ReadShortAt(uint64_t address) {
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddress = mapMemoryAddress(address);
    // TODO: IMPLEMENT
    return -1;
}

uint8_t CMachine::ReadByteAt(uint64_t address) {
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddress = mapMemoryAddress(address);
    // TODO: IMPLEMENT
    return -1;
}

bool CMachine::WriteQuadwordAt(uint64_t address, uint64_t* data) {
    // TODO: implement
    return false;
}

bool CMachine::WriteWordAt(uint64_t address, uint32_t* data) {
    // TODO: implement
    return false;
}

bool CMachine::WriteShortAt(uint64_t address, uint16_t* data) {
    // TODO: implement
    return false;
}

bool CMachine::WriteByteAt(uint64_t address, uint8_t* data) {
    // TODO: implement
    return false;
}