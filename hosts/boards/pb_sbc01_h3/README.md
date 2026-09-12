# operit-board-pb-sbc01-h3

Board profile for PB_SBC01_H3 hardware.

This crate owns the board-specific Host API implementations used by
`apps/pb_sbc01_h3`. The first implemented surface is `RobotFaceHost`, which writes
validated expression state for a local face renderer process.

The crate does not own Operit Core startup, Access pairing, Link packets, or
application command routing.
