# S6 Brand Icon Brief — dsh-desk

> 执行者：cursor-agent。请完整执行本文件的全部任务，不要扩 scope。

## Product context

**dsh-desk** is a Tauri 2 Windows desktop shell that spawns `dsh web` (the DeepSeek
Harness WebGUI) as a child process and presents it in a WebView2 window. The app is
tray-resident: the tray icon is the primary surface users interact with, so the mark
MUST stay legible at **16×16 px**. Secondary surfaces: taskbar, Start menu, installer.

## Task

1. **Design the source icon** and save it to `design/app-icon.svg`.
   - 1024×1024, square canvas, transparent background (no full-bleed fill to the edges
     unless the tile shape itself is the design; keep ≥ 4% safe padding).
   - **Vector only**: pure SVG paths/shapes. NO `<text>` elements, NO embedded rasters,
     NO external fonts or references — convert any letterforms to outlined paths yourself.
   - Must render correctly in `resvg` (used by Tauri CLI) — stick to standard SVG 1.1
     features: path, circle, rect, linearGradient/radialGradient, clipPath, mask.

2. **Design direction** (creative latitude within these rails):
   - Monogram **"dsh"** (lowercase, bold, geometric, hand-outlined as paths) as the hero,
     optionally combined with a subtle terminal/command motif (e.g. a chevron `›`, or a
     small window chrome hint). A pure abstract mark is acceptable if it reads as
     "developer tool / harness" at small sizes.
   - Windows 11 Fluent aesthetic: rounded-square tile or free-floating glyph.
   - Color: DeepSeek brand blue **#4D6BFE** as primary (a subtle vertical gradient is
     welcome), glyph in white / near-white. Strong contrast, no thin strokes
     (≥ 56/1024 stroke weight at icon scale), no fine detail that dies at 16 px.
   - Test yourself: mentally downscale to 16/32/48 px. If in doubt, simplify.

3. **Generate the full Tauri icon set**:
   ```
   pnpm tauri icon design/app-icon.svg -o src-tauri/icons
   ```
   This overwrites the default-template icons in `src-tauri/icons/` (that is the goal —
   spec item S6). Tauri CLI accepts SVG input directly and bundles `resvg`.

4. **Verify** (report only, no edits):
   - Read `src-tauri/tauri.conf.json` and confirm every `icon` path it references now
     exists among the generated files.
   - List the generated files (names + sizes).
   - Confirm `icon.ico` contains multiple sizes and `icon.icns` was produced.

## Hard constraints

- Touch ONLY `design/**` and `src-tauri/icons/**`. Do NOT modify any other file
  (no lib.rs, no tauri.conf.json, no package.json, no README).
- Do NOT run `pnpm install`, `pnpm tauri build`, `pnpm tauri dev`, or any git command
  that changes repository state.
- Do NOT add dependencies.
- If `pnpm tauri icon` fails on the SVG, fix the SVG (simplify features) and retry —
  do not switch to a raster pipeline.

## Revision R1 — letterform fix

The v1 artwork has a critical legibility defect: the monogram reads as **"D21"**, not "dsh".

- The lowercase **s** is drawn as an S-curve that visually reads as the digit **2**.
- The lowercase **h** lost its arch and second leg; it reads as the digit **1**.

Fix, keeping everything else (DeepSeek-blue gradient tile, rounded square, terminal
chevron accent, safe margins, 1024 canvas, pure vector paths):

1. Redraw the three letterforms as true lowercase `d s h`:
   - `d`: bowl + ascender, ascender clearly the tallest stroke of the word.
   - `s`: proper double-curve with OPEN terminals at both ends (top-left and
     bottom-right openings) — it must not resemble the digit 2.
   - `h`: full ascender + shoulder arch + second leg.
2. If you judge three letters cannot stay legible at 32 px, fall back to a single
   bold `d` monogram + the terminal chevron (an honest single letter beats an
   ambiguous wordmark).
3. Self-verify geometry in SVG comments: bounding box per letter; ascender ≥ 1.35 ×
   x-height; explicit note of the s terminals' openness.
4. Regenerate the full icon set (`pnpm tauri icon design/app-icon.svg -o src-tauri/icons`)
   and report per the original brief. Constraints from the original brief still apply.

## Revision R2 — font-derived letterforms (MANDATORY method change)

Human visual review of R1 found the hand-drawn letterforms still fail:

- The `d` reads as **b** (ascender is on the LEFT of the bowl; a lowercase d has the
  bowl on the LEFT and the ascender on the RIGHT).
- The `h` reads as **P** (the arch collapses).
- The chevron overlaps the h's leg, creating a confusing notch.

Root cause: authoring letterform bezier paths blind (without seeing the render) does
not converge. **Stop hand-drawing glyphs entirely.** Use a real font's outlines instead:

1. Pick a bold sans font present on this machine — check in order:
   `C:\Windows\Fonts\segoeuib.ttf` (Segoe UI Bold), then `C:\Windows\Fonts\arialbd.ttf`
   (Arial Bold). Use the first that exists.
2. In a SCRATCH directory outside the repo (e.g. `%TEMP%\dsh-icon-tools`), run
   `npm install opentype.js` and write a small Node script that:
   - loads the font file,
   - `font.getPath("dsh", x, y, fontSize)` → `path.toPathData(2)` → one `d` attribute,
   - measures the path bbox (`path.getBoundingBox()`) and prints it.
   Do NOT modify the repo's package.json or add repo dependencies. Delete the scratch
   dir afterwards is NOT required.
3. Compose `design/app-icon.svg`:
   - Keep the existing tile: `<rect x=72 y=72 width=880 height=880 rx=200>` with the
     existing blue gradient.
   - Replace ALL three hand-drawn letter paths with ONE path from the font data,
     transformed so that:
     - the word width ≈ 700–760 px, horizontally centered (word center x ≈ 512),
     - vertically centered by its visual mass: glyph bbox center y ≈ 512,
     - fill `#FFFFFF`.
   - **Remove the chevron entirely.**
   - No `<text>` elements — only the outline path (satisfies the original vector rule).
4. In the SVG comment, record: font used, fontSize, and the path bbox before/after
   transform (this replaces the per-letter bbox self-check).
5. Regenerate the icon set (`pnpm tauri icon design/app-icon.svg -o src-tauri/icons`)
   and report per the original brief. Original Hard constraints still apply.
