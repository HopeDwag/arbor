use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, PartialEq)]
pub enum Focus {
    Sidebar,
    Terminal,
}

#[derive(Debug)]
pub enum Action {
    ToggleFocus,
    FocusSidebar,
    FocusTerminal,
    SidebarUp,
    SidebarDown,
    SidebarSelect,
    SidebarCreate,
    SidebarArchive,
    StatusCycle,
    Filter,
    OpenPR,
    TerminalInput(KeyEvent),
    Quit,
    None,
}

pub fn handle_key(key: KeyEvent, focus: &Focus) -> Action {
    // Shift+Arrow works from either pane
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        match key.code {
            KeyCode::Left => return Action::FocusSidebar,
            KeyCode::Right => return Action::FocusTerminal,
            _ => {}
        }
    }

    // Ctrl-a toggle still works as a fallback
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('a') {
        return Action::ToggleFocus;
    }

    // Ctrl-g opens PR in browser (sidebar) or passes through (terminal)
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('g') {
        return match focus {
            Focus::Sidebar => Action::OpenPR,
            Focus::Terminal => Action::TerminalInput(key),
        };
    }

    match focus {
        Focus::Sidebar => match key.code {
            KeyCode::Up => Action::SidebarUp,
            KeyCode::Down => Action::SidebarDown,
            KeyCode::Enter => Action::SidebarSelect,
            KeyCode::Char('n') => Action::SidebarCreate,
            KeyCode::Char('a') => Action::SidebarArchive,
            KeyCode::Char('s') => Action::StatusCycle,
            KeyCode::Char('/') => Action::Filter,
            KeyCode::Esc | KeyCode::Char('q') => Action::Quit,
            _ => Action::None,
        },
        Focus::Terminal => Action::TerminalInput(key),
    }
}
