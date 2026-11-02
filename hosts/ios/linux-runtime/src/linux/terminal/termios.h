#pragma once
#include <array>
#include <cstdint>
#include <termios.h>

namespace operit::arm64::terminal {
// asm-generic Linux termios is 36 bytes; termios2 appends two 32-bit speeds.
// Memory serialization belongs to the syscall boundary, not the native ABI.
struct Attributes {
    uint32_t input = 0, output = 0, control = 0, local = 0;
    uint8_t line = 0;
    std::array<uint8_t, 19> characters{};
    uint32_t input_speed = 0, output_speed = 0;
};
Attributes attributes(int fd);
void set_attributes(int fd, const Attributes &value, unsigned action = 0);
uint32_t speed_from_bits(uint32_t bits);
uint32_t speed_bits(uint32_t speed);
int output_queued(int fd);
int input_queued(int fd);
int modem_bits(int fd);
void change_modem_bits(int fd, uint32_t bits, unsigned operation);
void flow(int fd, unsigned action);
void flush(int fd, unsigned action);
void exclusive(int fd, bool enable);
void packet_mode(int fd, bool enable);
void line_discipline(int fd, unsigned discipline);
unsigned line_discipline(int fd);
uint8_t packet_flags(uint8_t native_flags);
} // namespace operit::arm64::terminal
