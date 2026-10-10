# Graphics options (port-only)

Decided 2026-10-10 with the user. Code: `rc-engine/src/graphics.rs` (the settings), `rc-game/src/menus/pause/port.rs`
(the Port Options rows), `rc-engine/src/menu_render.rs` (`sync_graphics_rows` / `read_graphics_rows`).

## The rule

Every option controls exactly one thing, and its **first value is always Original**: what the PS2 showed on a TV.
The other values are modern alternatives. A **Preset** row sets them all: Original (every option Original, the
default) or Enhanced; it shows Custom when the options match no preset (never chosen with ✕). The preset is not
stored; it is read back from the options. Options never change game state: the game logic keeps its own decisions
(LOD choice, the moby "drawn" flags, culling), the options only change what is drawn.

## Rows (in this order on the page)

| Row | Values | Original means | Status |
|---|---|---|---|
| Preset | Original / Enhanced / (Custom) | every row Original | done |
| Anti-aliasing | Original / Off / 2x / 4x / 8x | the game's own frame-end softening blit (the "AA blit", G-REN-017), scaled to the frame | rows Off..8x exist (MSAA); Original planned |
| Detail distance | Original / Far / Farther / Maximum | the game's LOD distances (×1, ×2, ×4, never lower) | planned |
| Texture filtering | Original / Smooth / Sharp | the GS mip rule: one level from depth, `round(log2(z/32)+K)`; Smooth = the same level unrounded, two levels blended; Sharp = the GPU's footprint level + anisotropic | planned |
| HUD | Original / Sharp pixels / High resolution | the 512×416 screen scaled up with bilinear filtering, as the TV and an emulator's display showed it; Sharp pixels = nearest (the port's earlier look); High resolution = drawn at the frame's resolution | Original / Sharp pixels done |
| Shadows | on / off | on | exists |
| Aspect ratio, Resolution, Fullscreen | | | exist (not part of the preset: they depend on the screen) |

Enhanced: Anti-aliasing 4x (the best of 4x / 2x the GPU offers), Detail Far, Textures Smooth, HUD High resolution
(Original until High resolution exists).

## Notes

* The HUD's per-sprite filtering is the GS's and stays exact in every mode (`hud.wgsl`); only the composite to the
  frame changes. At a 416p frame both modes are identical (1:1).
* The interlace half-line offsets (XYOFFSET) are not part of Anti-aliasing Original: on a progressive screen they would
  only show as jitter.
* Persisted keys in the port settings file: `hud`, `textures`, `detail` (values: the lower-case names). `RC_GFX_HUD`,
  `RC_GFX_TEXTURES`, `RC_GFX_DETAIL`, `RC_GFX_PRESET=original|enhanced` override at start.
