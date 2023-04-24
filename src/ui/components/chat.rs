use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use chrono::Local;
use log::warn;
use rustyline::{line_buffer::LineBuffer, At, Word};
use tokio::sync::broadcast::Sender;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::{
    emotes::{
        emotes_enabled, hide_all_emotes, hide_message_emotes, show_span_emotes, Emotes,
        SharedEmotes,
    },
    handlers::{
        app::SharedMessages,
        config::{SharedCompleteConfig, Theme},
        data::DataBuilder,
        state::{NormalMode, State},
        user_input::{
            events::{Event, Key},
            input::TerminalAction,
        },
    },
    twitch::TwitchAction,
    ui::{
        components::Component,
        statics::{COMMANDS, TWITCH_MESSAGE_LIMIT},
    },
    utils::{
        styles::{BORDER_NAME_DARK, BORDER_NAME_LIGHT},
        text::{first_similarity, get_cursor_position, title_spans, TitleStyle},
    },
};

use super::utils::centered_rect;

#[derive(Debug)]
pub struct ChatWidget {
    config: SharedCompleteConfig,
    tx: Sender<TwitchAction>,
    messages: SharedMessages,
    emotes: Option<SharedEmotes>,
    // filters: SharedFilters,
    // theme: Theme,
}

impl ChatWidget {
    pub fn new(
        config: SharedCompleteConfig,
        tx: Sender<TwitchAction>,
        messages: SharedMessages,
    ) -> Self {
        Self {
            config,
            tx,
            messages,
            emotes: None,
        }
    }

    pub fn get_messages<'a, B: Backend>(
        &self,
        frame: &mut Frame<B>,
        v_chunks: Rc<[Rect]>,
        scroll_offset: usize,
        input: LineBuffer,
    ) -> VecDeque<Spans<'a>> {
        // Accounting for not all heights of rows to be the same due to text wrapping,
        // so extra space needs to be used in order to scroll correctly.
        let mut total_row_height: usize = 0;

        let mut messages = VecDeque::new();

        let general_chunk_height = v_chunks[0].height as usize - 2;

        // Horizontal chunks represents the list within the main chat window.
        let h_chunk = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1)])
            .split(frame.size());

        let message_chunk_width = h_chunk[0].width as usize;

        // let channel_switcher = if app.get_state() == State::ChannelSwitch {
        //     Some(centered_rect(60, 20, frame.size()))
        // } else {
        //     None
        // };

        // let is_behind_channel_switcher =
        //     |a, b| channel_switcher.map_or(false, |r| is_in_rect(r, a, b));

        let is_behind_channel_switcher = |_a, _b| false;

        let config = self.config.borrow();
        let messages_data = self.messages.borrow();

        'outer: for data in messages_data.iter() {
            // if app.filters.contaminated(data.payload.clone().as_str()) {
            //     continue;
            // }

            // Offsetting of messages for scrolling through said messages
            if scroll_offset > 0 {
                // scroll_offset -= 1;
                // hide_message_emotes(&data.emotes, self.emotes.borrow_mut().displayed, data.payload.width());
                let mut map = HashMap::new();
                hide_message_emotes(&data.emotes, &mut map, data.payload.width());

                continue;
            }

            let username_highlight: Option<&str> = if config.frontend.username_highlight {
                Some(config.twitch.username.as_str())
            } else {
                None
            };

            let spans = data.to_spans(
                &self.config.borrow().frontend,
                message_chunk_width,
                // if input.is_empty() {
                //     None
                // } else {
                //     match app.get_state() {
                //         State::Normal(Some(NormalMode::Search)) => Some(app.input_buffer.as_str()),
                //         _ => None,
                //     }
                // },
                None,
                username_highlight,
            );

            let mut payload = " ".to_string();
            payload.push_str(&data.payload);

            for span in spans.iter().rev() {
                let mut span = span.clone();

                if total_row_height < general_chunk_height {
                    if !data.emotes.is_empty() {
                        let current_row = general_chunk_height - total_row_height;
                        match show_span_emotes(
                            &data.emotes,
                            &mut span,
                            &mut self.emotes.as_ref().unwrap().borrow_mut(),
                            &payload,
                            self.config.borrow().frontend.margin as usize,
                            current_row as u16,
                            is_behind_channel_switcher,
                        ) {
                            Ok(p) => payload = p,
                            Err(e) => warn!("Unable to display some emotes: {e}"),
                        }
                    }

                    messages.push_front(span);
                    total_row_height += 1;
                } else {
                    if !emotes_enabled(&self.config.borrow().frontend)
                        || self.emotes.as_ref().unwrap().borrow().displayed.is_empty()
                    {
                        break 'outer;
                    }

                    // If the current message already had all its emotes deleted, the following messages should
                    // also have had their emotes deleted
                    hide_message_emotes(
                        &data.emotes,
                        &mut self.emotes.clone().unwrap().borrow_mut().displayed,
                        payload.width(),
                    );
                    if !data.emotes.is_empty()
                        && !data.emotes.iter().all(|e| {
                            !self
                                .emotes
                                .clone()
                                .unwrap()
                                .borrow()
                                .displayed
                                .contains_key(&(e.id, e.pid))
                        })
                    {
                        break 'outer;
                    }
                }
            }
        }

        // Padding with empty rows so chat can go from bottom to top.
        if general_chunk_height > total_row_height {
            for _ in 0..(general_chunk_height - total_row_height) {
                messages.push_front(Spans::from(vec![Span::raw("")]));
            }
        }

        // TODO: fix
        messages
    }
}

impl Component for ChatWidget {
    fn draw<B: Backend>(&self, f: &mut Frame<B>, area: Option<Rect>, emotes: Option<Emotes>) {
        // TODO: Don't let this be a thing
        let mut emotes = emotes.unwrap();
        let config = self.config.borrow();

        let area = area.map_or_else(|| f.size(), |a| a);

        let v_chunks: Rc<[Rect]> = Layout::default()
            .direction(Direction::Vertical)
            .margin(self.config.borrow().frontend.margin.clone())
            .constraints([Constraint::Min(1)])
            .split(area);

        if self.messages.borrow().len() > self.config.borrow().terminal.maximum_messages.clone() {
            for data in self
                .messages
                .borrow()
                .range(self.config.borrow().terminal.maximum_messages..)
            {
                hide_message_emotes(&data.emotes, &mut emotes.displayed, data.payload.width());
            }
            self.messages
                .borrow_mut()
                .truncate(self.config.borrow().terminal.maximum_messages);
        }

        // If we show the help screen, no need to get any messages
        // let messages = if app.get_state() == State::Help {
        //     hide_all_emotes(&mut emotes);
        //     VecDeque::new()
        // } else {
        //     get_messages(f, config, emotes, v_chunks.clone())
        // };

        // TODO: Remove this after making [`InputComponent`] a reality
        let input = LineBuffer::with_capacity(4096);

        let messages = self.get_messages(f, v_chunks.clone(), 0, input);

        let current_time = Local::now()
            .format(&config.frontend.date_format)
            .to_string();

        let spans = [
            TitleStyle::Combined("Time", &current_time),
            TitleStyle::Combined("Channel", config.twitch.channel.as_str()),
            // TitleStyle::Custom(Span::styled(
            //     if app.filters.reversed() {
            //         "retliF"
            //     } else {
            //         "Filter"
            //     },
            //     Style::default()
            //         .add_modifier(Modifier::BOLD)
            //         .fg(if app.filters.enabled() {
            //             Color::Green
            //         } else {
            //             Color::Red
            //         }),
            // )),
        ];

        let chat_title = if self.config.borrow().frontend.title_shown {
            Spans::from(title_spans(
                &spans,
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ))
        } else {
            Spans::default()
        };

        let mut final_messages = vec![];

        for item in messages {
            final_messages.push(ListItem::new(Text::from(item)));
        }

        let list = List::new(final_messages)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(self.config.borrow().frontend.border_type.clone().into())
                    .title(chat_title), // .style(match self.theme {
                                        //     Theme::Light => BORDER_NAME_LIGHT,
                                        //     _ => BORDER_NAME_DARK,
                                        // }),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(list, v_chunks[0]);

        // if config.frontend.state_tabs {
        //     render_state_tabs(frame, &layout, &app.get_state());
        // }

        // match window.app.get_state() {
        //     // States of the application that require a chunk of the main window
        //     State::Insert => render_chat_box(window, config.storage.mentions),
        //     State::MessageSearch => {
        //         let checking_func = |s: String| -> bool { !s.is_empty() };

        //         render_insert_box(
        //             window,
        //             "Message Search",
        //             None,
        //             None,
        //             Some(Box::new(checking_func)),
        //         );
        //     }

        //     // States that require popups
        //     State::Help => render_help_window(window),
        //     // State::ChannelSwitch => app.components.channel_switcher.draw(frame),
        //     _ => {}
        // }

        // todo!()
    }

    fn event(&mut self, event: &Event) -> Option<TerminalAction> {
        if let Event::Input(key) = event {
            match key {
                Key::Char('q') => return Some(TerminalAction::Quitting),
                Key::Esc => return Some(TerminalAction::BackOneLayer),
                Key::Ctrl('p') => panic!("Manual panic triggered by user."),
                _ => {}
            }
        }

        None
    }
}

// pub fn render_chat_box<T: Backend>(mention_suggestions: bool) {
//     let input_buffer = &app.input_buffer;

//     let current_input = input_buffer.to_string();

//     let suggestion = if mention_suggestions {
//         input_buffer
//             .chars()
//             .next()
//             .and_then(|start_character| match start_character {
//                 '/' => {
//                     let possible_suggestion = first_similarity(
//                         &COMMANDS
//                             .iter()
//                             .map(ToString::to_string)
//                             .collect::<Vec<String>>(),
//                         &current_input[1..],
//                     );

//                     let default_suggestion = possible_suggestion.clone();

//                     possible_suggestion.map_or(default_suggestion, |s| Some(format!("/{s}")))
//                 }
//                 '@' => {
//                     let possible_suggestion =
//                         first_similarity(&app.storage.get("mentions"), &current_input[1..]);

//                     let default_suggestion = possible_suggestion.clone();

//                     possible_suggestion.map_or(default_suggestion, |s| Some(format!("@{s}")))
//                 }
//                 _ => None,
//             })
//     } else {
//         None
//     };

//     render_insert_box(
//         window,
//         format!(
//             "Message Input: {} / {}",
//             current_input.len(),
//             *TWITCH_MESSAGE_LIMIT
//         )
//         .as_str(),
//         None,
//         suggestion,
//         Some(Box::new(|s: String| -> bool {
//             s.len() < *TWITCH_MESSAGE_LIMIT
//         })),
//     );
// }
