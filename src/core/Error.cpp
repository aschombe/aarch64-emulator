#include "core/Error.h"

bool CEmulatorErrorHandler::HasError(emulator_err_t err) {
    return m_uCurrentError & err; 
}

bool CEmulatorErrorHandler::SetError(emulator_err_t err) {
    if (HasError(err)) return false;
    m_uCurrentError |= err;
    return true;
}

bool CEmulatorErrorHandler::HandleError(emulator_err_t err) {
    if (HasError(err)) return false;
    m_uCurrentError &= ~err;
}