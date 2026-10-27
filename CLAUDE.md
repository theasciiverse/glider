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

Gamepad (`gilrs`, feature `gamepad`, on by default): d-pad or left stick steer, A
fires a band and confirms, B or right trigger boosts, Start pauses, Select goes
back. Confirmed working by David with a real controller, 2026-10-27. Steering
with the pad (d-pad, or the stick pushed over) always selects hold control
(`Cmd::PadSteer`), since a pad's presses and releases are exact whatever the
terminal reports; a keyboard steering key still picks hold or tap from the
terminal. Only devices with an A and a Start button count as pads
(`input::is_pad`): gilrs also lists other joysticks, and a 3Dconnexion SpaceMouse
(axes and one button) was taken for the pad and would have steered. Checked
2026-10-27 with both plugged in: the Xbox 360 controller is picked.

## Based on Glider, not a replica (David, 2026-10-27)

This is a terminal game based on Glider 4. The Pascal is translated faithfully as the
starting point, but where the original stands in the way of playing well here, it is
changed. Changes to the built-in houses are in `house.rs` (`PATCHES`), each applied
only over the value it was made for:

- **Hands-off House** (plays itself) was tuned on Ward Hartenstein's LC II, where a
  frame took longer than Glider's 2 ticks, and loses gliders here where tick-counted
  timers meet frame-counted flight (study of 2026-10-27: no single machine speed
  suits all its rooms). Three timers moved to the middle of the range that works:
  room 7 toaster wait 30 to 50 ticks (40 to 65 work), room 13 middle outlet pause
  65 to 175 (150 and up), room 17 fish wait 60 to 88 (80 to 95). It now plays to the
  end on every seed tried without losing a glider (`hands_off_house_plays_itself`).
  Tools for the next house like it: the ignored tests `search_hands_off_patch`
  (ROOM, OBJ, FIELD, FROM, TO, STEP, BASE), `trace_hands_off_room`,
  `hands_off_deaths_by_room`, `sweep_ready_room_ticks`; `Game::frame_ticks` and
  `ready_room_ticks` for slower-Mac timing (defaults 2.0 and 0).

## In the catalog (2026-10-27)

- `hub/catalog/glider.toml` and `glider.md` (store page), genre arcade.
- Preview: `asciiverse/glider.cast`, 100x30, 52 s, recorded by
  `asciiverse/record-preview.sh`: `glider --watch "Hands-off House"` (it plays
  itself), silhouette view, the original view from 20 s to 36 s; thumbnail at 12 s.
  `--watch` uses hold control (no thrust when nothing is held); with tap control the
  first recording lost a glider in seconds.
- On the website mockup (`websites/Asciiverse`) with the NEW! banner
  (`dev/mock-titles.toml`), the cast copied to `casts/glider.cast`, and the local
  Linux build packaged by `dev/build_catalog.py` into
  `dev-releases/glider-x86_64-unknown-linux-gnu.tar.gz` for av.
- No GitHub release yet: the repo `theasciiverse/glider` does not exist yet.

## Known gaps (2026-10-27)

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
- **Sharper silhouettes** (David asked for crisper ships and enemies, 2026-10-27):
  `scene` records the silhouettes alone in `Canvas::fore`, and every cell with one
  in it gets the character whose drawn shape is nearest (`view::sharp_cell`): block
  eighths and quadrants, light and heavy lines, `╱╲`, or Braille, each sampled as
  8x16 points; every pair of the cell's colours is tried. The art's dark parts
  inside a shape (the glider's and dart's lower wing, the folds) are `gfx::shade`
  of the ink, so the glider reads as folded paper again. A silhouette point lost to
  the background counts twice, or the 'copter's two-pixel blades vanish (test
  `silhouettes_keep_thin_parts_and_shades`). Braille costs a little extra, as its
  dots read as fuzz. Transitions drop `fore`, except the time bonus. Costs about
  1 ms a frame. What is behind keeps up to two of its own colours in a cell (a
  table top and the wall above it). Only Box Drawing, Block Elements and Braille are used, which
  terminals draw themselves; the Legacy Computing wedges (U+1FB3C on) would give
  smoother slopes but were not checked across terminals.
- **The clock** (David: it looked "like a ball of yarn", 2026-10-27) is drawn flat,
  like the tables (`draw::flat_clock`): its art's geometry (32x29) in plain shapes,
  yellow rim and bells, white face, two-pixel black hands and feet, recorded in
  `fore` without the silhouette flag and so drawn by the same cell matcher
  (test `the_clock_reads_as_a_clock`). Tried first and dropped: every kept-art
  object through the matcher in its own colours, without its outer black
  outline. The clock stayed a tangle (shaded rim, one-pixel hands) and the books
  lost their colours to the black lines between them.
- **Objects in their own colours** (David asked for "the computer and some other
  objects", 2026-10-27; the books stay as they are): `draw::sharp_art` lists the
  house's things drawn through the matcher in their art's colours, without the
  outer black outline (`Canvas::art`): the Mac, kettle, fish bowl, guitar, power
  switch, thermostat, vents, ceiling duct and drip. Chosen object by object from
  renders in rooms of The House. Kept in Braille: books, battery, rubber bands,
  candle (Braille's brightest dot keeps their small bright detail on a dark body:
  orange label, green lettering, brass), paper, outlet, grease, fans, basket;
  toaster, shredder and light switch came out worse in flat colour. Test
  `the_mac_is_drawn_in_its_colours`.
- **GAME OVER** (David: "needs work", 2026-10-27): the falling word's sheets of
  notebook paper are drawn plain in this view (`Show::letter`): white, purple
  letter, the top edge as a gray line, no rules, margin, holes or black sides;
  recorded in `fore`, so the letters are crisp. As each new row starts, the
  last row's letters become paper, so only the falling word carries letters and
  the pile above it reads as a stack of pages (on the Mac it is a smear of letter
  crumbs). The animation and timing are Calhoun's. Test
  `game_over_letters_are_sharp`. The Original view's GAME OVER is unchanged.

In both views, text that appears during play is terminal text over the picture, not
the Mac's 12-point letters, which are a smudge at terminal sizes (David, 2026-10-27):
the time bonus box, the points floating up from a prize, the final score on the
GAME OVER screen (`draw::Label`, `Show::label`, `view::draw_labels`). The GAME
OVER letters stay art (simplified in the silhouette view, above). The Original view's headline bar is still drawn as on the Mac.

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
