#include "core/FileSystem.h"
#include "core/Error.h"
#include "util/Logging.h"

#include <errno.h>
#include <fcntl.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

CFileSystem::CFileSystem() {
    m_pFileDescriptors =
        (FDescriptor *)calloc(MAX_FILE_COUNT, sizeof(FDescriptor));
    m_pFileDescMap = (FDescMap *)calloc(MAX_FILE_COUNT, sizeof(FDescMap));
    pFileSystem = this;
}

CFileSystem::~CFileSystem() {
    for (U8 currFd = 0; currFd < m_uCurrentFileCount; currFd++) {
        if (m_pFileDescriptors[currFd].bIsOpen)
            this->Close(currFd);
    }

    free(m_pFileDescriptors);
    free(m_pFileDescMap);

    m_pFileDescriptors = NULL;
    m_pFileDescMap = NULL;
    pFileSystem = NULL;
}

U16 CFileSystem::Open(char *filename, int flags, bool syncEnabled) {
    int nErrno = 0; // set to 0 to avoid compiler warnings
    int nFd;
    struct stat sFStats;
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();

    // check if the file exists if we don't want to create it
    if ((flags & O_CREAT) != 0 && (errno = stat(filename, &sFStats)) != 0) {
        pErrorHandler->SetError(EmulatorError::NO_ENTRY_FOUND_ERR);
        return nErrno < 0 ? (U16)nErrno : (U16)-1;
    }

    nFd = open(filename, flags);

    m_pFileDescMap[m_uNextFdNum].bSyncEnabled = syncEnabled;
    m_pFileDescMap[m_uNextFdNum].nOrigFd = nFd;
    m_pFileDescMap[m_uNextFdNum].sFilename = filename;

    m_pFileDescriptors[m_uNextFdNum].uFdNumber = m_uNextFdNum;
    m_pFileDescriptors[m_uNextFdNum].uLength = MAX_FILE_SIZE;
    m_pFileDescriptors[m_uNextFdNum].uSeekPos = 0;
    m_pFileDescriptors[m_uNextFdNum].uData =
        (U8 *)calloc(MAX_FILE_SIZE, sizeof(U8));

    m_uNextFdNum++; // increment fd number

    return m_pFileDescriptors[m_uNextFdNum].uFdNumber;
}

U16 CFileSystem::Close(U16 fd) {
    int nErrno;
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();

    if (!m_pFileDescriptors[fd].bIsOpen) {
        pErrorHandler->SetError(EmulatorError::FD_DOESNT_EXIST_ERR);
        return 0;
    }

    m_pFileDescriptors[fd].bIsOpen = false; // will be cleared anyway...
    nErrno = close(m_pFileDescMap[fd].nOrigFd);
    free(m_pFileDescriptors[fd].uData);

    // clear file descriptor mapping
    // possible source of memory corruption...?
    memset(&m_pFileDescMap[fd], 0, sizeof(FDescMap));
    memset(&m_pFileDescriptors[fd], 0, sizeof(FDescriptor));

    return (U16)nErrno;
}

U32 CFileSystem::Read(U16 fd, U8 *data, U32 count) {
    U32 nCopied = 0;
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();
    FDescriptor sFDesc = m_pFileDescriptors[fd];

    if (!sFDesc.bIsOpen) {
        pErrorHandler->SetError(EmulatorError::FD_DOESNT_EXIST_ERR);
        return -1;
    }

    while (nCopied < count) {
        data[nCopied++] = sFDesc.uData[sFDesc.uSeekPos++];
    }

    return nCopied;
}

U32 CFileSystem::Write(U16 fd, U8 *data, U32 count) {
    U32 nCopied = 0;
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();
    FDescriptor sFDesc = m_pFileDescriptors[fd];

    if (!sFDesc.bIsOpen) {
        pErrorHandler->SetError(EmulatorError::FD_DOESNT_EXIST_ERR);
        return -1;
    }

    while (nCopied < count) {
        sFDesc.uData[sFDesc.uSeekPos++] = data[nCopied++];
    }

    return nCopied;
}

U32 CFileSystem::Seek(U16 fd, U16 offset, FSeekMode mode) {
    CEmulatorErrorHandler *pErrorHandler =
        CEmulatorErrorHandler::GetErrorHandler();
    FDescriptor sFDesc = m_pFileDescriptors[fd];

    if (!sFDesc.bIsOpen) {
        pErrorHandler->SetError(EmulatorError::FD_DOESNT_EXIST_ERR);
        return -1;
    }

    switch (mode) {
    case FSeekMode::FSEEK_CUR:
        sFDesc.uSeekPos += offset;
        break;
    case FSeekMode::FSEEK_SET:
        sFDesc.uSeekPos = offset;
        break;
    case FSeekMode::FSEEK_END:
        if (offset > 0) {
            pErrorHandler->SetError(EmulatorError::INVALID_LOCATION_ERR);
            return -1;
        }
        sFDesc.uSeekPos = sFDesc.uLength + offset;
        break;
    default:
        pErrorHandler->SetError(EmulatorError::INVALID_LOCATION_ERR);
        return -1;
    }

    return sFDesc.uSeekPos;
}
