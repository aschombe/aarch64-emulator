#pragma once

#include <stdlib.h>

#include "core/CoreTypes.h"

#define BASE_FD_NUM    3
#define MAX_FILE_COUNT 255
#define MAX_FILE_SIZE  UINT16_MAX

#define OVERLAY_SYNC_ENABLED true

typedef struct {
    int   nOrigFd;
    char* sFilename;
    bool  bSyncEnabled;
} FDescMap;

typedef enum {
    FSEEK_SET,
    FSEEK_CUR,
    FSEEK_END
} FSeekMode;

typedef struct {
    U16 uFdNumber;
    U16 uLength;
    U16 uSeekPos;

    U8* uData;

    bool bIsOpen;
} FDescriptor;

class CFileSystem;

static CFileSystem* pFileSystem = NULL;

// virtual fs to protect system
class CFileSystem {
public:
    CFileSystem();
    ~CFileSystem();

    // implemented in case it is useful, should be revisted
    static CFileSystem* GetCurrentFileSystem() { return pFileSystem; }

    U16 Open(char* filename, int flags, bool syncEnabled);
    U16 Close(U16 fd);
    U32 Read(U16 fd, U8* data, U32 count);
    U32 Write(U16 fd, U8* data, U32 count);

    U32 Seek(U16 fd, U16 offset, FSeekMode mode);
private:
    U16 m_uNextFdNum = BASE_FD_NUM;
    U8  m_uMaxFileCount = MAX_FILE_COUNT;
    U8  m_uCurrentFileCount = 0;

    bool m_bSyncEnabled = OVERLAY_SYNC_ENABLED;

    FDescriptor* m_pFileDescriptors = NULL;
    FDescMap* m_pFileDescMap = NULL;
};