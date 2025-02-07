#ifndef ERROR_H
#define ERROR_H

#include <stdlib.h>
#include <stdint.h>

class CEmulatorErrorHandler;

static CEmulatorErrorHandler* pErrorHandler = NULL;

// must be defined in powers of 2
typedef enum {
    NONE                   = 0,
    NO_MACHINE_ERR         = 1,
    SEGMENTATION_FAULT_ERR = 2,
    INVALID_INSN_FMT_ERR   = 4,
    INVALID_ADDRESS_ERR    = 8,
} emulator_err_t;

typedef uint32_t err_mask_t;

class CEmulatorErrorHandler {
public:
    static CEmulatorErrorHandler* GetErrorHandler() { return pErrorHandler; }

    CEmulatorErrorHandler() { m_uCurrentError =  emulator_err_t::NONE; pErrorHandler = this; }
    ~CEmulatorErrorHandler() { pErrorHandler = NULL; }

    err_mask_t GetAllErrors() { return m_uCurrentError; }
    void ClearAllErrors() { m_uCurrentError = emulator_err_t::NONE; }

    bool HasError(emulator_err_t err);
    bool SetError(emulator_err_t err);
    bool HandleError(emulator_err_t err);

private:
    uint32_t m_uCurrentError;
};

#endif