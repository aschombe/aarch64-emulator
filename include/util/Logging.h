#ifndef LOGGING_H
#define LOGGING_H

#include <stdio.h>
#include <string.h>
#include <time.h>

// lower you go, the more information you get
typedef enum {
    DEBUG        = 1,
    EXTRA_STATUS = 2,
    STATUS       = 4,
    ERROR        = 8,
} LogLevel;

#ifndef LOG_MASK
#define LOG_MASK = 0xf;
#endif

#define __FILENAME__ (strrchr(__FILE__, '/') ? strrchr(__FILE__, '/') + 1 : __FILE__)
const char* stringOfLogLevel(LogLevel level);

#define dologm(mask, ...) if ((mask & LOG_MASK) != 0) { \
    time_t t = time(NULL); \
    struct tm now = *localtime(&t); \
    char dtbuf[50] = {0}; \
    sprintf(dtbuf, "%02d/%02d/%02d, %02d:%02d:%02d", now.tm_mon + 1, now.tm_mday, now.tm_year + 1900, now.tm_hour, now.tm_min, now.tm_sec); \
    printf("%-8s [%s] %25s:%-10d ", stringOfLogLevel(mask), dtbuf, __FILENAME__, __LINE__); \
    printf(__VA_ARGS__); \
}
#define dolog(...) dologm(STATUS, __VA_ARGS__)

#endif