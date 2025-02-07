#include <stdio.h>

#include "core/Machine.h"
#include "core/Error.h"
#include "util/Logging.h"

uint64_t CMachine::MapMemoryAddress(uint64_t addr) {
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddr = 0;

    uMappedAddr = addr - this->GetMemBottom();
    if ((this->GetMemBottom() > addr) || uMappedAddr >= MEM_SIZE) {
        pErrorHandler->SetError(emulator_err_t::INVALID_ADDRESS_ERR);
        return -1;
    }

    return uMappedAddr;
}

uint64_t CMachine::ReadQuadwordAt(uint64_t address) {
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddress = this->MapMemoryAddress(address);
    uint64_t data = 0;
    size_t i = 0;
    
    if (uMappedAddress == (uint64_t) -1) {
        if (pErrorHandler->HasError(emulator_err_t::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(emulator_err_t::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }

        return -1;
    }

    for (i = 0; i < 8; i++) {
        data |= m_aMemory[uMappedAddress + i] << (i*8);
    }

    return data;
}

uint32_t CMachine::ReadWordAt(uint64_t address) {
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddress = this->MapMemoryAddress(address);
    uint32_t data = 0;
    size_t i = 0;
    
    if (uMappedAddress == (uint64_t) -1) {
        if (pErrorHandler->HasError(emulator_err_t::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(emulator_err_t::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return -1;
    }

    for (i = 0; i < 4; i++) {
        data |= m_aMemory[uMappedAddress + i] << (i*8);
    }

    return data;
}

uint16_t CMachine::ReadShortAt(uint64_t address) {
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddress = this->MapMemoryAddress(address);
    uint16_t data = 0;
    size_t i = 0;
    
    if (uMappedAddress == (uint64_t) -1) {
        if (pErrorHandler->HasError(emulator_err_t::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(emulator_err_t::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return -1;
    }

    for (i = 0; i < 2; i++) {
        data |= m_aMemory[uMappedAddress + i] << (i*8);
    }

    return data;
}

uint8_t CMachine::ReadByteAt(uint64_t address) {
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddress = this->MapMemoryAddress(address);
    uint8_t data = 0;
    
    if (uMappedAddress == (uint64_t) -1) {
        if (pErrorHandler->HasError(emulator_err_t::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(emulator_err_t::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return -1;
    }

    data = m_aMemory[uMappedAddress];

    return data;
}

bool CMachine::WriteQuadwordAt(uint64_t address, uint64_t* data) {
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddress = this->MapMemoryAddress(address);
    size_t i = 0;
    
    if (uMappedAddress == (uint64_t) -1) {
        if (pErrorHandler->HasError(emulator_err_t::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(emulator_err_t::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return false;
    }

    for (i = 0; i < 8; i++) {
        m_aMemory[uMappedAddress + i] = (uint8_t) (*data >> i*8) & 0xff;
    }

    return true;
}

bool CMachine::WriteWordAt(uint64_t address, uint32_t* data) {
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddress = this->MapMemoryAddress(address);
    size_t i = 0;
    
    if (uMappedAddress == (uint64_t) -1) {
        if (pErrorHandler->HasError(emulator_err_t::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(emulator_err_t::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return false;
    }

    for (i = 0; i < 4; i++) {
        m_aMemory[uMappedAddress + i] = (uint8_t) (*data >> i*8) & 0xff;
    }

    return true;
}

bool CMachine::WriteShortAt(uint64_t address, uint16_t* data) {
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddress = this->MapMemoryAddress(address);
    size_t i = 0;
    
    if (uMappedAddress == (uint64_t) -1) {
        if (pErrorHandler->HasError(emulator_err_t::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(emulator_err_t::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return false;
    }

    for (i = 0; i < 2; i++) {
        m_aMemory[uMappedAddress + i] = (uint8_t) (*data >> i*8) & 0xff;
    }

    return true;
}

bool CMachine::WriteByteAt(uint64_t address, uint8_t* data) {
    CEmulatorErrorHandler* pErrorHandler = CEmulatorErrorHandler::GetErrorHandler();
    uint64_t uMappedAddress = this->MapMemoryAddress(address);
    
    if (uMappedAddress == (uint64_t) -1) {
        if (pErrorHandler->HasError(emulator_err_t::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(emulator_err_t::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return false;
    }

    m_aMemory[uMappedAddress] = (uint8_t) *data & 0xff;

    return true;
}