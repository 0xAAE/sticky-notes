use super::{
    PopupVariant, get_popup_item_by_index,
    service::Message,
    utils::{cosmic_font, with_background},
};
use crate::{
    app::utils::{background_color, text_color},
    fl,
    icons::IconSet,
    notes::{NoteStyle, NotesCollection},
};
use cosmic::{
    iced::{Length, window::Id},
    widget::{self, text_editor::Action},
};
use cosmic::{
    prelude::*,
    widget::markdown::{
        Highlight, Item, Settings, Style as MarkdownStyle, Text, Uri, Viewer as MarkdownViewer,
    },
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum StickyWindowError {
    #[error("already in edit mode")]
    AlreadyEditing,
    #[error("not in edit mode")]
    EditingIsOff,
}

pub struct StickyWindow {
    note_id: Uuid,
    edit_context: Option<EditContext>,
    style_names: Option<Vec<String>>,
    icon_size: u16,
    // optionally display popup menu
    popup_menu: Option<PopupVariant>,
    // optionally display a toolbar in view mode (in edit mode the toolbar is always visible)
    view_toolbar: bool,
    // markdown
    markdown: Vec<Item>,
}

struct EditContext {
    /// currently edited content
    content: widget::text_editor::Content,
}

impl StickyWindow {
    pub fn new(
        note_id: Uuid,
        notes: &NotesCollection,
        icon_size: u16,
        popup_menu: Option<PopupVariant>,
    ) -> Self {
        let markdown = if let Ok(note) = notes.try_get_note(&note_id) {
            widget::markdown::parse(note.get_content()).collect()
        } else {
            Vec::new()
        };
        Self {
            note_id,
            edit_context: None,
            style_names: None,
            icon_size,
            popup_menu,
            view_toolbar: false,
            markdown,
        }
    }

    pub fn get_note_id(&self) -> Uuid {
        self.note_id
    }

    pub fn hide_popup_menu(&mut self) {
        self.popup_menu = None;
    }

    pub fn start_edit(&mut self, init_content: &str) -> Result<(), StickyWindowError> {
        if self.edit_context.is_some() {
            Err(StickyWindowError::AlreadyEditing)
        } else {
            self.edit_context = Some(EditContext {
                content: widget::text_editor::Content::with_text(init_content),
            });
            Ok(())
        }
    }

    pub fn finish_edit(&mut self) -> Result<String, StickyWindowError> {
        self.edit_context
            .take()
            .map(|context| {
                let s = context.content.text();
                self.markdown = widget::markdown::parse(&s).collect();
                s
            })
            .ok_or(StickyWindowError::EditingIsOff)
    }

    pub fn is_editing(&self) -> bool {
        self.edit_context.is_some()
    }

    pub fn do_edit_action(&mut self, action: Action) -> Result<(), StickyWindowError> {
        self.edit_context
            .as_mut()
            .map(|context| context.content.perform(action))
            .ok_or(StickyWindowError::EditingIsOff)
    }

    pub fn allow_select_style(&mut self, style_names: Vec<String>) {
        self.style_names = Some(style_names);
    }

    pub fn disable_select_style(&mut self) {
        self.style_names = None;
    }

    // true - toolbar is visible
    // false - toolbar is hidden
    pub fn set_toolbar_visibility(&mut self, is_visible: bool) {
        self.view_toolbar = is_visible;
    }

    #[allow(clippy::too_many_lines)]
    pub fn build_view<'a>(
        &'a self,
        window_id: Id,
        notes: &'a NotesCollection,
        icons: &IconSet,
    ) -> Element<'a, Message> {
        if let Some(edit_context) = &self.edit_context {
            if let Ok(style) = notes.try_get_note_style(self.get_note_id()) {
                let note_toolbar = widget::row::with_capacity(1).push(
                    icons
                        .checked()
                        .apply(widget::button::icon)
                        .icon_size(self.icon_size)
                        .on_press(Message::NoteEdit(window_id, false))
                        .width(Length::Shrink),
                );

                let note_content = widget::container(
                    widget::text_editor(&edit_context.content)
                        .on_action(move |act| Message::Edit(window_id, act))
                        .height(Length::Fill),
                )
                .width(Length::Fill)
                .height(Length::Fill);

                with_background(
                    widget::column::with_capacity(2)
                        .push(note_toolbar)
                        .push(note_content)
                        .into(),
                    style.get_background_color(),
                    style.is_light(),
                )
            } else {
                widget::text("problem-text").into()
            }
        } else if let Ok(note) = notes.try_get_note(&self.note_id)
            && let Ok(style) = notes.try_get_style(&note.style())
        {
            let is_locked = note.is_locked();

            let note_toolbar = if self.view_toolbar {
                let mut toolbar =
                    widget::row::with_capacity(8).spacing(cosmic::theme::spacing().space_s);
                // display menu variant optionally:
                if let Some(menu) = &self.popup_menu {
                    if let PopupVariant::DropdownMenu(popup_list) = menu {
                        toolbar = toolbar.push(widget::dropdown(popup_list, None, |index| {
                            Message::Signal(get_popup_item_by_index(index))
                        }));
                    } else {
                        // PopupVariant::AppletMenu
                        toolbar = toolbar.push(
                            icons
                                .menu()
                                .apply(widget::button::icon)
                                .icon_size(self.icon_size)
                                .on_press(Message::OpenMenu(window_id))
                                .width(Length::Shrink),
                        );
                    }
                }
                // lock / unlock
                toolbar = toolbar.push(
                    if is_locked {
                        icons.unlock()
                    } else {
                        icons.lock()
                    }
                    .apply(widget::button::icon)
                    .icon_size(self.icon_size)
                    .on_press(Message::NoteLock(window_id, !is_locked))
                    .width(Length::Shrink),
                );
                // copy if content is not empty
                if !note.get_content().is_empty() {
                    toolbar = toolbar.push(
                        icons
                            .copy()
                            .apply(widget::button::icon)
                            .icon_size(self.icon_size)
                            .on_press(Message::NoteCopy(self.note_id))
                            .width(Length::Shrink),
                    );
                }
                // more options if it is unlocked
                if !is_locked {
                    toolbar = toolbar.push(
                        icons
                            .edit()
                            .apply(widget::button::icon)
                            .icon_size(self.icon_size)
                            .on_press(Message::NoteEdit(window_id, true))
                            .width(Length::Shrink),
                    );
                    if let Some(styles) = &self.style_names {
                        // add style pick list
                        toolbar = toolbar.push(
                            widget::dropdown(
                                styles,
                                notes
                                    .try_get_note_style_index(self.note_id)
                                    .map_err(|e| tracing::error!("failed to get style index: {e}"))
                                    .ok(),
                                move |index| Message::NoteStyleSelected(window_id, index),
                            )
                            .placeholder(fl!("select-default-style")),
                        );
                    } else {
                        // add button "down"
                        toolbar = toolbar.push(
                            icons
                                .down()
                                .apply(widget::button::icon)
                                .icon_size(self.icon_size)
                                .on_press(Message::NoteStyle(window_id))
                                .width(Length::Shrink),
                        );
                    }
                    toolbar = toolbar.push(
                        icons
                            .delete()
                            .apply(widget::button::icon)
                            .icon_size(self.icon_size)
                            .on_press(Message::NoteDelete(window_id))
                            .width(Length::Shrink),
                    );
                }
                toolbar = toolbar.push(widget::space().width(Length::Fill)).push(
                    icons
                        .create()
                        .apply(widget::button::icon)
                        .icon_size(self.icon_size)
                        .on_press(Message::NoteNew)
                        .width(Length::Shrink),
                );
                toolbar
            } else {
                widget::row(None)
            };

            // build markdown view element
            let theme_accessor = cosmic::theme::active();
            let theme = theme_accessor.cosmic();
            let font = cosmic_font(style.get_font().style);
            let markdown_style = MarkdownStyle {
                font,
                code_block_font: cosmic::font::mono(),
                inline_code_font: font,
                inline_code_color: theme.text_button.selected_text.into(),
                inline_code_highlight: Highlight {
                    background: theme.background(true).base.into(),
                    border: cosmic::iced::border::rounded(0.1),
                },
                inline_code_padding: cosmic::iced::Padding::ZERO,
                link_color: theme.link_button.selected_text.into(),
            };
            let settings = Settings::with_text_size(style.get_font().size, markdown_style);
            let note_content = widget::column::with_capacity(2)
                .width(Length::Fill)
                .height(Length::Fill)
                //.push(colored_markdown_container);
                .push(
                    widget::markdown::view_with(
                        &self.markdown, //items,
                        settings,
                        &CodeBlockViewer(style),
                    )
                    .map(Message::OpenUrl),
                );
            with_background(
                widget::column::with_capacity(2)
                    .push(note_toolbar)
                    .push(note_content)
                    .into(),
                style.get_background_color(),
                style.is_light(),
            )
        } else {
            // build problem view
            widget::text("problem-text").into()
        }
    }
}

// CodeBlockViewer is to customize code block depending on particular NoteStyle
#[derive(Debug, Clone, Copy)]
struct CodeBlockViewer<'a>(&'a NoteStyle);

impl<'a> MarkdownViewer<'a, Uri, Theme, Renderer> for CodeBlockViewer<'a> {
    fn on_link_click(url: Uri) -> Uri {
        url
    }

    /// Displays a code block.
    fn code_block(
        &self,
        settings: Settings,
        _language: Option<&'_ str>,
        code: &'a str,
        _lines: &'a [Text],
    ) -> cosmic::iced::core::Element<'a, Uri, Theme, Renderer> {
        let font = self.0.get_font();

        // code block widget
        let code_widget = widget::text(code)
            .size(font.size)
            .font(cosmic::font::mono());

        // container for code block to apply text and background colors
        widget::container(
            widget::scrollable(code_widget).direction(
                cosmic::iced::widget::scrollable::Direction::Horizontal(
                    cosmic::iced::widget::scrollable::Scrollbar::default()
                        .width(settings.code_size / 2)
                        .scroller_width(settings.code_size / 2),
                ),
            ),
        )
        .style(|_theme| widget::container::Style {
            text_color: Some(text_color(self.0.is_light()).into()), // Делаем текст внутри кода белым
            background: Some(cosmic::iced::Background::Color(
                background_color(self.0.is_light()).into(),
            )),
            ..Default::default()
        })
        .width(Length::Fill)
        .padding(settings.code_size / 4)
        .into() // Конвертируем в Element
    }
}
