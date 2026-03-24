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
    SidebarHelp,
    SidebarResizeLeft,
    SidebarResizeRight,
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

    match focus {
        Focus::Sidebar => match key.code {
            KeyCode::Up | KeyCode::Char('k') => Action::SidebarUp,
            KeyCode::Down | KeyCode::Char('j') => Action::SidebarDown,
            KeyCode::Enter => Action::SidebarSelect,
            KeyCode::Char('n') => Action::SidebarCreate,
            KeyCode::Char('a') => Action::SidebarArchive,
            KeyCode::Char('?') => Action::SidebarHelp,
            KeyCode::Char('<') => Action::SidebarResizeLeft,
            KeyCode::Char('>') => Action::SidebarResizeRight,
            KeyCode::Esc => Action::FocusTerminal,
            KeyCode::Char('q') => Action::Quit,
            _ => Action::None,
        },
        Focus::Terminal => Action::TerminalInput(key),
    }
}
