# Plan B: Visual Refresh (Everforest + Tree Theme) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all hardcoded ratatui colors with a centralized Everforest theme, add tree trunk/branch visuals to the sidebar, add tree emoji accents to section headers and footer, and remap terminal ANSI colors to Everforest.

**Architecture:** Create a new `src/ui/theme.rs` module exposing an Everforest color palette as `Color::Rgb` constants. Migrate all color references in `control_panel.rs`, `app.rs`, and `terminal.rs` to use the theme. Add box-drawing characters (│├└─●) to the sidebar gutter for tree visuals.

**Tech Stack:** Rust, ratatui 0.29 (Color::Rgb support), crossterm 0.28

**Prerequisite:** Plan A (Feature Enrichment) must be complete.

---

### Task 1: Create theme module with Everforest palette

**Files:**
- Create: `src/ui/theme.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Create `src/ui/theme.rs`**

```rust
use ratatui::style::Color;

pub struct Theme {
    pub bg: Color,
    pub bg0: Color,
    pub bg1: Color,
    pub bg2: Color,
    pub bg3: Color,
    pub bg4: Color,
    pub fg: Color,
    pub grey0: Color,
    pub grey1: Color,
    pub grey2: Color,
    pub red: Color,
    pub orange: Color,
    pub yellow: Color,
    pub green: Color,
    pub aqua: Color,
    pub blue: Color,
    pub purple: Color,
}

impl Theme {
    pub const fn everforest() -> Self {
        Self {
            bg:     Color::Rgb(0x27, 0x2e, 0x33),
            bg0:    Color::Rgb(0x23, 0x2a, 0x2e),
            bg1:    Color::Rgb(0x2e, 0x38, 0x3c),
            bg2:    Color::Rgb(0x37, 0x41, 0x45),
            bg3:    Color::Rgb(0x41, 0x4b, 0x50),
            bg4:    Color::Rgb(0x49, 0x51, 0x56),
            fg:     Color::Rgb(0xd3, 0xc6, 0xaa),
            grey0:  Color::Rgb(0x7a, 0x84, 0x78),
            grey1:  Color::Rgb(0x85, 0x92, 0x89),
            grey2:  Color::Rgb(0x9d, 0xa9, 0xa0),
            red:    Color::Rgb(0xe6, 0x7e, 0x80),
            orange: Color::Rgb(0xe6, 0x98, 0x75),
            yellow: Color::Rgb(0xdb, 0xbc, 0x7f),
            green:  Color::Rgb(0xa7, 0xc0, 0x80),
            aqua:   Color::Rgb(0x83, 0xc0, 0x92),
            blue:   Color::Rgb(0x7f, 0xbb, 0xb3),
            purple: Color::Rgb(0xd6, 0x99, 0xb6),
        }
    }
}

pub static THEME: Theme = Theme::everforest();
```

- [ ] **Step 2: Export from `src/ui/mod.rs`**

```rust
mod control_panel;
mod terminal;
pub mod theme;

pub use control_panel::render_control_panel;
pub use control_panel::ControlPanelState;
pub use terminal::render_terminal;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`

Expected: Compiles with no errors (theme is defined but not yet used)

- [ ] **Step 4: Commit**

```bash
git add src/ui/theme.rs src/ui/mod.rs
git commit -m "feat: add Everforest theme module"
```

---

### Task 2: Migrate control_panel.rs colors to theme

**Files:**
- Modify: `src/ui/control_panel.rs`

- [ ] **Step 1: Add theme import and replace all colors**

Add at the top of `src/ui/control_panel.rs`:

```rust
use crate::ui::theme::THEME;
```

Replace every color reference in the file. The full mapping:

| Find | Replace |
|------|---------|
| `Color::Cyan` | `THEME.aqua` |
| `Color::DarkGray` (in border/text styling) | `THEME.grey0` |
| `Color::DarkGray` (in `bg()` calls for dialogs/highlights) | `THEME.bg3` |
| `Color::White` | `THEME.fg` |
| `Color::Yellow` | `THEME.yellow` |
| `Color::Green` | `THEME.green` |
| `Color::Magenta` | `THEME.purple` |
| `Color::Gray` | `THEME.grey1` |
| `Color::Red` | `THEME.red` |

Specific replacements (line-by-line, referencing post-Plan-A state):

**Border styling:**
```rust
let border_style = if focused {
    Style::default().fg(THEME.aqua)
} else {
    Style::default().fg(THEME.bg3)
};
```

**Group headers:**
```rust
Style::default().fg(THEME.grey0).add_modifier(Modifier::BOLD)
```
Count badge:
```rust
Style::default().fg(THEME.grey1)
```

**Activity icons:**
```rust
Span::styled(format!("{} ", frame_char), Style::default().fg(THEME.aqua))
// ...
Span::styled("! ", Style::default().fg(THEME.yellow))
// ...
WorkflowStatus::Queued => Span::styled("\u{25B6} ", Style::default().fg(THEME.grey0)),
WorkflowStatus::Done => Span::styled("\u{2713} ", Style::default().fg(THEME.green)),
WorkflowStatus::InReview => Span::styled("\u{e728} ", Style::default().fg(THEME.aqua)),
WorkflowStatus::InProgress => Span::styled("\u{00B7} ", Style::default().fg(THEME.grey0)),
```

**Name styles:**
```rust
let name_style = if is_selected && focused {
    Style::default().fg(THEME.aqua).add_modifier(Modifier::BOLD)
} else if is_selected {
    Style::default().fg(THEME.grey0).add_modifier(Modifier::BOLD)
} else {
    Style::default().fg(THEME.fg)
};
```

**Tags:**
```rust
// Dirty
Span::styled(" M", Style::default().fg(THEME.yellow))
// PR colors
Color::Green -> THEME.green
Color::Yellow -> THEME.yellow
Color::Magenta -> THEME.purple
Color::Red -> THEME.red
```

**Row 2 (commit msg/stats):**
```rust
Style::default().fg(THEME.grey0) // commit message
Style::default().fg(THEME.aqua)  // ahead
Style::default().fg(THEME.yellow) // behind
Style::default().fg(THEME.grey0) // age
```

**Highlight:**
```rust
let highlight = if focused {
    Style::default().bg(THEME.bg3)
} else {
    Style::default()
};
```

**Footer:**
```rust
let new_style = if focused {
    Style::default().fg(THEME.green).add_modifier(Modifier::BOLD)
} else {
    Style::default().fg(THEME.grey0)
};
let hint_style = Style::default().fg(THEME.grey0);
```

**Filter bar:**
```rust
Span::styled(" \u{2315} ", Style::default().fg(THEME.aqua)),
Span::styled(format!("{}_", filter_text), Style::default().fg(THEME.aqua)),
```

**Dialog colors:** Replace all `Color::Cyan` → `THEME.aqua`, `Color::DarkGray` (bg) → `THEME.bg3`, `Color::White` → `THEME.fg`, `Color::Yellow` → `THEME.yellow`, `Color::Gray` → `THEME.grey1`.

- [ ] **Step 2: Verify it compiles and tests pass**

Run: `cargo test`

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/ui/control_panel.rs
git commit -m "refactor: migrate control_panel colors to Everforest theme"
```

---

### Task 3: Migrate app.rs colors to theme

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add theme import and replace all colors**

Add at the top of `src/app.rs`:

```rust
use crate::ui::theme::THEME;
```

**Detail bar (row 1):**
```rust
// Branch
Style::default().fg(THEME.aqua).add_modifier(Modifier::BOLD)
// Status label
Style::default().fg(THEME.grey0)
// PR colors
("Open", THEME.green), ("Draft", THEME.yellow), ("Merged", THEME.purple), ("Closed", THEME.red)
// Ahead/behind
Style::default().fg(THEME.aqua)  // ahead
Style::default().fg(THEME.yellow) // behind
```

**Detail bar (row 2):**
```rust
Style::default().fg(THEME.grey0) // path and URL
```

**Status bar (`build_status_line`):**
```rust
let bg = THEME.bg1;
let fg = THEME.fg;
let key_style = Style::default().fg(THEME.green).bg(bg).add_modifier(Modifier::BOLD);
let label_style = Style::default().fg(fg).bg(bg);
let sep_style = Style::default().fg(THEME.grey0).bg(bg);
// Flash:
let flash_style = Style::default().fg(THEME.green).bg(bg).add_modifier(Modifier::BOLD);
```

**Scroll indicator in terminal.rs** (will be handled in Task 5, not here).

- [ ] **Step 2: Verify it compiles and tests pass**

Run: `cargo test`

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "refactor: migrate app.rs colors to Everforest theme"
```

---

### Task 4: Tree trunk/branch visuals in sidebar

**Files:**
- Modify: `src/ui/control_panel.rs` (card rendering — prepend tree characters)

- [ ] **Step 1: Add tree gutter characters to card rows**

In `src/ui/control_panel.rs`, inside the `for (flat_idx, wt) in &group_wts` loop, determine if this is the last item in the group:

```rust
let is_last = group_wts.iter().position(|(idx, _)| *idx == *flat_idx).unwrap()
    == group_wts.len() - 1;
```

Replace the `Span::raw("  ")` prefix in row 1 with tree characters:

```rust
let trunk_color = if is_selected && focused { THEME.green } else { THEME.bg2 };

// Row 1: fork character + branch + leaf
let fork = if is_last { "\u{2514}\u{2500}" } else { "\u{251C}\u{2500}" };
let leaf_color = if is_selected && focused {
    THEME.green
} else {
    match wt.workflow_status {
        WorkflowStatus::InReview => THEME.blue,
        WorkflowStatus::Done => THEME.aqua,
        _ => THEME.bg3,
    }
};

let mut row1_spans = vec![
    Span::styled(fork, Style::default().fg(trunk_color)),
    Span::styled("\u{25CF}", Style::default().fg(leaf_color)),
    icon,
    Span::styled(display_name.clone(), name_style),
];
```

Replace the `Span::raw("    ")` prefix in row 2 with trunk continuation:

```rust
let trunk_cont = if is_last { "   " } else { "\u{2502}  " };
let mut row2_spans = vec![
    Span::styled(trunk_cont, Style::default().fg(trunk_color)),
    Span::styled(msg.chars().take(36).collect::<String>(), msg_style),
];
```

- [ ] **Step 2: Verify it compiles and visually inspect**

Run: `cargo build && cargo run -- --repo /path/to/test/repo`

Visually verify:
- `├─●` appears before non-last items
- `└─●` appears before the last item in each group
- `│` continues on row 2 for non-last items
- Active card shows green trunk + green leaf
- Review cards show blue leaf, Done cards show aqua leaf

- [ ] **Step 3: Run tests**

Run: `cargo test`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/ui/control_panel.rs
git commit -m "feat: add tree trunk/branch/leaf visuals to sidebar cards"
```

---

### Task 5: Tree emojis in headers, title, and footer

**Files:**
- Modify: `src/ui/control_panel.rs` (group headers, sidebar title, footer)

- [ ] **Step 1: Add emojis to group headers**

In `src/ui/control_panel.rs`, update the `groups` array:

```rust
let groups: &[(WorkflowStatus, &str)] = &[
    (WorkflowStatus::InProgress, "\u{1F33F} IN PROGRESS"),
    (WorkflowStatus::InReview, "\u{1F343} IN REVIEW"),
    (WorkflowStatus::Queued, "\u{1F331} QUEUED"),
    (WorkflowStatus::Done, "\u{1F342} DONE"),
];
```

- [ ] **Step 2: Update sidebar title**

Change the block title:

```rust
let block = Block::default()
    .title(" \u{1F332} arbor ")
    .borders(Borders::ALL)
    .border_style(border_style);
```

- [ ] **Step 3: Update footer count with tree emoji**

In the footer rendering:

```rust
let count_str = format!("\u{1F332} {} worktrees", wt_count);
```

- [ ] **Step 4: Verify it compiles and tests pass**

Run: `cargo test`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ui/control_panel.rs
git commit -m "feat: add tree emojis to section headers, title, and footer"
```

---

### Task 6: Terminal ANSI color mapping to Everforest

**Files:**
- Modify: `src/ui/terminal.rs:102-108` (convert_vt100_color)

- [ ] **Step 1: Add theme import and remap ANSI indices**

Add at the top of `src/ui/terminal.rs`:

```rust
use crate::ui::theme::THEME;
```

Replace `convert_vt100_color`:

```rust
fn convert_vt100_color(color: vt100_ctt::Color) -> Color {
    match color {
        vt100_ctt::Color::Default => Color::Reset,
        vt100_ctt::Color::Idx(i) => match i {
            0 => THEME.bg3,
            1 | 9 => THEME.red,
            2 | 10 => THEME.green,
            3 | 11 => THEME.yellow,
            4 | 12 => THEME.blue,
            5 | 13 => THEME.purple,
            6 | 14 => THEME.aqua,
            7 | 15 => THEME.fg,
            8 => THEME.bg4,
            _ => Color::Indexed(i),
        },
        vt100_ctt::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
```

Also update the scroll indicator:

```rust
let indicator_style = Style::reset().fg(THEME.bg0).bg(THEME.yellow).add_modifier(Modifier::BOLD);
```

And update `dim_style` / `dim_color` to use theme colors:

```rust
fn dim_style(style: Style) -> Style {
    let fg = style.fg.map(dim_color).unwrap_or(THEME.grey0);
    let bg = style.bg.unwrap_or(Color::Reset);
    Style::reset().fg(fg).bg(bg).add_modifier(Modifier::DIM)
}
```

- [ ] **Step 2: Verify it compiles and tests pass**

Run: `cargo test`

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/ui/terminal.rs
git commit -m "refactor: map terminal ANSI colors to Everforest palette"
```
