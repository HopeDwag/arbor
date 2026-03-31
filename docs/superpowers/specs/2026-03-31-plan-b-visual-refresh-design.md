# Plan B: Visual Refresh (Everforest + Tree Theme)

Purely cosmetic changes applied after Plan A is complete. Replaces scattered hardcoded colors with a centralized Everforest theme, adds tree trunk/branch visuals to the sidebar, and adds tree emoji accents. No new functionality — only changes how existing data is presented.

**Prerequisite:** Plan A (Feature Enrichment) must be complete. Plan B styles the two-row cards, footer, detail bar, and badges that Plan A introduces.

## 1. Theme Module

### New File: `src/ui/theme.rs`

A struct holding all Everforest Dark Hard colors as `Color::Rgb(r, g, b)`:

```rust
pub struct Theme {
    pub bg: Color,    // #272e33
    pub bg0: Color,   // #232a2e
    pub bg1: Color,   // #2e383c
    pub bg2: Color,   // #374145
    pub bg3: Color,   // #414b50
    pub bg4: Color,   // #495156
    pub fg: Color,    // #d3c6aa
    pub grey0: Color, // #7a8478
    pub grey1: Color, // #859289
    pub grey2: Color, // #9da9a0
    pub red: Color,   // #e67e80
    pub orange: Color,// #e69875
    pub yellow: Color,// #dbbc7f
    pub green: Color, // #a7c080
    pub aqua: Color,  // #83c092
    pub blue: Color,  // #7fbbb3
    pub purple: Color,// #d699b6
}
```

A `Theme::everforest()` constructor returns the palette. Exposed as a constant or passed by reference to rendering functions.

### Color Migration

Every rendering call site replaces hardcoded `Color::*` values:

| Current | Everforest |
|---------|-----------|
| `Color::Cyan` | `theme.aqua` |
| `Color::DarkGray` (borders) | `theme.bg3` |
| `Color::DarkGray` (text) | `theme.grey0` |
| `Color::White` | `theme.fg` |
| `Color::Yellow` | `theme.yellow` |
| `Color::Green` | `theme.green` |
| `Color::Magenta` | `theme.purple` |
| `Color::Gray` | `theme.grey1` |
| `Color::Red` | `theme.red` |
| `Color::Black` | `theme.bg0` |

No runtime theme switching — just a clean single source of truth.

## 2. Tree Trunk/Branch Visuals

### Sidebar Gutter

Each card's 2-char left padding is replaced with tree-drawing characters:

**Non-last item in group (row 1):** `├─●` then content
**Non-last item in group (row 2):** `│  ` then content
**Last item in group (row 1):** `└─●` then content
**Last item in group (row 2):** `   ` then content (trunk capped)

Characters:
- `│` (U+2502) — vertical trunk
- `├` (U+251C) — fork (non-last)
- `└` (U+2514) — fork (last in group)
- `─` (U+2500) — horizontal branch
- `●` (U+25CF) — leaf node

### Colors

- Trunk/branch characters: `theme.bg2` (inactive), `theme.green` (active/selected card)
- Leaf node `●` colored by status:
  - Selected+focused: `theme.green`
  - InReview: `theme.blue`
  - Done: `theme.aqua`
  - Otherwise: `theme.bg3`

### Implementation

The tree characters are prepended to each card's `Line` spans in `render_control_panel()`. A `is_last_in_group` flag is tracked during the group rendering loop to decide between `├` and `└`.

## 3. Emojis and Badges

### Section Headers

Prepend emoji to each group label:

| Status | Emoji | Header |
|--------|-------|--------|
| InProgress | 🌿 (U+1F33F) | `🌿 IN PROGRESS` |
| InReview | 🍃 (U+1F343) | `🍃 IN REVIEW` |
| Queued | 🌱 (U+1F331) | `🌱 QUEUED` |
| Done | 🍂 (U+1F342) | `🍂 DONE` |

### Section Count Badges

After the header label, render the group item count in `theme.grey1` on `theme.bg1` background:

```
🌿 IN PROGRESS  3
```

The count is right-aligned or placed inline after the label text.

### Sidebar Title

Change from `" arbor "` to `" 🌲 arbor "`.

### Footer

Prepend `🌲` to the worktree count: `🌲 8 worktrees`.

## 4. Terminal Color Mapping

### Modify `convert_vt100_color()` in terminal.rs

Replace standard ANSI 16-color indices (0-15) with Everforest RGB values:

| Index | ANSI Name | Everforest |
|-------|-----------|-----------|
| 0 | Black | `theme.bg3` |
| 1 | Red | `theme.red` |
| 2 | Green | `theme.green` |
| 3 | Yellow | `theme.yellow` |
| 4 | Blue | `theme.blue` |
| 5 | Magenta | `theme.purple` |
| 6 | Cyan | `theme.aqua` |
| 7 | White | `theme.fg` |
| 8-15 | Bright variants | Same as 0-7 |

Indices 16-255 (extended palette) and true-color RGB values pass through unchanged.

## 5. Status Bar Restyle

Apply theme colors to `build_status_line()`:

| Element | Current | Everforest |
|---------|---------|-----------|
| Background | `Color::DarkGray` | `theme.bg1` |
| Key hints | `Color::Cyan` + Bold | `theme.green` + Bold |
| Labels | `Color::White` | `theme.fg` |
| Separators | `Color::Gray` | `theme.grey0` |
| Flash messages | `Color::Green` + Bold | `theme.green` + Bold |

## Files Changed

| File | Changes |
|------|---------|
| `src/ui/theme.rs` | **New file.** Everforest color definitions |
| `src/ui/mod.rs` | Export theme module |
| `src/ui/control_panel.rs` | Tree trunk/branch/leaf rendering, emoji headers, count badges, footer emoji, all colors → theme |
| `src/ui/terminal.rs` | ANSI → Everforest color mapping in `convert_vt100_color()` |
| `src/app.rs` | Detail bar colors → theme, status bar colors → theme, header colors → theme |
| `src/keys.rs` | No changes |
| `src/persistence.rs` | No changes |
