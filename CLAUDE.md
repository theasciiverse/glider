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
  (title screen), 1000 to 1007 (help pages), FONT 2060 (family 16, 12 pt); `snd ` 1001 to 1030 = the game's sounds 1 to 30.
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

- **Hands-off House** (Ward Hartenstein's house that plays itself) does not play
  through, and the study of 2026-10-27 says the port is not why. Findings:
  - Every glider lost is at a point where a timer counted in Mac ticks meets the
    glider's motion counted in frames: the toaster's wait between launches (room 7,
    `extra` = 30 ticks), the outlets' spark cycles (room 13), and the power switches'
    90-tick lockout (room 21, "Switch twitch.": the design needs the glider back up
    past a switch within 90 ticks of passing it going down, so the switch ignores it).
  - Sweeping ticks per frame (`frame_ticks`, fractional allowed): room 7 passes only
    from 2.4 to 3.0, room 21 only at about 2.0 (2.1 to 2.3 fail it), and room 13 fails
    at every speed tried, for every ReadyRoom drawing cost from 0 to 60 ticks. No
    single machine speed plays the house. A frame cost growing with the number of
    moving things drawn (2 + k per mover, k 0 to 0.5) did not fit either; dropped
    rather than fitting more parameters.
  - Rooms without such timers play through at every speed, and the house's own Read
    Me says room 13 "occasionally" fried the glider on the original too.
  - So the house depends on how long each frame took on Ward's LC II, which varied
    with what was drawn. Glider here keeps the game's own timing: 2 ticks a frame,
    as on any Mac fast enough.
  - The definitive check, not done: run Glider 4.05 itself in a Mac emulator (Mini
    vMac or Basilisk II) and compare frame by frame. The repo has the source and the
    THINK Pascal project, not a built application.
  - Tools: `frame_ticks` and `ready_room_ticks` on `Game` (defaults 2.0 and 0, the
    true game), and the ignored tests `sweep_ready_room_ticks` (FT, RR),
    `hands_off_deaths_by_room` and `trace_hands_off_room` (ROOM, FT).
  `DISSOLVE_TICKS` = 12 is an estimate.
- Not done: saved games, help screen idle demos (STR# 131), air currents visible
  (ppat 128/129), relative mouse, B&W art, the Room Editor.
- System font (Chicago) text is drawn with family 16 instead: Chicago is not in
  Glider.r.

## Views (2026-10-27)

Two looks, V switches, saved in glider.cfg (`--silhouette`, `--original`):

- **Original**: Calhoun's art in half blocks, each terminal pixel the average of
  the Mac pixels under it; the headline bar drawn as on the Mac.
- **Silhouette** (default), asked for by David: the room's background (with stairs
  and paintings) blurred and dimmed (`Canvas::soften`), so the dithered wallpaper
  stops being noise; tables, shelves, cabinets, windows and mirrors flat, with their
  edges in box-drawing characters over the picture (heavy bars and posts, light
  outlines, half lines only where a bar or post ends mid-cell; never over anything
  that moves); everything that moves or hurts (glider, darts, 'copters, balloons,
  band, flames, sparks, toast, fish, ball) a solid silhouette in a colour by role
  (`draw::ink`); the house's own objects (books, clocks, batteries, vents, switches)
  keep their art, since flattened they stop saying what they are (tried first: books
  became a red slab, the clock a yellow square). Drawn in Braille cells (AQuest's
  Mixed method, brightest pixel per dot); the headline's figures go to the status
  line as text.

In both views, text that appears during play is terminal text over the picture, not
the Mac's 12-point letters, which are a smudge at terminal sizes (David, 2026-10-27):
the time bonus box, the points floating up from a prize, the final score on the
GAME OVER screen (`draw::Label`, `Show::label`, `view::draw_labels`). The GAME
OVER letters stay art. The Original view's headline bar is still drawn as on the Mac.

## Glider PRO (separate title, not started)

David wants a separate port of Glider PRO via elasota/Aerofoil too. Name: David
suggested Aerofoil; recommended instead (2026-10-27, not decided): "Glider PRO", the
original's own name, as "Glider" here, crediting Aerofoil as the source, since
Aerofoil is Eric Lasota's name for his own maintained port. Who holds the "Glider
PRO" trademark was not found. Research
2026-10-27: Glider PRO is GPL-2.0 (Calhoun's repo, code and data; Aerofoil says
"or later"); C, 46k lines; Aerofoil C++ `GpApp/` about 33.6k lines of gameplay;
houses BinHex, room 512x322, flat big-endian data fork (866 + 348 x rooms). No
terminal port found.

## Commands

```bash
cargo run --release                      # title screen, The House
cargo run --release -- "Mad House"       # a built-in house; --houses lists them
cargo run --release -- --watch "Hands-off House"
cargo run --release -- --shot 30 --room 3 --size 128x44   # a frame as ANSI, no terminal
RUST_LOG=debug cargo run --release 2>glider.log
cargo test --release
```
