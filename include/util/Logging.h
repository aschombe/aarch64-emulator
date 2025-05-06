#pragma once

#include <stdio.h>
#include <string.h>
#include <time.h>

// lower you go, the more information you get
typedef enum {
    DEBUG = 1,
    EXTRA_STATUS = 2,
    STATUS = 4,
    ERROR = 8,
} LogLevel;

#ifndef LOG_MASK
#define LOG_MASK 0xf
#endif

#define __FILENAME__                                                           \
    (strrchr(__FILE__, '/') ? strrchr(__FILE__, '/') + 1 : __FILE__)
const char *stringOfLogLevel(LogLevel level);

void dologm(LogLevel mask, const char *format, ...);

#define dolog(...) dologm(STATUS, __VA_ARGS__)