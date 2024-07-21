#include <os/log.h>

os_log_t wrapped_get_default_log() {
    return OS_LOG_DEFAULT;
}

bool wrapped_os_log_type_enabled(os_log_t log, os_log_type_t type) {
    return os_log_type_enabled(log, type);
}

void wrapped_os_log_with_type(os_log_t log, os_log_type_t type, const char* message) {
    os_log_with_type(log, type, "%{public}s", message);
}
