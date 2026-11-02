#include "linux/terminal/termios.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include <algorithm>
#include <cerrno>
#include <sys/ioctl.h>
#include <unistd.h>

namespace operit::arm64::terminal {
namespace {
struct Flag {
    uint32_t guest;
    tcflag_t host;
};
constexpr Flag input_flags[] = {{0x0001, IGNBRK},  {0x0002, BRKINT}, {0x0004, IGNPAR}, {0x0008, PARMRK},
                                {0x0010, INPCK},   {0x0020, ISTRIP}, {0x0040, INLCR},  {0x0080, IGNCR},
                                {0x0100, ICRNL},   {0x0400, IXON},   {0x0800, IXANY},  {0x1000, IXOFF},
                                {0x2000, IMAXBEL}, {0x4000, IUTF8}};
constexpr Flag output_flags[] = {
    {0x0001, OPOST}, {0x0004, ONLCR}, {0x0008, OCRNL}, {0x0010, ONOCR},
    {0x0020, ONLRET}, {0x0040, OFILL}, {0x0080, OFDEL},
#if !defined(__APPLE__)
    {0x0002, OLCUC},
#endif
};
constexpr Flag control_flags[] = {{0x0040, CSTOPB}, {0x0080, CREAD},  {0x0100, PARENB},     {0x0200, PARODD},
                                  {0x0400, HUPCL},  {0x0800, CLOCAL}, {0x80000000, CRTSCTS}};
constexpr Flag local_flags[] = {{0x00001, ISIG},    {0x00002, ICANON},  {0x00008, ECHO},   {0x00010, ECHOE},
                                {0x00020, ECHOK},   {0x00040, ECHONL},  {0x00080, NOFLSH}, {0x00100, TOSTOP},
                                {0x00200, ECHOCTL}, {0x00400, ECHOPRT}, {0x00800, ECHOKE}, {0x01000, FLUSHO},
                                {0x04000, PENDIN},  {0x08000, IEXTEN},  {0x10000, EXTPROC}};
struct Character {
    unsigned guest, host;
    bool count;
};
constexpr Character characters[] = {
    {0, VINTR, false},     {1, VQUIT, false},    {2, VERASE, false},  {3, VKILL, false},
    {4, VEOF, false},      {5, VTIME, true},     {6, VMIN, true},     {8, VSTART, false},
    {9, VSTOP, false},     {10, VSUSP, false},   {11, VEOL, false},   {12, VREPRINT, false},
    {13, VDISCARD, false}, {14, VWERASE, false}, {15, VLNEXT, false}, {16, VEOL2, false}};
constexpr uint32_t baud_mask = 0x100f;
constexpr uint32_t rates[] = {0,       50,      75,      110,     134,     150,     200,    300,
                              600,     1200,    1800,    2400,    4800,    9600,    19200,  38400,
                              57600,   115200,  230400,  460800,  500000,  576000,  921600, 1000000,
                              1152000, 1500000, 2000000, 2500000, 3000000, 3500000, 4000000};
template <size_t N> uint32_t from_native(tcflag_t value, const Flag (&flags)[N]) {
    uint32_t result = 0;
    for (auto flag : flags)
        if ((value & flag.host) == flag.host)
            result |= flag.guest;
    return result;
}
template <size_t N> tcflag_t to_native(uint32_t value, const Flag (&flags)[N]) {
    uint32_t known = 0;
    tcflag_t result = 0;
    for (auto flag : flags) {
        known |= flag.guest;
        if (value & flag.guest)
            result |= flag.host;
    }
    if (value & ~known)
        guest_error(22);
    return result;
}
uint32_t native_rate(speed_t speed) {
#if defined(__APPLE__)
    if (speed > UINT32_MAX)
        guest_error(75);
    return uint32_t(speed);
#else
    return speed_from_bits(uint32_t(speed));
#endif
}
speed_t native_speed(uint32_t rate) {
#if defined(__APPLE__)
    return speed_t(rate);
#else
    uint32_t bits = speed_bits(rate);
    if (bits == 0x1000)
        guest_error(22);
    return speed_t(bits);
#endif
}
uint32_t output_from_native(tcflag_t value) {
    uint32_t result = from_native(value, output_flags);
#if defined(__APPLE__)
    if (value & NLDLY)
        result |= 0x100;
    result |= uint32_t((value & CRDLY) >> 12) << 9;
    if (value & OXTABS)
        result |= 0x1800;
    else
        result |= uint32_t((value & (TAB1 | TAB2)) >> 10) << 11;
    if (value & BSDLY)
        result |= 0x2000;
    if (value & VTDLY)
        result |= 0x4000;
    if (value & FFDLY)
        result |= 0x8000;
#else
    result |= uint32_t(value) & 0xff00;
#endif
    return result;
}
tcflag_t output_to_native(uint32_t value) {
    constexpr uint32_t delays = 0xff00;
    auto result = to_native(value & ~delays, output_flags);
#if defined(__APPLE__)
    if (value & 0x100)
        result |= NL1;
    constexpr tcflag_t carriage[] = {CR0, CR1, CR2, CR3};
    constexpr tcflag_t tabs[] = {TAB0, TAB1, TAB2, OXTABS};
    result |= carriage[(value >> 9) & 3] | tabs[(value >> 11) & 3];
    if (value & 0x2000)
        result |= BS1;
    if (value & 0x4000)
        result |= VT1;
    if (value & 0x8000)
        result |= FF1;
#else
    result |= tcflag_t(value & delays);
#endif
    return result;
}
constexpr Flag modem_flags[] = {{0x001, TIOCM_LE},  {0x002, TIOCM_DTR}, {0x004, TIOCM_RTS},
                                {0x008, TIOCM_ST},  {0x010, TIOCM_SR},  {0x020, TIOCM_CTS},
                                {0x040, TIOCM_CAR}, {0x080, TIOCM_RNG}, {0x100, TIOCM_DSR}};
} // namespace
uint32_t speed_from_bits(uint32_t bits) {
    bits &= baud_mask;
    if (bits == 0x1000)
        guest_error(22);
    unsigned index = bits & 15;
    if (bits & 0x1000)
        index += 15;
    return rates[index];
}
uint32_t speed_bits(uint32_t speed) {
    for (unsigned index = 0; index < sizeof(rates) / sizeof(*rates); ++index)
        if (rates[index] == speed)
            return index < 16 ? index : 0x1000 | (index - 15);
    return 0x1000; // TCGETS2 BOTHER; the exact speed is serialized separately.
}
Attributes attributes(int fd) {
    struct termios native{};
    check_host(::tcgetattr(fd, &native));
    Attributes result;
    result.input = from_native(native.c_iflag, input_flags);
    result.output = output_from_native(native.c_oflag);
    result.control = from_native(native.c_cflag, control_flags);
    switch (native.c_cflag & CSIZE) {
    case CS5:
        break;
    case CS6:
        result.control |= 0x10;
        break;
    case CS7:
        result.control |= 0x20;
        break;
    case CS8:
        result.control |= 0x30;
        break;
    }
    result.local = from_native(native.c_lflag, local_flags);
    result.output_speed = native_rate(::cfgetospeed(&native));
    result.input_speed = native_rate(::cfgetispeed(&native));
    result.control |= speed_bits(result.output_speed);
    if (result.input_speed && result.input_speed != result.output_speed)
        result.control |= speed_bits(result.input_speed) << 16;
    for (auto c : characters)
        result.characters[c.guest] =
            !c.count && native.c_cc[c.host] == _POSIX_VDISABLE ? 0 : native.c_cc[c.host];
    return result;
}
void set_attributes(int fd, const Attributes &value, unsigned action) {
    if (action > 2)
        guest_error(22);
    if (value.line)
        guest_error(22); // This runtime exposes the native N_TTY-equivalent discipline.
    struct termios native{};
    native.c_iflag = to_native(value.input, input_flags);
    native.c_oflag = output_to_native(value.output);
    native.c_cflag = to_native(value.control & ~(baud_mask | (baud_mask << 16) | 0x30u), control_flags);
    constexpr tcflag_t sizes[] = {CS5, CS6, CS7, CS8};
    native.c_cflag |= sizes[(value.control >> 4) & 3];
    native.c_lflag = to_native(value.local, local_flags);
    std::fill(std::begin(native.c_cc), std::end(native.c_cc), cc_t(_POSIX_VDISABLE));
    for (auto c : characters) {
        uint8_t byte = value.characters[c.guest];
        if (!c.count && byte && byte == uint8_t(_POSIX_VDISABLE))
            guest_error(22);
        native.c_cc[c.host] = !c.count && !byte ? cc_t(_POSIX_VDISABLE) : byte;
    }
    const uint32_t output_bits = value.control & baud_mask;
    const uint32_t input_bits = (value.control >> 16) & baud_mask;
    const uint32_t output = output_bits == 0x1000 ? value.output_speed : speed_from_bits(output_bits);
    const uint32_t input = !input_bits            ? output
                           : input_bits == 0x1000 ? value.input_speed
                                                  : speed_from_bits(input_bits);
    check_host(::cfsetospeed(&native, native_speed(output)));
    check_host(::cfsetispeed(&native, native_speed(input)));
    constexpr int actions[] = {TCSANOW, TCSADRAIN, TCSAFLUSH};
    check_host(::tcsetattr(fd, actions[action], &native));
}
int output_queued(int fd) {
    int count = 0;
    check_host(::ioctl(fd, TIOCOUTQ, &count));
    return count;
}
int input_queued(int fd) {
    int count = 0;
    check_host(::ioctl(fd, FIONREAD, &count));
    return count;
}
int modem_bits(int fd) {
    int value = 0;
    check_host(::ioctl(fd, TIOCMGET, &value));
    return int(from_native(tcflag_t(value), modem_flags));
}
void change_modem_bits(int fd, uint32_t value, unsigned operation) {
    if (operation > 2)
        guest_error(22);
    int native = int(to_native(value, modem_flags));
    const unsigned long request = operation == 0 ? TIOCMSET : operation == 1 ? TIOCMBIS : TIOCMBIC;
    check_host(::ioctl(fd, request, &native));
}
void flow(int fd, unsigned action) {
    if (action > 3)
        guest_error(22);
    constexpr int actions[] = {TCOOFF, TCOON, TCIOFF, TCION};
    check_host(::tcflow(fd, actions[action]));
}
void flush(int fd, unsigned action) {
    if (action > 2)
        guest_error(22);
    constexpr int actions[] = {TCIFLUSH, TCOFLUSH, TCIOFLUSH};
    check_host(::tcflush(fd, actions[action]));
}
void exclusive(int fd, bool enable) {
    check_host(::ioctl(fd, enable ? TIOCEXCL : TIOCNXCL));
}
void packet_mode(int fd, bool enable) {
    int value = enable;
    check_host(::ioctl(fd, TIOCPKT, &value));
}
void line_discipline(int fd, unsigned discipline) {
    if (discipline != 0)
        guest_error(22);
#if defined(__APPLE__)
    int native = TTYDISC;
#else
    int native = 0;
#endif
    check_host(::ioctl(fd, TIOCSETD, &native));
}
unsigned line_discipline(int fd) {
    int native = 0;
    check_host(::ioctl(fd, TIOCGETD, &native));
#if defined(__APPLE__)
    if (native != TTYDISC)
#else
    if (native != 0)
#endif
        guest_error(25);
    return 0;
}
uint8_t packet_flags(uint8_t value) {
    // Darwin and Linux intentionally use distinct symbols even where values coincide.
    uint8_t result = 0;
    if (value & TIOCPKT_FLUSHREAD)
        result |= 1;
    if (value & TIOCPKT_FLUSHWRITE)
        result |= 2;
    if (value & TIOCPKT_STOP)
        result |= 4;
    if (value & TIOCPKT_START)
        result |= 8;
    if (value & TIOCPKT_NOSTOP)
        result |= 16;
    if (value & TIOCPKT_DOSTOP)
        result |= 32;
#ifdef TIOCPKT_IOCTL
    if (value & TIOCPKT_IOCTL)
        result |= 64;
#endif
    return result;
}
} // namespace operit::arm64::terminal
