use crossterm::event::KeyEvent;
use ratatui::{
    prelude::Rect,
    layout::{Constraint, Direction, Layout},
    text::Line,
    widgets::{Block, Borders, Tabs},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::{
    action::Action,
    components::{Component, 
        home::Home,
        ktree::Ktree,
        lei::Lei,
        patchsets::Patchsets,
        config_view::ConfigView,
    },
    config::Config,
    tui::{Event, Tui},
};

pub struct App {
    config: Config,
    tick_rate: f64,
    frame_rate: f64,
    tabs_components: HashMap<Mode, Vec<Box<dyn Component>>>,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Home,
    Config,
}

impl App {
    pub fn new(
        tick_rate: f64,
        frame_rate: f64,
        domain: String,
        list: String,
        query: String,
    ) -> color_eyre::Result<Self> {
        let config = Config::new()?;
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let mut tabs_components: HashMap<Mode, Vec<Box<dyn Component>>> = HashMap::new();
        tabs_components.insert(Mode::Home, vec![
            Box::new(Home::new()),
            Box::new(Ktree::new()),
            Box::new(Lei::new(domain, list, query)),
            Box::new(Patchsets::new()),
        ]);
        tabs_components.insert(Mode::Config, vec![
            Box::new(ConfigView::new(config.clone()))
        ]);
        Ok(Self {
            tick_rate,
            frame_rate,
            tabs_components,
            should_quit: false,
            should_suspend: false,
            config,
            mode: Mode::Home,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
        })
    }

    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let mut tui = Tui::new()?
            // .mouse(true) // uncomment this line to enable mouse support
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        for components in self.tabs_components.values_mut() {
            for component in components.iter_mut() {
                component.register_action_handler(self.action_tx.clone())?;
                component.register_config_handler(self.config.clone())?;
                component.init(tui.size()?)?;
            }
        }

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                // tui.mouse(true);
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }
        match self.tabs_components.get_mut(&self.mode) {
            Some(components) => {
                for component in components.iter_mut() {
                    if let Some(action) = component.handle_events(Some(event.clone()))? {
                        action_tx.send(action)?;
                    }
                }
            }
            None => {
                // TODO: Feedback
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<()> {
        self.last_tick_key_events.push(key);

        if let Some(action) = self.config.keybindings.get_action(&self.mode, &self.last_tick_key_events) {
            info!("Got action: {action:?}");
            self.action_tx.send(action.clone())?;
        }

        Ok(())
    }

    fn handle_actions(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                debug!("{action:?}");
            }
            match action {
                Action::Tick => {
                    self.last_tick_key_events.drain(..);
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                Action::SwitchModeHome => self.mode = Mode::Home,
                Action::SwitchModeConfig => self.mode = Mode::Config,
                _ => {}
            }
            match self.tabs_components.get_mut(&self.mode) {
                Some(components) => {
                    for component in components.iter_mut() {
                        if let Some(action) = component.update(action.clone())? {
                            self.action_tx.send(action)?
                        };
                    }
                }
                None => {
                    // TODO: Feedback
                }
            }
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> color_eyre::Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        tui.draw(|frame| {
            // Split screen: 3 lines for tabs at top
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(frame.area());

            // Configure titles and discovers which is active
            let titles = vec![Line::from(" 1. Home "), Line::from(" 2. Config ")];
            let tab_index = match self.mode {
                Mode::Home => 0,
                Mode::Config => 1,
            };

            // Creates and render the tabs widget at top (chunks)
            let tabs = Tabs::new(titles)
                .block(Block::default().borders(Borders::ALL).title(" Tabs "))
                .select(tab_index)
                .highlight_style(
                    ratatui::style::Style::default()
                        .add_modifier(ratatui::style::Modifier::REVERSED)
                );

            frame.render_widget(tabs, chunks[0]);

            // Render tabs_components based active tab
            match self.tabs_components.get_mut(&self.mode) {
                Some(components) => {
                    for component in components.iter_mut() {
                        if let Err(err) = component.draw(frame, chunks[1]) {
                            let _ = self
                                .action_tx
                                .send(Action::Error(format!("Failed to draw: {:?}", err)));
                        }
                    }
                }
                None => {
                    // TODO: Feedback
                }
            }
        })?;
        Ok(())
    }
}
