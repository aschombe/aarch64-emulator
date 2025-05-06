#include <stdio.h>

#include "core/Error.h"
#include "core/Machine.h"
#include "util/Logging.h"

U64 CMachine::MapMemoryAddress(U64 addr) {
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();
    U64 uMappedAddr = 0;

    uMappedAddr = addr - this->GetMemBottom();
    if ((this->GetMemBottom() > addr) || uMappedAddr >= MEM_SIZE) {
        pErrorHandler->SetError(EmulatorError::INVALID_ADDRESS_ERR);
        return -1;
    }

    return uMappedAddr;
}

U64 CMachine::ReadQuadwordAt(U64 address) {
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();
    U64 uMappedAddress = this->MapMemoryAddress(address);
    U64 data = 0;
    SZ i = 0;

    if (uMappedAddress == (U64)-1) {
        if (pErrorHandler->HasError(EmulatorError::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(EmulatorError::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }

        return -1;
    }

    for (i = 0; i < 8; i++) {
        data |= m_aMemory[uMappedAddress + i] << (i * 8);
    }

    return data;
}

U32 CMachine::ReadWordAt(U64 address) {
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();
    U64 uMappedAddress = this->MapMemoryAddress(address);
    U32 data = 0;
    SZ i = 0;

    if (uMappedAddress == (U64)-1) {
        if (pErrorHandler->HasError(EmulatorError::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(EmulatorError::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return -1;
    }

    for (i = 0; i < 4; i++) {
        data |= m_aMemory[uMappedAddress + i] << (i * 8);
    }

    return data;
}

U16 CMachine::ReadShortAt(U64 address) {
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();
    U64 uMappedAddress = this->MapMemoryAddress(address);
    U16 data = 0;
    SZ i = 0;

    if (uMappedAddress == (U64)-1) {
        if (pErrorHandler->HasError(EmulatorError::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(EmulatorError::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return -1;
    }

    for (i = 0; i < 2; i++) {
        data |= m_aMemory[uMappedAddress + i] << (i * 8);
    }

    return data;
}

U8 CMachine::ReadByteAt(U64 address) {
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();
    U64 uMappedAddress = this->MapMemoryAddress(address);
    U8 data = 0;

    if (uMappedAddress == (U64)-1) {
        if (pErrorHandler->HasError(EmulatorError::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(EmulatorError::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return -1;
    }

    data = m_aMemory[uMappedAddress];

    return data;
}

bool CMachine::WriteQuadwordAt(U64 address, U64 *data) {
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();
    U64 uMappedAddress = this->MapMemoryAddress(address);
    SZ i = 0;

    if (uMappedAddress == (U64)-1) {
        if (pErrorHandler->HasError(EmulatorError::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(EmulatorError::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return false;
    }

    for (i = 0; i < 8; i++) {
        m_aMemory[uMappedAddress + i] = (U8)(*data >> i * 8) & 0xff;
    }

    return true;
}

bool CMachine::WriteWordAt(U64 address, U32 *data) {
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();
    U64 uMappedAddress = this->MapMemoryAddress(address);
    SZ i = 0;

    if (uMappedAddress == (U64)-1) {
        if (pErrorHandler->HasError(EmulatorError::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(EmulatorError::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return false;
    }

    for (i = 0; i < 4; i++) {
        m_aMemory[uMappedAddress + i] = (U8)(*data >> i * 8) & 0xff;
    }

    return true;
}

bool CMachine::WriteShortAt(U64 address, U16 *data) {
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();
    U64 uMappedAddress = this->MapMemoryAddress(address);
    SZ i = 0;

    if (uMappedAddress == (U64)-1) {
        if (pErrorHandler->HasError(EmulatorError::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(EmulatorError::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return false;
    }

    for (i = 0; i < 2; i++) {
        m_aMemory[uMappedAddress + i] = (U8)(*data >> i * 8) & 0xff;
    }

    return true;
}

bool CMachine::WriteByteAt(U64 address, U8 *data) {
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();
    U64 uMappedAddress = this->MapMemoryAddress(address);

    if (uMappedAddress == (U64)-1) {
        if (pErrorHandler->HasError(EmulatorError::INVALID_ADDRESS_ERR)) {
            pErrorHandler->HandleError(EmulatorError::INVALID_ADDRESS_ERR);
            dologm(ERROR, "address %lu could not be mapped!\n", address);
        }
        return false;
    }

    m_aMemory[uMappedAddress] = (U8)*data & 0xff;

    return true;
}

U32 CMachine::ReadInstructionAt(U64 address) {
    // TODO: Implement
    return -1;
}
