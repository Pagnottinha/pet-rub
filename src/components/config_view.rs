use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style, Modifier},
    widgets::{Block, BorderType, Borders, HighlightSpacing, List, ListState},
};
use std::{fs, path::Path};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::Action,
    config::Config,
};

pub struct ConfigView {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    list_state: ListState,
    display_items: Vec<String>,
}

impl ConfigView {
    pub fn new(config: Config) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            command_tx: None,
            config,
            list_state,
            display_items: Vec::new(),
        }
    }

    fn build_display_items(&mut self) {
        self.display_items.clear();
    
        self.display_items.push("=== DIRECTORIES ===".to_string());
        self.display_items.push(format!("  Data Dir:   {}", self.config.config.data_dir.display()));
        self.display_items.push(format!("  Config Dir: {}", self.config.config.config_dir.display()));
        self.display_items.push("".to_string());

        self.display_items.push("=== GLOBAL KEYBINDINGS ===".to_string());
        if self.config.keybindings.global.is_empty() {
            self.display_items.push("  No global keybinding configured.".to_string());
        } else {
            for (keys, action) in &self.config.keybindings.global {
                let key_strs: Vec<String> = keys.iter().map(|k| crate::config::key_event_to_string(k)).collect();
                self.display_items.push(format!("  {} -> {}", key_strs.join(" + "), action));
            }
        }
        self.display_items.push("".to_string());

        for (mode, binds) in &self.config.keybindings.modes {
            self.display_items.push(format!("=== MODE: {:?} ===", mode));
            if binds.is_empty() {
                self.display_items.push("  No keybinding defined.".to_string());
            } else {
                for (keys, action) in binds {
                    let key_strs: Vec<String> = keys.iter().map(|k| crate::config::key_event_to_string(k)).collect();
                    self.display_items.push(format!("  {} -> {}", key_strs.join(" + "), action));
                }
            }
            self.display_items.push("".to_string());
        }
    }
}

impl Component for ConfigView {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> color_eyre::Result<()> {
        self.config = config;
        self.build_display_items();
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let next = match self.list_state.selected() {
                    Some(selected) if !self.display_items.is_empty() => {
                        if selected < self.display_items.len() - 1 {
                            selected + 1
                        } else {
                            0
                        }
                    }
                    _ => 0,
                };
                self.list_state.select(Some(next));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let prev = match self.list_state.selected() {
                    Some(selected) if !self.display_items.is_empty() => {
                        if selected > 0 {
                            selected - 1
                        } else {
                            self.display_items.len() - 1
                        }
                    }
                    _ => 0,
                };
                self.list_state.select(Some(prev));
            }
            _ => {}
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::EditConfig => {
                let config_path = self.config.config.config_dir.join("config.json5");
                let target_dir = Path::new("/tmp/config.json5");
                fs::copy(config_path, target_dir)?;
                return Ok(Some(Action::EditFile(
                        target_dir.to_string_lossy().into_owned(),
                        Some(Box::new(Action::ValidateAndSaveConfig))
                )));
            },
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let list_block = Block::default()
            .title(" System Configs ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));

        let list = List::new(self.display_items.clone())
            .block(list_block)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);

        frame.render_stateful_widget(list, area, &mut self.list_state);
        
        Ok(())
    }
}