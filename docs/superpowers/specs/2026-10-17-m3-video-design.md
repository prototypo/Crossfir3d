# M3 "Video settings and light" — Design

Date: 2026-10-17
Status: draft, waiting for owner approval (reviewer questions answered)
Builds on: M1 and M2 designs in this folder (both merged).

## Intent

The owner plays on modern Mac screens and external monitors and wants the game to
fit them: a window that resizes and maximises, sharp but fast drawing on big
screens, and a choice between smoothness and battery. After playing M2 the owner
also said: "the walls look flat and the shadows are too dark". A lighting test
(three screenshots) showed that more background light, soft corner shading, and a
lower sun make walls look solid. The owner then asked for the sun to follow
Crossfire's own clock.

## Goals

1. **Window** (owner: "in a window that can be maximised").
   - Normal resizable window with standard macOS behaviour: double-clicking the
     title bar (or Option-clicking the green button) maximises it; the green
     button itself enters macOS full screen. No custom window code (Claude's call).
   - A "Full screen" setting (borderless) for those who prefer it.
   - The game remembers window size, position, and monitor between runs, and the
     full-screen setting when it was set in the settings screen (Claude's call;
     the green-button full screen is macOS's own and is not saved).
2. **Render scale.** Draw the 3D view at 50%, 75%, or 100% of the window's real
   pixels, scaled up to fit. Default 100% (Claude's call: the M1–M2 scene is
   light; the owner can lower it on a 5K monitor). The HUD and menus always draw
   at full sharpness.
3. **Frame rate** (owner: "smooth movement, but let me choose").
   - "Match screen" (vsync, follows the display, up to 120 Hz on ProMotion) —
     default.
   - Caps of 30, 60, or 120 frames per second to save battery.
   - "Unlimited" (no vsync) for testing.
4. **Sun follows the game clock** (owner: yes).
   - Crossfire's day: 28 hours, 1500 server ticks per hour (`include/tod.h`),
     6 periods: Night, Dawn, Morning, Noon, Evening, Dusk.
   - The server does not send the time. At login the client sends the `time`
     command once, reads the hour from the reply ("It is … o'clock …"), and from
     then on advances the clock with the server's `tick` messages. The `time`
     reply is not shown in the parchment box or log (Claude's call: it is the
     client's own question).
   - The hour sets only the **direction** of the light: low in the east at dawn,
     high at noon, low in the west at dusk. At night (sun below the horizon) the
     same light turns into a bluish moon from a fixed high angle, without shadows.
   - The light's **strength** follows only the darkness of the player's square,
     as in M1. The server already darkens outdoor maps by hour and season
     (`server/weather.cpp`) and keeps dungeons dark, so this needs no
     indoor/outdoor test and never cuts light twice. Dark nights stay as dark as
     the server makes them. (Claude's call, from the server source.)
   - The sun's path uses the fixed hours of the 6 periods, not the season table;
     only the angle can drift a little from the server's darkness by season.
   - The server's per-square darkness still dims each square, as now.
5. **Lighter shadows.** Raise the soft background (ambient) light so shadowed
   areas are not almost black. The lighting test showed 4× today's value (600 vs
   150) looks right; exact values are the builder's, checked by screenshot.
6. **Ambient occlusion** (soft shading in corners and where walls meet floors):
   off / low / medium / high. Default medium (Claude's call). It needs
   anti-aliasing other than MSAA (see 8).
7. **Shadow quality:** off / low / medium / high (shadow map size; Bevy cascade
   settings). Default medium.
8. **Anti-aliasing:** off / MSAA 4× / TAA. If ambient occlusion is on, MSAA is not
   allowed (Bevy limit); the menu then offers off / TAA only and switches MSAA to
   TAA (Claude's call).
9. **Brightness, contrast, gamma:** sliders with sensible ranges (builder's
   choice), default neutral.
10. **Zoom limits** stay as now (mouse wheel). No field-of-view setting
    (Claude's call: not asked for; can come later).
11. **Settings screen.** In game, Escape opens it when nothing else is open (no
    parchment box, no command box, no inventory); Escape again closes it. The
    login form also gets a "Settings" button, so window and frame-rate choices
    can be made before logging in (Claude's call). It covers the
    screen like the inventory, in the same style. Changes apply at once. A
    "Reset to defaults" button. (Claude's calls.)
12. **Saved settings.** One small text file, read at start, written on every
    change. Path `$HOME/Library/Application Support/MacCrossfire/settings.txt`
    (same folder as the quick-slot file). A missing or broken file means
    defaults; a broken line is skipped, never a crash.

## Non-goals

- No key rebinding, audio volume sliders, or UI scale (later).
- No new art, models, or textures (M4).
- No change to the server or protocol beyond sending `time` once.
- No per-monitor profiles; one set of settings.

## Anti-goals

- A setting that crashes or blanks the screen (for example MSAA + ambient
  occlusion in Bevy) — the menu must never offer a broken combination.
- The settings screen taking keys while closed, or the game taking keys while it
  is open (the M2 key-mode rules apply: a new `Mode::Settings`).
- Losing the window on start because a saved monitor is unplugged: fall back to
  the main screen.

## Constraints

Unchanged from M1/M2: GPLv2+, Rust + Bevy 0.20 + bevy_egui, Mac first, works on
Crossfire 1.75.0+. The `time` reply format must be checked against
`common/time.cpp` (`time_format_time`) in the pinned server and 1.75.0; if the
reply cannot be parsed, the sun stays at a fixed "morning" angle.

## Testing

- Unit tests: settings file read/write (defaults, bad lines, round trip); the
  allowed-combination rule (ambient occlusion forbids MSAA); clock parsing from a
  real `time` reply and advancing by ticks; sun angle and strength per hour
  (night = no sun, noon = highest); key-mode rules for Escape and the settings
  mode.
- Screenshots: each lighting setting at noon and dusk; render scale 50%; the
  settings screen.
- By hand (owner): maximise on the external monitor, restart and see the window
  come back the same; frame-rate choices feel right.

## Left to the builder

- Exact light values, sun path curve, moon colour.
- Settings screen layout and widget types.
- How the frame cap is done (a small sleep in the frame loop is fine).
- Settings file format details (plain `key=value` lines suggested).
