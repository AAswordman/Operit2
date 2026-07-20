#define _POSIX_C_SOURCE 200809L

#include <errno.h>
#include <fcntl.h>
#include <pty.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/select.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <termios.h>
#include <unistd.h>

enum {
    control_buffer_capacity = 128,
    io_buffer_capacity = 4096,
};

static const unsigned char escape_byte = 0x1b;
static const unsigned char bell_byte = 0x07;
static const char resize_prefix[] = "\033]1337;OPERIT_RESIZE;";
static const char ready_marker[] = "\033]1337;OPERIT_TERMINAL_READY\007";

/** Writes every byte to one file descriptor or terminates on a transport failure. */
static void write_all(int file_descriptor, const unsigned char *bytes, size_t length) {
    size_t offset = 0;

    while (offset < length) {
        ssize_t written = write(file_descriptor, bytes + offset, length - offset);
        if (written < 0) {
            if (errno == EINTR) {
                continue;
            }
            perror("operit terminal write");
            exit(1);
        }
        offset += (size_t)written;
    }
}

/** Parses one positive decimal terminal dimension. */
static int parse_dimension(const char *value) {
    char *end = NULL;
    long parsed = strtol(value, &end, 10);

    if (value[0] == '\0' || *end != '\0' || parsed < 1 || parsed > 1000) {
        fprintf(stderr, "operit terminal requires a positive terminal dimension\n");
        exit(2);
    }
    return (int)parsed;
}

/** Applies one terminal window size to the PTY master. */
static void set_terminal_size(int pty_master, int rows, int columns) {
    struct winsize size = {
        .ws_row = (unsigned short)rows,
        .ws_col = (unsigned short)columns,
        .ws_xpixel = 0,
        .ws_ypixel = 0,
    };

    if (ioctl(pty_master, TIOCSWINSZ, &size) < 0) {
        perror("operit terminal resize");
        exit(1);
    }
}

/** Configures the serial transport to forward input without locally echoing it. */
static void configure_serial_transport(void) {
    struct termios settings;

    if (tcgetattr(STDIN_FILENO, &settings) < 0) {
        perror("operit terminal serial attributes");
        exit(1);
    }
    settings.c_lflag &= ~(ECHO | ICANON | ISIG | IEXTEN);
    settings.c_cc[VMIN] = 1;
    settings.c_cc[VTIME] = 0;
    if (tcsetattr(STDIN_FILENO, TCSANOW, &settings) < 0) {
        perror("operit terminal serial configuration");
        exit(1);
    }
}

/** Starts the interactive shell with the PTY slave as its controlling terminal. */
static pid_t start_shell(int pty_slave) {
    pid_t child = fork();

    if (child < 0) {
        perror("operit terminal fork");
        exit(1);
    }
    if (child == 0) {
        if (setsid() < 0 || ioctl(pty_slave, TIOCSCTTY, 0) < 0) {
            perror("operit terminal session");
            _exit(1);
        }
        if (dup2(pty_slave, STDIN_FILENO) < 0 ||
            dup2(pty_slave, STDOUT_FILENO) < 0 ||
            dup2(pty_slave, STDERR_FILENO) < 0) {
            perror("operit terminal redirect");
            _exit(1);
        }
        if (pty_slave > STDERR_FILENO) {
            close(pty_slave);
        }
        execl("/bin/sh", "sh", "-i", (char *)NULL);
        perror("operit terminal shell");
        _exit(127);
    }
    return child;
}

/** Applies one browser resize control sequence and reports whether it was consumed. */
static bool apply_resize_control(unsigned char *control, size_t length, int pty_master) {
    const size_t prefix_length = sizeof(resize_prefix) - 1;
    char *column_separator;
    int rows;
    int columns;

    if (length <= prefix_length + 2 ||
        memcmp(control, resize_prefix, prefix_length) != 0 ||
        control[length - 1] != bell_byte) {
        return false;
    }
    control[length - 1] = '\0';
    column_separator = strchr((char *)control + prefix_length, ';');
    if (column_separator == NULL) {
        return false;
    }
    *column_separator = '\0';
    rows = parse_dimension((char *)control + prefix_length);
    columns = parse_dimension(column_separator + 1);
    set_terminal_size(pty_master, rows, columns);
    return true;
}

/** Bridges one serial input byte into the shell PTY or handles an in-band resize request. */
static void forward_input_byte(
    unsigned char byte,
    unsigned char *control,
    size_t *control_length,
    int pty_master
) {
    if (*control_length == 0) {
        if (byte == escape_byte) {
            control[(*control_length)++] = byte;
            return;
        }
        write_all(pty_master, &byte, 1);
        return;
    }

    control[(*control_length)++] = byte;
    if (*control_length == 2 && control[1] != ']') {
        write_all(pty_master, control, *control_length);
        *control_length = 0;
        return;
    }
    if (byte == bell_byte) {
        if (!apply_resize_control(control, *control_length, pty_master)) {
            write_all(pty_master, control, *control_length);
        }
        *control_length = 0;
        return;
    }
    if (*control_length == control_buffer_capacity) {
        write_all(pty_master, control, *control_length);
        *control_length = 0;
    }
}

/** Returns the exit status of the shell process. */
static int shell_exit_status(pid_t shell) {
    int status;

    if (waitpid(shell, &status, 0) < 0) {
        perror("operit terminal wait");
        return 1;
    }
    if (WIFEXITED(status)) {
        return WEXITSTATUS(status);
    }
    if (WIFSIGNALED(status)) {
        return 128 + WTERMSIG(status);
    }
    return 1;
}

/** Runs the serial-to-PTY relay for one interactive Linux terminal. */
int main(int argument_count, char **arguments) {
    unsigned char control[control_buffer_capacity];
    unsigned char io_buffer[io_buffer_capacity];
    size_t control_length = 0;
    int pty_master;
    int pty_slave;
    int rows;
    int columns;
    pid_t shell;

    if (argument_count != 3) {
        fprintf(stderr, "usage: operit-runtime-terminal ROWS COLUMNS\n");
        return 2;
    }
    rows = parse_dimension(arguments[1]);
    columns = parse_dimension(arguments[2]);
    configure_serial_transport();
    if (openpty(&pty_master, &pty_slave, NULL, NULL, NULL) < 0) {
        perror("operit terminal openpty");
        return 1;
    }
    set_terminal_size(pty_master, rows, columns);
    shell = start_shell(pty_slave);
    close(pty_slave);
    write_all(STDOUT_FILENO, (const unsigned char *)ready_marker, sizeof(ready_marker) - 1);

    for (;;) {
        fd_set read_set;
        int maximum_file_descriptor = pty_master > STDIN_FILENO ? pty_master : STDIN_FILENO;
        int ready;

        FD_ZERO(&read_set);
        FD_SET(STDIN_FILENO, &read_set);
        FD_SET(pty_master, &read_set);
        ready = select(maximum_file_descriptor + 1, &read_set, NULL, NULL, NULL);
        if (ready < 0) {
            if (errno == EINTR) {
                continue;
            }
            perror("operit terminal select");
            return shell_exit_status(shell);
        }
        if (FD_ISSET(STDIN_FILENO, &read_set)) {
            ssize_t read_length = read(STDIN_FILENO, io_buffer, sizeof(io_buffer));
            if (read_length <= 0) {
                return shell_exit_status(shell);
            }
            for (ssize_t index = 0; index < read_length; ++index) {
                forward_input_byte(io_buffer[index], control, &control_length, pty_master);
            }
        }
        if (FD_ISSET(pty_master, &read_set)) {
            ssize_t read_length = read(pty_master, io_buffer, sizeof(io_buffer));
            if (read_length <= 0) {
                return shell_exit_status(shell);
            }
            write_all(STDOUT_FILENO, io_buffer, (size_t)read_length);
        }
    }
}
