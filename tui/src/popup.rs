use std::io;

use base64::{prelude::BASE64_STANDARD, Engine};
use crossterm::event::Event as CrosstermEvent;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, BorderType, Clear, StatefulWidget, Widget},
};
use ratatui_explorer::{FileExplorer, Theme};
use ratatui_image::{
    picker::{Picker, ProtocolType},
    protocol::StatefulProtocol,
    StatefulImage,
};
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::{Input, Key};

use crate::app::Event;

pub enum Popup {
    FileExplorer(FileExplorer, UnboundedSender<Event>),
    ImagePreview(Box<dyn StatefulProtocol>, UnboundedSender<Event>),
    MarkdownPreview(String, UnboundedSender<Event>),
}

impl Popup {
    pub fn file_explorer(event_sender: UnboundedSender<Event>) -> io::Result<Self> {
        let theme = Theme::default()
            .add_default_title()
            .with_title_bottom(|fe| format!("[ {} files ]", fe.files().len()).into())
            .with_style(Color::Yellow)
            .with_highlight_item_style(Modifier::BOLD)
            .with_highlight_dir_style(Style::new().blue().bold())
            .with_highlight_symbol("> ")
            .with_block(Block::bordered().border_type(BorderType::Rounded));
        let file_explorer = FileExplorer::with_theme(theme)?;
        Ok(Self::FileExplorer(file_explorer, event_sender))
    }

    pub fn image_preview(
        contents: String,
        event_sender: UnboundedSender<Event>,
    ) -> anyhow::Result<Popup> {
        let data = BASE64_STANDARD.decode(contents.as_bytes())?;
        let img = image::load_from_memory(&data)?;
        let user_fontsize = (7, 14);
        let user_protocol = ProtocolType::Halfblocks;
        let mut picker = Picker::new(user_fontsize);
        picker.protocol_type = user_protocol;
        let image = picker.new_resize_protocol(img);
        Ok(Popup::ImagePreview(image, event_sender))
    }

    pub fn markdown_preview(
        contents: String,
        event_sender: UnboundedSender<Event>,
    ) -> anyhow::Result<Popup> {
        let contents = BASE64_STANDARD.decode(contents.as_bytes())?;
        Ok(Popup::MarkdownPreview(
            String::from_utf8(contents)?,
            event_sender,
        ))
    }

    pub async fn handle_input(
        &mut self,
        input: Input,
        raw_event: CrosstermEvent,
    ) -> anyhow::Result<()> {
        match self {
            Popup::FileExplorer(ref mut explorer, ref mut event_sender) => match input.key {
                Key::Esc => {
                    let _ = event_sender.send(Event::PopupClosed);
                }
                Key::Enter => {
                    let file = explorer.current().clone();
                    if file.is_dir() {
                        return Ok(());
                    }
                    let event = Event::FileSelected(file);
                    let _ = event_sender.send(event);
                    let _ = event_sender.send(Event::PopupClosed);
                }
                _ => explorer.handle(&raw_event)?,
            },
            Popup::ImagePreview(_, ref event_sender) if input.key == Key::Esc => {
                let _ = event_sender.send(Event::PopupClosed);
            }
            Popup::MarkdownPreview(_, ref event_sender) if input.key == Key::Esc => {
                let _ = event_sender.send(Event::PopupClosed);
            }
            _ => {}
        }
        Ok(())
    }
}

impl Widget for &mut Popup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            Popup::FileExplorer(explorer, _) => render_explorer(area, buf, explorer),
            Popup::ImagePreview(ref mut protocol, _) => render_image_preview(area, buf, protocol),
            Popup::MarkdownPreview(contents, _) => {
                render_markdown_preview(area, buf, contents);
            }
        }
    }
}

fn render_explorer(area: Rect, buf: &mut Buffer, explorer: &mut FileExplorer) {
    let popup_area = popup_area(area, 50, 50);
    Clear.render(popup_area, buf);
    explorer.widget().render(popup_area, buf);
}

fn render_image_preview(area: Rect, buf: &mut Buffer, protocol: &mut Box<dyn StatefulProtocol>) {
    let popup_area = popup_area(area, 80, 80);
    Clear.render(popup_area, buf);
    let image = StatefulImage::new(None);
    image.render(popup_area, buf, protocol);
}

fn render_markdown_preview(area: Rect, buf: &mut Buffer, contents: &str) {
    let popup_area = popup_area(area, 80, 80);
    Clear.render(popup_area, buf);
    let text = tui_markdown::from_str(contents);
    text.render(popup_area, buf);
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}