#include "util/Logging.h"
#include <stdarg.h>

const char *stringOfLogLevel(LogLevel level) {
    switch (level) {
    case DEBUG:
        return "DEBUG";
    case EXTRA_STATUS:
        return "EXTRA";
    case STATUS:
        return "STATUS";
    case ERROR:
        return "ERROR";
    }

    return "UNKNOWN";
}

void dologm(LogLevel mask, const char *format, ...) {
    if ((mask & LOG_MASK) == 0)
        return;

    va_list args;
    va_start(args, format);

    time_t t = time(NULL);
    struct tm now = *localtime(&t);
    char dtbuf[50] = {0};
    snprintf(dtbuf, sizeof(dtbuf), "%02d/%02d/%02d, %02d:%02d:%02d",
             now.tm_mon + 1, now.tm_mday, now.tm_year + 1900, now.tm_hour,
             now.tm_min, now.tm_sec);
    printf("%-8s [%s] %25s:%-10d ", stringOfLogLevel(mask), dtbuf, __FILENAME__,
           __LINE__);
    vprintf(format, args);

    va_end(args);
}
