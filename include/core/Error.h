#pragma once

#include <stdlib.h>
#include <stdint.h>

#include "core/CoreTypes.h"

class CEmulatorErrorHandler;

static CEmulatorErrorHandler* pErrorHandler;

// must be defined in powers of 2
typedef enum {
    NONE                   = 0,
    NO_MACHINE_ERR         = 1,
    SEGMENTATION_FAULT_ERR = 2,
    INVALID_INSN_FMT_ERR   = 4,
    INVALID_ADDRESS_ERR    = 8,
    NO_ENTRY_FOUND_ERR     = 16,
    FD_DOESNT_EXIST_ERR    = 32,
    NOT_ENOUGH_SPACE_ERR   = 64,
} EmulatorError;

typedef I32 ErrorMask;

class CEmulatorErrorHandler {
public:
    static CEmulatorErrorHandler* GetErrorHandler() { return pErrorHandler; }

    CEmulatorErrorHandler() { 
        m_uCurrentError =  EmulatorError::NONE;
        pErrorHandler = this;
    }
    ~CEmulatorErrorHandler() { pErrorHandler = NULL; }

    ErrorMask GetAllErrors() { return m_uCurrentError; }
    void ClearAllErrors() { m_uCurrentError = EmulatorError::NONE; }

    bool HasError(EmulatorError err);
    bool SetError(EmulatorError err);
    bool HandleError(EmulatorError err);

private:
    I32 m_uCurrentError;
};
