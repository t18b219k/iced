//! Display fields that can be filled with text.
//!
//! A [`TextInput`] has some local [`State`].
pub mod cursor;
mod editor;
mod ime_state;
mod value;

pub use cursor::Cursor;
use iced_renderer::core::text::Paragraph;
pub use value::Value;

use editor::Editor;

use crate::core::alignment;
use crate::core::event::{self, Event};
use crate::core::ime;
use crate::core::keyboard;
use crate::core::layout;
use crate::core::mouse::{self, click};
use crate::core::renderer;
use crate::core::text::{self, Text};
use crate::core::time::{Duration, Instant};
use crate::core::touch;
use crate::core::widget;
use crate::core::widget::operation::{self, Operation};
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Clipboard, Color, Element, Layout, Length, Padding, Pixels, Point,
    Rectangle, Shell, Size, Vector, Widget, IME,
};
use crate::runtime::Command;

pub use iced_style::text_input::{Appearance, StyleSheet};

use self::ime_state::IMEState;

/// A field that can be filled with text.
///
/// # Example
/// ```no_run
/// # pub type TextInput<'a, Message> =
/// #     iced_widget::TextInput<'a, Message, iced_widget::renderer::Renderer<iced_widget::style::Theme>>;
/// #
/// #[derive(Debug, Clone)]
/// enum Message {
///     TextInputChanged(String),
/// }
///
/// let value = "Some text";
///
/// let input = TextInput::new(
///     "This is the placeholder...",
///     value,
/// )
/// .on_input(Message::TextInputChanged)
/// .padding(10);
/// ```
/// ![Text input drawn by `iced_wgpu`](https://github.com/iced-rs/iced/blob/7760618fb112074bc40b148944521f312152012a/docs/images/text_input.png?raw=true)
#[allow(missing_debug_implementations)]
pub struct TextInput<'a, Message, Renderer = crate::Renderer>
where
    Renderer: text::Renderer,
    Renderer::Theme: StyleSheet,
{
    id: Option<Id>,
    placeholder: String,
    value: Value,
    is_secure: bool,
    font: Option<Renderer::Font>,
    width: Length,
    padding: Padding,
    size: Option<Pixels>,
    line_height: text::LineHeight,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_paste: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
    icon: Option<Icon<Renderer::Font>>,
    style: <Renderer::Theme as StyleSheet>::Style,
}

/// The default [`Padding`] of a [`TextInput`].
pub const DEFAULT_PADDING: Padding = Padding::new(5.0);

impl<'a, Message, Renderer> TextInput<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: text::Renderer,
    Renderer::Theme: StyleSheet,
{
    /// Creates a new [`TextInput`].
    ///
    /// It expects:
    /// - a placeholder,
    /// - the current value
    pub fn new(placeholder: &str, value: &str) -> Self {
        TextInput {
            id: None,
            placeholder: String::from(placeholder),
            value: Value::new(value),
            is_secure: false,
            font: None,
            width: Length::Fill,
            padding: DEFAULT_PADDING,
            size: None,
            line_height: text::LineHeight::default(),
            on_input: None,
            on_paste: None,
            on_submit: None,
            icon: None,
            style: Default::default(),
        }
    }

    /// Sets the [`Id`] of the [`TextInput`].
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Converts the [`TextInput`] into a secure password input.
    pub fn password(mut self) -> Self {
        self.is_secure = true;
        self
    }

    /// Sets the message that should be produced when some text is typed into
    /// the [`TextInput`].
    ///
    /// If this method is not called, the [`TextInput`] will be disabled.
    pub fn on_input<F>(mut self, callback: F) -> Self
    where
        F: 'a + Fn(String) -> Message,
    {
        self.on_input = Some(Box::new(callback));
        self
    }

    /// Sets the message that should be produced when the [`TextInput`] is
    /// focused and the enter key is pressed.
    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }

    /// Sets the message that should be produced when some text is pasted into
    /// the [`TextInput`].
    pub fn on_paste(
        mut self,
        on_paste: impl Fn(String) -> Message + 'a,
    ) -> Self {
        self.on_paste = Some(Box::new(on_paste));
        self
    }

    /// Sets the [`Font`] of the [`TextInput`].
    ///
    /// [`Font`]: text::Renderer::Font
    pub fn font(mut self, font: Renderer::Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Sets the [`Icon`] of the [`TextInput`].
    pub fn icon(mut self, icon: Icon<Renderer::Font>) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Sets the width of the [`TextInput`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the [`Padding`] of the [`TextInput`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the text size of the [`TextInput`].
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Sets the [`text::LineHeight`] of the [`TextInput`].
    pub fn line_height(
        mut self,
        line_height: impl Into<text::LineHeight>,
    ) -> Self {
        self.line_height = line_height.into();
        self
    }

    /// Sets the style of the [`TextInput`].
    pub fn style(
        mut self,
        style: impl Into<<Renderer::Theme as StyleSheet>::Style>,
    ) -> Self {
        self.style = style.into();
        self
    }

    /// Lays out the [`TextInput`], overriding its [`Value`] if provided.
    ///
    /// [`Renderer`]: text::Renderer
    pub fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
        value: Option<&Value>,
    ) -> layout::Node {
        layout(
            renderer,
            limits,
            self.width,
            self.padding,
            self.size,
            self.font,
            self.line_height,
            self.icon.as_ref(),
            tree.state.downcast_mut::<State<Renderer::Paragraph>>(),
            value.unwrap_or(&self.value),
            &self.placeholder,
            self.is_secure,
        )
    }

    /// Draws the [`TextInput`] with the given [`Renderer`], overriding its
    /// [`Value`] if provided.
    ///
    /// [`Renderer`]: text::Renderer
    pub fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        value: Option<&Value>,
    ) {
        draw(
            renderer,
            theme,
            layout,
            cursor,
            tree.state.downcast_ref::<State<Renderer::Paragraph>>(),
            value.unwrap_or(&self.value),
            self.on_input.is_none(),
            self.is_secure,
            self.icon.as_ref(),
            &self.style,
        );
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer>
    for TextInput<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: text::Renderer,
    Renderer::Theme: StyleSheet,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph>::new())
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        // Unfocus text input if it becomes disabled
        if self.on_input.is_none() {
            state.last_click = None;
            state.is_focused = None;
            state.is_pasting = None;
            state.is_dragging = false;
        }
    }

    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout(
            renderer,
            limits,
            self.width,
            self.padding,
            self.size,
            self.font,
            self.line_height,
            self.icon.as_ref(),
            tree.state.downcast_mut::<State<Renderer::Paragraph>>(),
            &self.value,
            &self.placeholder,
            self.is_secure,
        )
    }

    fn operate(
        &self,
        tree: &mut Tree,
        _layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation<Message>,
    ) {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        operation.focusable(state, self.id.as_ref().map(|id| &id.0));
        operation.text_input(state, self.id.as_ref().map(|id| &id.0));
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        ime: &dyn IME,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        update(
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            ime,
            shell,
            &mut self.value,
            self.size,
            self.line_height,
            self.font,
            self.is_secure,
            self.on_input.as_deref(),
            self.on_paste.as_deref(),
            &self.on_submit,
            || tree.state.downcast_mut::<State<Renderer::Paragraph>>(),
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        draw(
            renderer,
            theme,
            layout,
            cursor,
            tree.state.downcast_ref::<State<Renderer::Paragraph>>(),
            &self.value,
            self.on_input.is_none(),
            self.is_secure,
            self.icon.as_ref(),
            &self.style,
        );
    }

    fn mouse_interaction(
        &self,
        _state: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse_interaction(layout, cursor, self.on_input.is_none())
    }
}

impl<'a, Message, Renderer> From<TextInput<'a, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + text::Renderer,
    Renderer::Theme: StyleSheet,
{
    fn from(
        text_input: TextInput<'a, Message, Renderer>,
    ) -> Element<'a, Message, Renderer> {
        Element::new(text_input)
    }
}

/// The content of the [`Icon`].
#[derive(Debug, Clone)]
pub struct Icon<Font> {
    /// The font that will be used to display the `code_point`.
    pub font: Font,
    /// The unicode code point that will be used as the icon.
    pub code_point: char,
    /// The font size of the content.
    pub size: Option<Pixels>,
    /// The spacing between the [`Icon`] and the text in a [`TextInput`].
    pub spacing: f32,
    /// The side of a [`TextInput`] where to display the [`Icon`].
    pub side: Side,
}

/// The side of a [`TextInput`].
#[derive(Debug, Clone)]
pub enum Side {
    /// The left side of a [`TextInput`].
    Left,
    /// The right side of a [`TextInput`].
    Right,
}

/// The identifier of a [`TextInput`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Id(widget::Id);

impl Id {
    /// Creates a custom [`Id`].
    pub fn new(id: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        Self(widget::Id::new(id))
    }

    /// Creates a unique [`Id`].
    ///
    /// This function produces a different [`Id`] every time it is called.
    pub fn unique() -> Self {
        Self(widget::Id::unique())
    }
}

impl From<Id> for widget::Id {
    fn from(id: Id) -> Self {
        id.0
    }
}

/// Produces a [`Command`] that focuses the [`TextInput`] with the given [`Id`].
pub fn focus<Message: 'static>(id: Id) -> Command<Message> {
    Command::widget(operation::focusable::focus(id.0))
}

/// Produces a [`Command`] that moves the cursor of the [`TextInput`] with the given [`Id`] to the
/// end.
pub fn move_cursor_to_end<Message: 'static>(id: Id) -> Command<Message> {
    Command::widget(operation::text_input::move_cursor_to_end(id.0))
}

/// Produces a [`Command`] that moves the cursor of the [`TextInput`] with the given [`Id`] to the
/// front.
pub fn move_cursor_to_front<Message: 'static>(id: Id) -> Command<Message> {
    Command::widget(operation::text_input::move_cursor_to_front(id.0))
}

/// Produces a [`Command`] that moves the cursor of the [`TextInput`] with the given [`Id`] to the
/// provided position.
pub fn move_cursor_to<Message: 'static>(
    id: Id,
    position: usize,
) -> Command<Message> {
    Command::widget(operation::text_input::move_cursor_to(id.0, position))
}

/// Produces a [`Command`] that selects all the content of the [`TextInput`] with the given [`Id`].
pub fn select_all<Message: 'static>(id: Id) -> Command<Message> {
    Command::widget(operation::text_input::select_all(id.0))
}

/// Computes the layout of a [`TextInput`].
pub fn layout<Renderer>(
    renderer: &Renderer,
    limits: &layout::Limits,
    width: Length,
    padding: Padding,
    size: Option<Pixels>,
    font: Option<Renderer::Font>,
    line_height: text::LineHeight,
    icon: Option<&Icon<Renderer::Font>>,
    state: &mut State<Renderer::Paragraph>,
    value: &Value,
    placeholder: &str,
    is_secure: bool,
) -> layout::Node
where
    Renderer: text::Renderer,
{
    let font = font.unwrap_or_else(|| renderer.default_font());
    let text_size = size.unwrap_or_else(|| renderer.default_size());

    let padding = padding.fit(Size::ZERO, limits.max());
    let limits = limits
        .width(width)
        .pad(padding)
        .height(line_height.to_absolute(text_size));

    let text_bounds = limits.resolve(Size::ZERO);

    let placeholder_text = Text {
        font,
        line_height,
        content: placeholder,
        bounds: Size::new(f32::INFINITY, text_bounds.height),
        size: text_size,
        horizontal_alignment: alignment::Horizontal::Left,
        vertical_alignment: alignment::Vertical::Center,
        shaping: text::Shaping::Advanced,
    };

    state.placeholder.update(placeholder_text);

    let secure_value = is_secure.then(|| value.secure());
    let value = secure_value.as_ref().unwrap_or(value);

    state.value.update(Text {
        content: &value.to_string(),
        ..placeholder_text
    });

    if let Some(icon) = icon {
        let icon_text = Text {
            line_height,
            content: &icon.code_point.to_string(),
            font: icon.font,
            size: icon.size.unwrap_or_else(|| renderer.default_size()),
            bounds: Size::new(f32::INFINITY, text_bounds.height),
            horizontal_alignment: alignment::Horizontal::Center,
            vertical_alignment: alignment::Vertical::Center,
            shaping: text::Shaping::Advanced,
        };

        state.icon.update(icon_text);

        let icon_width = state.icon.min_width();

        let mut text_node = layout::Node::new(
            text_bounds - Size::new(icon_width + icon.spacing, 0.0),
        );

        let mut icon_node =
            layout::Node::new(Size::new(icon_width, text_bounds.height));

        match icon.side {
            Side::Left => {
                text_node.move_to(Point::new(
                    padding.left + icon_width + icon.spacing,
                    padding.top,
                ));

                icon_node.move_to(Point::new(padding.left, padding.top));
            }
            Side::Right => {
                text_node.move_to(Point::new(padding.left, padding.top));

                icon_node.move_to(Point::new(
                    padding.left + text_bounds.width - icon_width,
                    padding.top,
                ));
            }
        };

        layout::Node::with_children(
            text_bounds.pad(padding),
            vec![text_node, icon_node],
        )
    } else {
        let mut text = layout::Node::new(text_bounds);
        text.move_to(Point::new(padding.left, padding.top));

        layout::Node::with_children(text_bounds.pad(padding), vec![text])
    }
}

/// Processes an [`Event`] and updates the [`State`] of a [`TextInput`]
/// accordingly.
pub fn update<'a, Message, Renderer>(
    event: Event,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    renderer: &Renderer,
    clipboard: &mut dyn Clipboard,
    ime: &dyn IME,
    shell: &mut Shell<'_, Message>,
    value: &mut Value,
    size: Option<Pixels>,
    line_height: text::LineHeight,
    font: Option<Renderer::Font>,
    is_secure: bool,
    on_input: Option<&dyn Fn(String) -> Message>,
    on_paste: Option<&dyn Fn(String) -> Message>,
    on_submit: &Option<Message>,
    state: impl FnOnce() -> &'a mut State<Renderer::Paragraph>,
) -> event::Status
where
    Message: Clone,
    Renderer: text::Renderer,
{
    let update_cache = |state, value| {
        replace_paragraph(
            renderer,
            state,
            layout,
            value,
            font,
            size,
            line_height,
        );
    };

    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerPressed { .. }) => {
            let state = state();
            if state.is_focused.is_some() {
                if is_secure {
                    ime.password_mode();
                } else {
                    ime.inside();
                }
            }
            let click_position = if on_input.is_some() {
                cursor.position_over(layout.bounds())
            } else {
                None
            };
            let focus_gained =
                state.is_focused.is_none() && click_position.is_some();
            let focus_lost =
                state.is_focused.is_some() && click_position.is_none();
            state.is_focused = if click_position.is_some() {
                state.is_focused.or_else(|| {
                    let now = Instant::now();

                    Some(Focus {
                        updated_at: now,
                        now,
                        is_window_focused: true,
                    })
                })
            } else {
                None
            };

            if let Some(cursor_position) = click_position {
                let text_layout = layout.children().next().unwrap();
                let target = cursor_position.x - text_layout.bounds().x;

                let click =
                    mouse::Click::new(cursor_position, state.last_click);

                match click.kind() {
                    click::Kind::Single => {
                        let position = if target > 0.0 {
                            let value = if is_secure {
                                value.secure()
                            } else {
                                value.clone()
                            };

                            find_cursor_position(
                                text_layout.bounds(),
                                &value,
                                state,
                                target,
                            )
                        } else {
                            None
                        }
                        .unwrap_or(0);

                        if state.keyboard_modifiers.shift() {
                            state.cursor.select_range(
                                state.cursor.start(value),
                                position,
                            );
                        } else {
                            state.cursor.move_to(position);
                        }
                        state.is_dragging = true;
                    }
                    click::Kind::Double => {
                        if is_secure {
                            state.cursor.select_all(value);
                        } else {
                            let position = find_cursor_position(
                                text_layout.bounds(),
                                value,
                                state,
                                target,
                            )
                            .unwrap_or(0);

                            state.cursor.select_range(
                                value.previous_start_of_word(position),
                                value.next_end_of_word(position),
                            );
                        }

                        state.is_dragging = false;
                    }
                    click::Kind::Triple => {
                        state.cursor.select_all(value);
                        state.is_dragging = false;
                    }
                }

                state.last_click = Some(click);

                if !is_secure && focus_gained {
                    let bounds = text_layout.bounds();
                    let cursor_index =
                        find_cursor_position(bounds, value, state, target)
                            .unwrap_or(0);
                    let (width, offset) = measure_cursor_and_scroll_offset(
                        &state.value,
                        bounds,
                        cursor_index,
                    );
                    ime.set_ime_position(
                        (bounds.x + width - offset) as i32,
                        (bounds.y + bounds.height) as i32,
                    );
                } else if focus_lost {
                    let mut editor = Editor::new(value, &mut state.cursor);
                    ime.outside();
                    if let Some((old_ime_state, on_input)) =
                        state.ime_state.take().zip(on_input)
                    {
                        old_ime_state
                            .before_preedit_text()
                            .chars()
                            .for_each(|ch| editor.insert(ch));
                        let message = (on_input)(editor.contents());
                        shell.publish(message);
                    }
                }

                return event::Status::Captured;
            }
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerLifted { .. })
        | Event::Touch(touch::Event::FingerLost { .. }) => {
            state().is_dragging = false;
        }
        Event::Mouse(mouse::Event::CursorMoved { position })
        | Event::Touch(touch::Event::FingerMoved { position, .. }) => {
            let state = state();

            if state.is_dragging {
                let text_layout = layout.children().next().unwrap();
                let target = position.x - text_layout.bounds().x;

                let value = if is_secure {
                    value.secure()
                } else {
                    value.clone()
                };

                let position = find_cursor_position(
                    text_layout.bounds(),
                    &value,
                    state,
                    target,
                )
                .unwrap_or(0);

                state
                    .cursor
                    .select_range(state.cursor.start(&value), position);

                return event::Status::Captured;
            }
        }
        Event::Keyboard(keyboard::Event::CharacterReceived(c)) => {
            let state = state();

            if let Some(focus) = &mut state.is_focused {
                let Some(on_input) = on_input else {
                    return event::Status::Ignored;
                };

                if state.is_pasting.is_none()
                    && !state.keyboard_modifiers.command()
                    && !c.is_control()
                {
                    let mut editor = Editor::new(value, &mut state.cursor);

                    editor.insert(c);

                    let message = (on_input)(editor.contents());
                    shell.publish(message);

                    focus.updated_at = Instant::now();

                    update_cache(state, value);

                    return event::Status::Captured;
                }
            }
        }
        Event::Keyboard(keyboard::Event::KeyPressed { key_code, .. }) => {
            let state = state();

            if let Some(focus) = &mut state.is_focused {
                let Some(on_input) = on_input else {
                    return event::Status::Ignored;
                };

                let modifiers = state.keyboard_modifiers;
                focus.updated_at = Instant::now();

                match key_code {
                    keyboard::KeyCode::Enter
                    | keyboard::KeyCode::NumpadEnter => {
                        if let Some(on_submit) = on_submit.clone() {
                            shell.publish(on_submit);
                        }
                    }
                    keyboard::KeyCode::Backspace => {
                        if platform::is_jump_modifier_pressed(modifiers)
                            && state.cursor.selection(value).is_none()
                        {
                            if is_secure {
                                let cursor_pos = state.cursor.end(value);
                                state.cursor.select_range(0, cursor_pos);
                            } else {
                                state.cursor.select_left_by_words(value);
                            }
                        }

                        let mut editor = Editor::new(value, &mut state.cursor);
                        editor.backspace();

                        let message = (on_input)(editor.contents());
                        shell.publish(message);

                        update_cache(state, value);
                    }
                    keyboard::KeyCode::Delete => {
                        if platform::is_jump_modifier_pressed(modifiers)
                            && state.cursor.selection(value).is_none()
                        {
                            if is_secure {
                                let cursor_pos = state.cursor.end(value);
                                state
                                    .cursor
                                    .select_range(cursor_pos, value.len());
                            } else {
                                state.cursor.select_right_by_words(value);
                            }
                        }

                        let mut editor = Editor::new(value, &mut state.cursor);
                        editor.delete();

                        let message = (on_input)(editor.contents());
                        shell.publish(message);

                        update_cache(state, value);
                    }
                    keyboard::KeyCode::Left => {
                        if platform::is_jump_modifier_pressed(modifiers)
                            && !is_secure
                        {
                            if modifiers.shift() {
                                state.cursor.select_left_by_words(value);
                            } else {
                                state.cursor.move_left_by_words(value);
                            }
                        } else if modifiers.shift() {
                            state.cursor.select_left(value);
                        } else {
                            state.cursor.move_left(value);
                        }
                    }
                    keyboard::KeyCode::Right => {
                        if platform::is_jump_modifier_pressed(modifiers)
                            && !is_secure
                        {
                            if modifiers.shift() {
                                state.cursor.select_right_by_words(value);
                            } else {
                                state.cursor.move_right_by_words(value);
                            }
                        } else if modifiers.shift() {
                            state.cursor.select_right(value);
                        } else {
                            state.cursor.move_right(value);
                        }
                    }
                    keyboard::KeyCode::Home => {
                        if modifiers.shift() {
                            state
                                .cursor
                                .select_range(state.cursor.start(value), 0);
                        } else {
                            state.cursor.move_to(0);
                        }
                    }
                    keyboard::KeyCode::End => {
                        if modifiers.shift() {
                            state.cursor.select_range(
                                state.cursor.start(value),
                                value.len(),
                            );
                        } else {
                            state.cursor.move_to(value.len());
                        }
                    }
                    keyboard::KeyCode::C
                        if state.keyboard_modifiers.command() =>
                    {
                        if let Some((start, end)) =
                            state.cursor.selection(value)
                        {
                            clipboard
                                .write(value.select(start, end).to_string());
                        }
                    }
                    keyboard::KeyCode::X
                        if state.keyboard_modifiers.command() =>
                    {
                        if let Some((start, end)) =
                            state.cursor.selection(value)
                        {
                            clipboard
                                .write(value.select(start, end).to_string());
                        }

                        let mut editor = Editor::new(value, &mut state.cursor);
                        editor.delete();

                        let message = (on_input)(editor.contents());
                        shell.publish(message);

                        update_cache(state, value);
                    }
                    keyboard::KeyCode::V => {
                        if state.keyboard_modifiers.command()
                            && !state.keyboard_modifiers.alt()
                        {
                            let content = match state.is_pasting.take() {
                                Some(content) => content,
                                None => {
                                    let content: String = clipboard
                                        .read()
                                        .unwrap_or_default()
                                        .chars()
                                        .filter(|c| !c.is_control())
                                        .collect();

                                    Value::new(&content)
                                }
                            };

                            let mut editor =
                                Editor::new(value, &mut state.cursor);

                            editor.paste(content.clone());

                            let message = if let Some(paste) = &on_paste {
                                (paste)(editor.contents())
                            } else {
                                (on_input)(editor.contents())
                            };
                            shell.publish(message);

                            state.is_pasting = Some(content);

                            update_cache(state, value);
                        } else {
                            state.is_pasting = None;
                        }
                    }
                    keyboard::KeyCode::A
                        if state.keyboard_modifiers.command() =>
                    {
                        state.cursor.select_all(value);
                    }
                    keyboard::KeyCode::Escape => {
                        state.is_focused = None;
                        state.is_dragging = false;
                        state.is_pasting = None;

                        state.keyboard_modifiers =
                            keyboard::Modifiers::default();
                    }
                    keyboard::KeyCode::Tab
                    | keyboard::KeyCode::Up
                    | keyboard::KeyCode::Down => {
                        return event::Status::Ignored;
                    }
                    _ => {}
                }

                return event::Status::Captured;
            }
        }
        Event::Keyboard(keyboard::Event::KeyReleased { key_code, .. }) => {
            let state = state();

            if state.is_focused.is_some() {
                match key_code {
                    keyboard::KeyCode::V => {
                        state.is_pasting = None;
                    }
                    keyboard::KeyCode::Tab
                    | keyboard::KeyCode::Up
                    | keyboard::KeyCode::Down => {
                        return event::Status::Ignored;
                    }
                    _ => {}
                }

                return event::Status::Captured;
            } else {
                state.is_pasting = None;
            }
        }
        Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
            let state = state();

            state.keyboard_modifiers = modifiers;
        }
        Event::Window(window::Event::Unfocused) => {
            let state = state();

            if let Some(focus) = &mut state.is_focused {
                focus.is_window_focused = false;
            }
        }
        Event::Window(window::Event::Focused) => {
            let state = state();

            if let Some(focus) = &mut state.is_focused {
                focus.is_window_focused = true;
                focus.updated_at = Instant::now();

                shell.request_redraw(window::RedrawRequest::NextFrame);
            }
        }
        Event::Window(window::Event::RedrawRequested(now)) => {
            let state = state();

            if let Some(focus) = &mut state.is_focused {
                if focus.is_window_focused {
                    focus.now = now;

                    let millis_until_redraw = CURSOR_BLINK_INTERVAL_MILLIS
                        - (now - focus.updated_at).as_millis()
                            % CURSOR_BLINK_INTERVAL_MILLIS;

                    shell.request_redraw(window::RedrawRequest::At(
                        now + Duration::from_millis(millis_until_redraw as u64),
                    ));
                }
            }
        }
        Event::IME(ime::Event::IMEEnabled) => {
            let state = state();
            if state.is_focused.is_some() {
                let _ = state.ime_state.replace(IMEState::default());
                return event::Status::Captured;
            }
        }
        Event::IME(ime::Event::IMEPreedit(preedit_text, range)) => {
            let state = state();
            if let ("", None) = (preedit_text.as_str(), range) {
                state.ime_state = None;
                return event::Status::Captured;
            }

            if state.is_focused.is_none() || is_secure {
                return event::Status::Ignored;
            }

            if let Some(focus) = &mut state.is_focused {
                // calcurate where we need to place candidate window.
                let text_bounds = layout.children().next().unwrap().bounds();
                let before_preedit_cursor = state.cursor.start(value);
                // "A|BC"
                // if cursor is between A and B then we have preedit text あいうえお, we have to display
                // "A|あいうえおBC"
                // so we split value to 2 pieces .
                // 1 before preedit cursor.
                // 2 after preedit text.
                let inputted_text = value.to_string();
                let before_preedit_text: String =
                    inputted_text.chars().take(before_preedit_cursor).collect();
                let before_preedit_paragraph = Text {
                    content: &before_preedit_text.clone(),
                    bounds: Size {
                        width: f32::INFINITY,
                        height: text_bounds.height,
                    },
                    size: size.unwrap_or_else(|| renderer.default_size()),
                    line_height,
                    font: font.unwrap_or_else(|| renderer.default_font()),
                    horizontal_alignment: alignment::Horizontal::Left,
                    vertical_alignment: alignment::Vertical::Center,
                    shaping: text::Shaping::Advanced,
                };

                let after_preedit_text: String =
                    inputted_text.chars().skip(before_preedit_cursor).collect();

                let whole_text = before_preedit_text.clone()
                    + &preedit_text
                    + &after_preedit_text;

                let whole_text = Text {
                    content: &whole_text,
                    ..before_preedit_paragraph
                };
                let mut paragraph = Renderer::Paragraph::default();
                paragraph.update(whole_text);

                {
                    let (width, offset) = measure_cursor_and_scroll_offset(
                        &paragraph,
                        text_bounds,
                        before_preedit_cursor + preedit_text.chars().count(),
                    );
                    let position = (
                        (text_bounds.x + width - offset) as i32,
                        (text_bounds.y + text_bounds.height) as i32,
                    );
                    ime.set_ime_position(position.0, position.1);
                }
                // set current state to ime_state.
                if let Some(ime_state) = state.ime_state.as_mut() {
                    ime_state
                        .before_preedit_paragraph_mut()
                        .update(before_preedit_paragraph);
                    ime_state.whole_paragraph_mut().update(whole_text);
                    ime_state.set_event(preedit_text, range);
                    ime_state.set_before_preedit_text(before_preedit_text);
                } else {
                    let mut new_state =
                        IMEState::<Renderer::Paragraph>::default();
                    new_state
                        .before_preedit_paragraph_mut()
                        .update(before_preedit_paragraph);
                    new_state.whole_paragraph_mut().update(whole_text);
                    new_state.set_event(preedit_text, range);
                    new_state.set_before_preedit_text(before_preedit_text);
                    let _ = state.ime_state.replace(new_state);
                }
                // measure underline width for drawing.
                if let Some(ime_state) = &mut state.ime_state {
                    let measure_width = move |chunk: &str| {
                        let text = Text {
                            content: chunk,
                            ..whole_text
                        };
                        let cursor_index = chunk.chars().count();
                        let mut paragraph = Renderer::Paragraph::default();
                        paragraph.update(text);
                        measure_cursor_and_scroll_offset(
                            &paragraph,
                            text_bounds,
                            cursor_index,
                        )
                        .0
                    };
                    ime_state.measure_underlines(measure_width);
                }
                focus.updated_at = Instant::now();
            }

            return event::Status::Captured;
        }
        // Insert text characters to value.
        // and delete current IME state.
        Event::IME(ime::Event::IMECommit(text)) => {
            let state = state();
            if let Some(focus) = &mut state.is_focused {
                let Some(on_input) = on_input else {
                    return event::Status::Ignored;
                };
                if state.is_pasting.is_none()
                    && !state.keyboard_modifiers.command()
                {
                    let mut editor = Editor::new(value, &mut state.cursor);

                    text.chars().for_each(|ch| editor.insert(ch));

                    let message = (on_input)(editor.contents());
                    shell.publish(message);

                    focus.updated_at = Instant::now();

                    update_cache(state, value);
                    state.ime_state = None;
                    return event::Status::Captured;
                }
            }
        }
        Event::IME(ime::Event::IMEDisabled) => {
            let state = state();
            state.ime_state = None;
            return event::Status::Captured;
        }
        _ => {}
    }

    event::Status::Ignored
}

/// Draws the [`TextInput`] with the given [`Renderer`], overriding its
/// [`Value`] if provided.
///
/// [`Renderer`]: text::Renderer
pub fn draw<Renderer>(
    renderer: &mut Renderer,
    theme: &Renderer::Theme,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    state: &State<Renderer::Paragraph>,
    value: &Value,
    is_disabled: bool,
    is_secure: bool,
    icon: Option<&Icon<Renderer::Font>>,
    style: &<Renderer::Theme as StyleSheet>::Style,
) where
    Renderer: text::Renderer,
    Renderer::Theme: StyleSheet,
{
    let secure_value = is_secure.then(|| value.secure());
    let value = secure_value.as_ref().unwrap_or(value);

    let bounds = layout.bounds();

    let mut children_layout = layout.children();
    let text_bounds = children_layout.next().unwrap().bounds();

    let is_mouse_over = cursor.is_over(bounds);

    let appearance = if is_disabled {
        theme.disabled(style)
    } else if state.is_focused() {
        theme.focused(style)
    } else if is_mouse_over {
        theme.hovered(style)
    } else {
        theme.active(style)
    };

    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border_radius: appearance.border_radius,
            border_width: appearance.border_width,
            border_color: appearance.border_color,
        },
        appearance.background,
    );

    if icon.is_some() {
        let icon_layout = children_layout.next().unwrap();

        renderer.fill_paragraph(
            &state.icon,
            icon_layout.bounds().center(),
            appearance.icon_color,
        );
    }
    let preedit_text = state.ime_state.as_ref().map(|ime_state| {
        (
            ime_state.before_preedit_text(),
            ime_state.underlines(),
            ime_state.before_preedit_paragraph(),
            ime_state.before_cursor_text().chars().count(),
            ime_state.whole_paragraph(),
        )
    });

    let text = value.to_string();

    let (cursor, offset) = if let Some(focus) = state
        .is_focused
        .as_ref()
        .filter(|focus| focus.is_window_focused)
    {
        match state.cursor.state(value) {
            cursor::State::Index(position) => {
                // in ime mode A|BC
                // あいうえお inserted between A and B will be A|あいうえおBC
                // so we need A 's width to display underline and cursors.
                let (text_value_width, offset) = if let Some((
                    before_preedit_text,
                    _underlines,
                    _before_preedit_pragraph,
                    before_cursor_text_char_count,
                    whole_paragraph,
                )) = preedit_text
                {
                    measure_cursor_and_scroll_offset(
                        whole_paragraph,
                        text_bounds,
                        before_preedit_text.chars().count()
                            + before_cursor_text_char_count,
                    )
                } else {
                    measure_cursor_and_scroll_offset(
                        &state.value,
                        text_bounds,
                        position,
                    )
                };

                let is_cursor_visible = ((focus.now - focus.updated_at)
                    .as_millis()
                    / CURSOR_BLINK_INTERVAL_MILLIS)
                    % 2
                    == 0;

                let cursor = if is_cursor_visible {
                    Some((
                        renderer::Quad {
                            bounds: Rectangle {
                                x: text_bounds.x + text_value_width,
                                y: text_bounds.y,
                                width: 1.0,
                                height: text_bounds.height,
                            },
                            border_radius: 0.0.into(),
                            border_width: 0.0,
                            border_color: Color::TRANSPARENT,
                        },
                        theme.value_color(style),
                    ))
                } else {
                    None
                };

                (cursor, offset)
            }
            cursor::State::Selection { start, end } => {
                let left = start.min(end);
                let right = end.max(start);
                let (left_position, left_offset) =
                    measure_cursor_and_scroll_offset(
                        &state.value,
                        text_bounds,
                        left,
                    );

                let (right_position, right_offset) =
                    measure_cursor_and_scroll_offset(
                        &state.value,
                        text_bounds,
                        right,
                    );

                let width = right_position - left_position;

                (
                    Some((
                        renderer::Quad {
                            bounds: Rectangle {
                                x: text_bounds.x + left_position,
                                y: text_bounds.y,
                                width,
                                height: text_bounds.height,
                            },
                            border_radius: 0.0.into(),
                            border_width: 0.0,
                            border_color: Color::TRANSPARENT,
                        },
                        theme.selection_color(style),
                    )),
                    if end == right {
                        right_offset
                    } else {
                        left_offset
                    },
                )
            }
        }
    } else {
        (None, 0.0)
    };
    // in ime mode we need to use whole_paragraph for determine offsetting text.
    let text_width = if let Some(preedit_text) = preedit_text {
        preedit_text.4.min_width()
    } else {
        state.value.min_width()
    };

    let render = |renderer: &mut Renderer| {
        if let Some((cursor, color)) = cursor {
            renderer.fill_quad(cursor, color);
            // render underlines for ime mode.
            if let Some((
                before_preedit_text,
                Some(underlines),
                before_preedit_paragraph,
                _,
                _,
            )) = preedit_text
            {
                let (left, _) = measure_cursor_and_scroll_offset(
                    before_preedit_paragraph,
                    text_bounds,
                    0,
                );
                let (right, _) = measure_cursor_and_scroll_offset(
                    before_preedit_paragraph,
                    text_bounds,
                    before_preedit_text.chars().count(),
                );
                let before_preedit_width = right - left;
                underlines
                    .iter()
                    .enumerate()
                    .for_each(|(index, underline)| {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: underline.0
                                        + text_bounds.x
                                        + before_preedit_width,
                                    y: text_bounds.y + text_bounds.height,
                                    width: underline.1,
                                    height: if index == 1 { 2.0 } else { 1.0 },
                                },
                                border_radius: cursor.border_radius,
                                border_width: 0.0,
                                border_color: cursor.border_color,
                            },
                            theme.value_color(style),
                        );
                    });
            }
        } else {
            renderer.with_translation(Vector::ZERO, |_| {});
        }

        renderer.fill_paragraph(
            if let Some((_, _, _, _, whole_paragraph)) = preedit_text {
                whole_paragraph
            } else if text.is_empty() && preedit_text.is_none() {
                &state.placeholder
            } else {
                &state.value
            },
            Point::new(text_bounds.x, text_bounds.center_y()),
            if text.is_empty() && preedit_text.is_none() {
                theme.placeholder_color(style)
            } else if is_disabled {
                theme.disabled_color(style)
            } else {
                theme.value_color(style)
            },
        );
    };

    if text_width > text_bounds.width {
        renderer.with_layer(text_bounds, |renderer| {
            renderer.with_translation(Vector::new(-offset, 0.0), render);
        });
    } else {
        render(renderer);
    }
}

/// Computes the current [`mouse::Interaction`] of the [`TextInput`].
pub fn mouse_interaction(
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    is_disabled: bool,
) -> mouse::Interaction {
    if cursor.is_over(layout.bounds()) {
        if is_disabled {
            mouse::Interaction::NotAllowed
        } else {
            mouse::Interaction::Text
        }
    } else {
        mouse::Interaction::default()
    }
}

/// The state of a [`TextInput`].
#[derive(Debug, Default, Clone)]
pub struct State<P: text::Paragraph> {
    value: P,
    placeholder: P,
    icon: P,
    is_focused: Option<Focus>,
    is_dragging: bool,
    is_pasting: Option<Value>,
    ime_state: Option<ime_state::IMEState<P>>,
    last_click: Option<mouse::Click>,
    cursor: Cursor,
    keyboard_modifiers: keyboard::Modifiers,
    // TODO: Add stateful horizontal scrolling offset
}

#[derive(Debug, Clone, Copy)]
struct Focus {
    updated_at: Instant,
    now: Instant,
    is_window_focused: bool,
}

impl<P: text::Paragraph> State<P> {
    /// Creates a new [`State`], representing an unfocused [`TextInput`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new [`State`], representing a focused [`TextInput`].
    pub fn focused() -> Self {
        Self {
            value: P::default(),
            placeholder: P::default(),
            icon: P::default(),
            is_focused: None,
            is_dragging: false,
            is_pasting: None,
            last_click: None,
            cursor: Cursor::default(),
            keyboard_modifiers: keyboard::Modifiers::default(),
            ime_state: None,
        }
    }

    /// Returns whether the [`TextInput`] is currently focused or not.
    pub fn is_focused(&self) -> bool {
        self.is_focused.is_some()
    }

    /// Returns the [`Cursor`] of the [`TextInput`].
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// Focuses the [`TextInput`].
    pub fn focus(&mut self) {
        let now = Instant::now();

        self.is_focused = Some(Focus {
            updated_at: now,
            now,
            is_window_focused: true,
        });

        self.move_cursor_to_end();
    }

    /// Unfocuses the [`TextInput`].
    pub fn unfocus(&mut self) {
        self.is_focused = None;
    }

    /// Moves the [`Cursor`] of the [`TextInput`] to the front of the input text.
    pub fn move_cursor_to_front(&mut self) {
        self.cursor.move_to(0);
    }

    /// Moves the [`Cursor`] of the [`TextInput`] to the end of the input text.
    pub fn move_cursor_to_end(&mut self) {
        self.cursor.move_to(usize::MAX);
    }

    /// Moves the [`Cursor`] of the [`TextInput`] to an arbitrary location.
    pub fn move_cursor_to(&mut self, position: usize) {
        self.cursor.move_to(position);
    }

    /// Selects all the content of the [`TextInput`].
    pub fn select_all(&mut self) {
        self.cursor.select_range(0, usize::MAX);
    }
}

impl<P: text::Paragraph> operation::Focusable for State<P> {
    fn is_focused(&self) -> bool {
        State::is_focused(self)
    }

    fn focus(&mut self) {
        State::focus(self);
    }

    fn unfocus(&mut self) {
        State::unfocus(self);
    }
}

impl<P: text::Paragraph> operation::TextInput for State<P> {
    fn move_cursor_to_front(&mut self) {
        State::move_cursor_to_front(self);
    }

    fn move_cursor_to_end(&mut self) {
        State::move_cursor_to_end(self);
    }

    fn move_cursor_to(&mut self, position: usize) {
        State::move_cursor_to(self, position);
    }

    fn select_all(&mut self) {
        State::select_all(self);
    }
}

mod platform {
    use crate::core::keyboard;

    pub fn is_jump_modifier_pressed(modifiers: keyboard::Modifiers) -> bool {
        if cfg!(target_os = "macos") {
            modifiers.alt()
        } else {
            modifiers.control()
        }
    }
}

fn offset<P: text::Paragraph>(
    text_bounds: Rectangle,
    value: &Value,
    state: &State<P>,
) -> f32 {
    if state.is_focused() {
        let cursor = state.cursor();

        let focus_position = match cursor.state(value) {
            cursor::State::Index(i) => i,
            cursor::State::Selection { end, .. } => end,
        };

        let (_, offset) = measure_cursor_and_scroll_offset(
            &state.value,
            text_bounds,
            focus_position,
        );

        offset
    } else {
        0.0
    }
}

fn measure_cursor_and_scroll_offset(
    paragraph: &impl text::Paragraph,
    text_bounds: Rectangle,
    cursor_index: usize,
) -> (f32, f32) {
    let grapheme_position = paragraph
        .grapheme_position(0, cursor_index)
        .unwrap_or(Point::ORIGIN);

    let offset = ((grapheme_position.x + 5.0) - text_bounds.width).max(0.0);

    (grapheme_position.x, offset)
}

/// Computes the position of the text cursor at the given X coordinate of
/// a [`TextInput`].
fn find_cursor_position<P: text::Paragraph>(
    text_bounds: Rectangle,
    value: &Value,
    state: &State<P>,
    x: f32,
) -> Option<usize> {
    let offset = offset(text_bounds, value, state);
    let value = value.to_string();

    let char_offset = state
        .value
        .hit_test(Point::new(x + offset, text_bounds.height / 2.0))
        .map(text::Hit::cursor)?;

    Some(
        unicode_segmentation::UnicodeSegmentation::graphemes(
            &value[..char_offset.min(value.len())],
            true,
        )
        .count(),
    )
}

fn replace_paragraph<Renderer>(
    renderer: &Renderer,
    state: &mut State<Renderer::Paragraph>,
    layout: Layout<'_>,
    value: &Value,
    font: Option<Renderer::Font>,
    text_size: Option<Pixels>,
    line_height: text::LineHeight,
) where
    Renderer: text::Renderer,
{
    let font = font.unwrap_or_else(|| renderer.default_font());
    let text_size = text_size.unwrap_or_else(|| renderer.default_size());

    let mut children_layout = layout.children();
    let text_bounds = children_layout.next().unwrap().bounds();

    state.value = Renderer::Paragraph::with_text(Text {
        font,
        line_height,
        content: &value.to_string(),
        bounds: Size::new(f32::INFINITY, text_bounds.height),
        size: text_size,
        horizontal_alignment: alignment::Horizontal::Left,
        vertical_alignment: alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
    });
}

const CURSOR_BLINK_INTERVAL_MILLIS: u128 = 500;
