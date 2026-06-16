use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
};

use tokio::sync::mpsc::UnboundedSender;

use crate::components::{Action, Component};

pub struct Popup {
    command_tx: Option<UnboundedSender<Action>>,
    show: bool,
    title: String,
    description: String,
    action: Option<Box<Action>>
}

impl Popup {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            show: false,
            title: String::new(),
            description: String::new(),
            action: None
        }
    }
}

impl Component for Popup {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::ShowPopup(ref title, ref description, ref action) => {
                self.title = title.to_string();
                self.description = description.to_string();
                self.action = action.clone();
                self.show = true;
            },
            Action::ClosePopup => {
                let action: Option<Action> = self.action.clone().map(|boxed| *boxed);
                self.show = false;
                self.action = None;
                return Ok(action)
            },
            _ => {}
        }
        Ok(None)
    }
    
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if self.show {
            let popup_area = center_rect(60, 40, area); // 60% width, 40% height

            // Clear the specific area to overwrite any background text
            frame.render_widget(Clear, popup_area);

            let popup_block = Block::default()
                .title(self.title.clone())
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow));

            frame.render_widget(&popup_block, popup_area);

            let inner_area = popup_block.inner(popup_area);
            
            let width = inner_area.width.max(1);
            
            let text_height = self.description
                .lines()
                .map(|line| (line.chars().count() as u16).div_ceil(width))
                .sum::<u16>()
                .max(1);
            
            let vertical_slices = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Fill(1),   // Top spacing pushes the content down
                    Constraint::Length(text_height), // Height of your text (1 row)
                    Constraint::Fill(1),   // Bottom spacing keeps it even
                ])
                .split(inner_area);

            let center_row_area = vertical_slices[1];

            let popup_text = Paragraph::new(self.description.clone())
                .wrap(Wrap { trim: true });

            frame.render_widget(popup_text, center_row_area);
        }

        Ok(())
    }
}

fn center_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
