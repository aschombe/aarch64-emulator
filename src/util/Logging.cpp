#include "util/Logging.h"

const char* stringOfLogLevel(loglevel_t level) {
    switch (level) {
        case DEBUG:        return "DEBUG";
        case EXTRA_STATUS: return "EXTRA";
        case STATUS:       return "STATUS";
        case ERROR:        return "ERROR";
    }
    
    return "UNKNOWN";
}