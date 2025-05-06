#include "core/Error.h"

bool CEmulatorErrorHandler::HasError(EmulatorError err) {
    return m_uCurrentError & err;
}

bool CEmulatorErrorHandler::SetError(EmulatorError err) {
    if (HasError(err))
        return false;
    m_uCurrentError |= err;
    return true;
}

bool CEmulatorErrorHandler::HandleError(EmulatorError err) {
    if (HasError(err))
        return false;
    m_uCurrentError &= ~err;
    return true;
}