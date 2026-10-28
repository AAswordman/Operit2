#!/usr/bin/env python3
"""Prepare the simulator's bounded desktop discovery poller from a pinned archive."""
from pathlib import Path
import hashlib
import io
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[4]
CRATE = "polling-2.8.0"
CHECKSUM = "4b2d323e8ca7996b3e23126511a523f7e62924d93ecd5ae73b333815b0eb3dce"
DEST = ROOT / "edge/boards/esp32/editor/simulator/.embuild"
PATCHES = {
    "src/iocp/mod.rs": [("ConcurrentQueue::bounded(1024)", "ConcurrentQueue::bounded(16)", 1),
                         ("Vec::with_capacity(1024)", "Vec::with_capacity(16)", 2)],
    "src/epoll.rs": [("[libc::epoll_event; 1024]", "[libc::epoll_event; 16]", 1),
                      ("[ev; 1024]", "[ev; 16]", 1)],
    "src/kqueue.rs": [("[libc::kevent; 1024]", "[libc::kevent; 16]", 1),
                       ("[ev; 1024]", "[ev; 16]", 1)],
    "src/port.rs": [("[libc::port_event; 1024]", "[libc::port_event; 16]", 1),
                     ("[ev; 1024]", "[ev; 16]", 1)],
}


def verify():
    """Verify every bounded event allocation in the prepared source."""
    for relative, replacements in PATCHES.items():
        text = (DEST / CRATE / relative).read_text(encoding="utf-8")
        for old, new, count in replacements:
            if old in text or text.count(new) != count:
                raise RuntimeError(f"Unexpected simulator poller source: {relative}")


def prepare():
    """Extract the checksum-pinned poller and bound its desktop readiness storage."""
    marker = DEST / CRATE / "operit-bounded-events-v1"
    if marker.exists():
        verify()
        return
    with urllib.request.urlopen(f"https://static.crates.io/crates/polling/{CRATE}.crate", timeout=60) as response:
        data = response.read()
    if hashlib.sha256(data).hexdigest() != CHECKSUM:
        raise RuntimeError("Simulator poller archive checksum mismatch")
    DEST.mkdir(parents=True, exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        for member in archive.getmembers():
            target = (DEST / member.name).resolve()
            if not target.is_relative_to((DEST / CRATE).resolve()) or not (member.isfile() or member.isdir()):
                raise RuntimeError(f"Unsafe simulator poller archive entry: {member.name}")
        archive.extractall(DEST, filter="data")
    for relative, replacements in PATCHES.items():
        path = DEST / CRATE / relative
        text = path.read_text(encoding="utf-8")
        for old, new, count in replacements:
            if text.count(old) != count:
                raise RuntimeError(f"Pinned simulator poller changed: {relative}")
            text = text.replace(old, new)
        path.write_text(text, encoding="utf-8")
    verify()
    marker.write_text(CHECKSUM + "\n", encoding="utf-8")


if __name__ == "__main__":
    prepare()
