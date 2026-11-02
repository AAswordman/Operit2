#include "linux/error_mapping.h"
#include "base/support.h"
#include <cerrno>

namespace operit::arm64 {
/* Host errno values differ from asm-generic Linux, especially on Darwin. */
int linux_errno(int error) {
    switch (error) {
    case 0:
        return 0;
    case ENOTSOCK:
        return 88;
    case EDESTADDRREQ:
        return 89;
    case EMSGSIZE:
        return 90;
    case EPROTOTYPE:
        return 91;
    case ENOPROTOOPT:
        return 92;
    case EPROTONOSUPPORT:
        return 93;
    case ESOCKTNOSUPPORT:
        return 94;
    case EOPNOTSUPP:
        return 95;
#if ENOTSUP != EOPNOTSUPP
    case ENOTSUP:
        return 95;
#endif
    case EPFNOSUPPORT:
        return 96;
    case ESHUTDOWN:
        return 108;
    case ETOOMANYREFS:
        return 109;
    case EHOSTDOWN:
        return 112;
    case ECANCELED:
        return 125;
    case EAFNOSUPPORT:
        return 97;
    case EADDRINUSE:
        return 98;
    case EADDRNOTAVAIL:
        return 99;
    case ENETDOWN:
        return 100;
    case ENETUNREACH:
        return 101;
    case ENETRESET:
        return 102;
    case ECONNABORTED:
        return 103;
    case ECONNRESET:
        return 104;
    case ENOBUFS:
        return 105;
    case EISCONN:
        return 106;
    case ENOTCONN:
        return 107;
    case ETIMEDOUT:
        return 110;
    case ECONNREFUSED:
        return 111;
    case EHOSTUNREACH:
        return 113;
    case EALREADY:
        return 114;
    case EINPROGRESS:
        return 115;
    case EPERM:
        return 1;
    case ENOENT:
        return 2;
    case ESRCH:
        return 3;
    case EINTR:
        return 4;
    case EIO:
        return 5;
    case ENXIO:
        return 6;
    case E2BIG:
        return 7;
    case ENOEXEC:
        return 8;
    case EBADF:
        return 9;
    case ECHILD:
        return 10;
    case EAGAIN:
        return 11;
    case ENOMEM:
        return 12;
    case EACCES:
        return 13;
    case EFAULT:
        return 14;
    case EBUSY:
        return 16;
    case EEXIST:
        return 17;
    case EXDEV:
        return 18;
    case ENODEV:
        return 19;
    case ENOTDIR:
        return 20;
    case EISDIR:
        return 21;
    case EINVAL:
        return 22;
    case ENFILE:
        return 23;
    case EMFILE:
        return 24;
    case ENOTTY:
        return 25;
    case EFBIG:
        return 27;
    case ENOSPC:
        return 28;
    case ESPIPE:
        return 29;
    case EROFS:
        return 30;
    case EMLINK:
        return 31;
    case EPIPE:
        return 32;
    case EDOM:
        return 33;
    case ERANGE:
        return 34;
    case ENAMETOOLONG:
        return 36;
    case ENOSYS:
        return 38;
    case ENOTEMPTY:
        return 39;
    case ELOOP:
        return 40;
    case EOVERFLOW:
        return 75;
    default:
        return 5;
    }
}
void check_host(int result) {
    if (result < 0)
        guest_error(linux_errno(errno));
}

} // namespace operit::arm64
