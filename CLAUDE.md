# Glider

Asciiverse port of **Glider 4.0** (John Calhoun, Casady & Greene, 1991), the paper
glider flying through the rooms of a house. Rust + Ratatui. Started 2026-10-27.

## Source and licence (checked 2026-10-27)

- github.com/softdorothy/Glider4 (Calhoun's own account), MIT, "Copyright (c) 2016
  softdorothy". The repo holds the THINK Pascal 3.02 source of Glider 4.05, the Color
  and B&W art (MacBinary), `Glider.r` and `snd.r` (DeRez text), the Room Editor 1.03
  source and 22 house files. The licence covers the whole repo, art and houses
  included. Several houses are by other authors (Ward Hartenstein's Hands-off House
  and Glass House, others named in their Read Me files); Calhoun published them in
  the MIT repo.
- That repo is this one's `upstream` remote; our history continues on top of his
  (branch `main`). No `origin` yet: the theasciiverse repo has not been created.
- The `.p` files use classic Mac CR line endings, so `wc -l` says 0. Read them with
  `tr '\r' '\n'`.
- No terminal port existed (searched GitHub and the web 2026-10-27). Known ports: a
  1994 Windows version, a 2020 browser version, and Aerofoil (which is Glider PRO, a
  different, later game; see "Glider PRO" below).

## How it is built

The game reads Calhoun's files directly, at start-up, nothing extracted:

- `src/mac.rs`: MacBinary, resource forks, DeRez text, QuickDraw PICT 1 and 2
  (bit and pixel maps, PackBits, opcode 0x99), `snd ` (format 1 and 2), bitmap
  `FONT`s, MacRoman.
- `src/assets.rs`: Color Art PICT 128 (sprites), 129 (masks, incl. the dithered fade
  masks), 130 (farm), 198/199 (stairs), 200 to 209 (backgrounds); Glider.r PICT 210
  (title screen), 1000 to 1007 (help pages), FONT 2060 (family 16, 12 pt) and 26144
  (family 204, 32 pt); `snd ` 1001 to 1030 = the game's sounds 1 to 30.
- `src/house.rs`: the house format (`houseRec`, 13830 bytes: 1270-byte header, 40
  rooms of 314; Mac `Rect`s stored top, left, bottom, right). Built-in houses are
  `include_bytes!`; any Glider 4 house file can be played from disk
  (`glider path/to/house`), its next files found beside it.
  `pictFile` in a house is never read by the game: every house plays with Color Art.
- `src/game.rs`: the play loop translated from G-PlayActive.p, G-PlayUtils.p and
  G-PlaySetUp.p, with the Pascal's names. Time is the Mac's TickCount (60/s), a
  frame is 2 ticks (Coordinate's `repeat until TickCount >= timeIs + 2`). Quirks
  kept on purpose: windows, mirrors and paintings decrement `nObjects`, so objects
  after them are not collision-checked; Pascal's `for` bound is taken once.
- `src/gfx.rs`: a 512x342 canvas and the QuickDraw calls Glider uses (pen-sized
  lines, 50% gray pattern ovals and traced regions, CopyMask, stretched CopyBits,
  InvertRect, DrawString with the FONTs).
- `src/draw.rs`: DrawAllObjects (tables, shelves, cabinets, windows and mirrors are
  drawn with QuickDraw from the room's rectangles, not art) and DrawCScene's order
  per frame; the headline bar is redrawn from state each frame.
- `src/fx.rs`: transitions drawn straight to the screen in the Pascal: side scrolls
  (32 strips, 1 tick each), up/down (19), DissBlocks (10-bit LFSR, taps $240), time
  bonus (30 ticks), ClosingAnimation, GameOver. The game adds each one's tick cost to
  its clock so timers (outlets, toasters, darts) run on through them as on a Mac.
- `src/view.rs`: half blocks, square Mac pixels, each terminal pixel the average of
  the Mac pixels under it.
- `src/input.rs`, `src/audio.rs` (4 effect voices + music + bass, 22254 Hz),
  `src/save.rs` (`$XDG_DATA_HOME/glider`: scores.txt per house, glider.cfg).

Controls: keyboard hold (kitty protocol) or tap (Glider's own dropKey method, used
when the terminal sends no key releases), switched by pressing a steering key;
moving the mouse over the room switches to Glider's absolute mouse control
(the original default). "Ozma" typed on the title screen: 50 gliders, as in Glider.

## Known gaps (2026-10-27)

- **Hands-off House** (plays itself, "never crashes") does not play through yet.
  At 2 ticks a frame the first glider is lost in room 7 to the toaster (the toaster
  waits `extra` ticks between launches while the toast flies in frames); at 3 ticks
  a frame it gets to room 13 ("Shockproof!", outlet timing, the very room its Read
  Me says was fixed by changing an outlet delay from 100 to 110). The house was
  built on an LC II with an accelerator, likely slower than 2 ticks a frame. Not yet
  known whether this is the house's machine dependence or a port error.
  `cargo test -- --ignored sweep_ready_room_ticks` and `trace_hands_off_room` are the
  tools. `READY_ROOM_TICKS` (time the Mac spent drawing a new room after the timers
  were set) is 0 for now; the sweep showed it does not matter for rooms 7 and 13.
  `DISSOLVE_TICKS` = 12 is an estimate.
- Not done: saved games, help screen idle demos (STR# 131), air currents visible
  (ppat 128/129), relative mouse, B&W art, the Room Editor.
- System font (Chicago) text is drawn with family 16 instead: Chicago is not in
  Glider.r.

## Graphics direction (open, David 2026-10-27)

David asked about box-drawing ("pipe") and half-pipe characters mixed with Braille,
and a silhouette style. Not decided; see the conversation's proposal: geometry-drawn
furniture as box-drawing, sprites in Braille, as a second view to compare.

## Glider PRO (separate title, not started)

David wants a separate port of Glider PRO via elasota/Aerofoil too. Research
2026-10-27: Glider PRO is GPL-2.0 (Calhoun's repo, code and data; Aerofoil says
"or later"); C, 46k lines; Aerofoil C++ `GpApp/` about 33.6k lines of gameplay;
houses BinHex, room 512x322, flat big-endian data fork (866 + 348 x rooms). No
terminal port found.

## Commands

```bash
cargo run --release                      # title screen, The House
cargo run --release -- "Mad House"       # a built-in house; --houses lists them
cargo run --release -- --watch "Hands-off House"
cargo run --release -- --shot 60 --size 128x44   # a frame as ANSI, no terminal
RUST_LOG=debug cargo run --release 2>glider.log
cargo test --release
```
