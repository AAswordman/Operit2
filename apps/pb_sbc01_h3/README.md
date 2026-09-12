# PB_SBC01_H3

Operit full Core hardware runtime for the PB_SBC01_H3 Linux board.

This entry starts the full Core tree on PB_SBC01_H3, attaches board-owned
robot face capabilities through Host API, and serves the authenticated Link
endpoint for phones, desktops, and Web Access clients.

## Binaries

```text
operit-pb-sbc01-h3        full Core controller for PB_SBC01_H3
operit-pb-sbc01-h3-face   local console renderer for the robot face state file
```

## Run

```bash
cargo run --manifest-path apps/pb_sbc01_h3/Cargo.toml --bin operit-pb-sbc01-h3 -- \
  --token "replace-with-a-strong-token"
```

The PB_SBC01_H3 controller writes robot face state to:

```text
$HOME/.local/share/operit2/pb_sbc01_h3/state/robot_face_state.json
```

Start the local face renderer on the display TTY with:

```bash
cargo run --manifest-path apps/pb_sbc01_h3/Cargo.toml --bin operit-pb-sbc01-h3-face -- \
  --state-file "$HOME/.local/share/operit2/pb_sbc01_h3/state/robot_face_state.json"
```

## Boundaries

- `apps/pb_sbc01_h3` owns full Core startup and PB_SBC01_H3 process orchestration.
- `hosts/linux` owns Linux platform Host API implementations.
- `hosts/boards/pb_sbc01_h3` owns PB_SBC01_H3 board-specific Host API surfaces.
- `core/crates/node/edge` owns the typed board service dispatch boundary.
- `core/crates/proxy/edge` owns typed board service calls from PB_SBC01_H3 code.
