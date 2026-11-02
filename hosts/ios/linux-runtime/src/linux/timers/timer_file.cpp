#include "linux/timers/timer_file.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include "linux/timers/timer_service.h"
#include "linux/timers/timer_state.h"
#include <fcntl.h>

namespace operit::arm64 {
TimerFile::TimerFile(std::shared_ptr<TimerService> service, std::shared_ptr<timers::State> state)
    : service_(std::move(service)), state_(std::move(state)) {}
std::shared_ptr<TimerFile> TimerFile::create(int32_t clock_id) {
    if (clock_id != 0 && clock_id != 1)
        guest_error(22);
    auto service = TimerService::acquire();
    auto state = service->create(clock_id);
    return std::shared_ptr<TimerFile>(new TimerFile(std::move(service), std::move(state)));
}
TimerFile::~TimerFile() {
    service_->remove(state_);
}
FD TimerFile::duplicate_descriptor() const {
    FD descriptor(::fcntl(state_->reader.get(), F_DUPFD_CLOEXEC, 0));
    check_host(descriptor.get());
    return descriptor;
}
TimerSpec TimerFile::set_time(uint32_t flags, const TimerSpec &value) {
    return service_->set_time(*state_, flags, value);
}
TimerSpec TimerFile::get_time() {
    return service_->get_time(*state_);
}
uint64_t TimerFile::read() {
    return service_->read(*state_);
}
int32_t TimerFile::clock_id() const {
    return state_->clock_id;
}
} // namespace operit::arm64
