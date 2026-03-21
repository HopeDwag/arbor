use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, PartialEq)]
pub enum Focus {
    Sidebar,
    Terminal,
}

#[derive(Debug)]
pub enum Action {
    ToggleFocus,
    SidebarUp,
    SidebarDown,
    SidebarSelect,
    SidebarCreate,
    SidebarDelete,
    SidebarHelp,
    SidebarResizeLeft,
    SidebarResizeRight,
    FocusTerminal,
    TerminalInput(KeyEvent),
    Quit,
    None,
}

pub fn handle_key(key: KeyEvent, focus: &Focus) -> Action {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('a') {
        return Action::ToggleFocus;
    }

    match focus {
        Focus::Sidebar => match key.code {
            KeyCode::Up | KeyCode::Char('k') => Action::SidebarUp,
            KeyCode::Down | KeyCode::Char('j') => Action::SidebarDown,
            KeyCode::Enter => Action::SidebarSelect,
            KeyCode::Char('n') => Action::SidebarCreate,
            KeyCode::Char('d') => Action::SidebarDelete,
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
