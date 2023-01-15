use super::app::Application;
use anyhow::Result;
use crossterm::event::{poll, read, Event, KeyCode, KeyEvent};
use std::time::Duration;

#[derive(Default)]
pub struct InputHandler {
    input_buffer: String,
    input_search_history: InputSearchHistory,
    input_state: InputState,
}


#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum InputState {
    #[default]
    Valid,
    Incomplete(String),
    Invalid(String),
}

impl InputHandler {
    pub fn handle_input<A: Application>(&mut self, app: &mut A) -> Result<()> {
        let poll_timeout = if app.is_searching() {
            Duration::from_millis(1)
        } else {
            Duration::from_millis(100)
        };

        if poll(poll_timeout)? {
            let read_event = read()?;
            if let Event::Key(key_event) = read_event {
                match key_event {
                    KeyEvent {
                        code: KeyCode::Char(character),
                        ..
                    } => self.handle_char_input(character, app),
                    _ => self.handle_non_char_input(key_event.code, app),
                }
            }
        }

        Ok(())
    }

    fn handle_char_input<A: Application>(&mut self, character: char, app: &mut A) {
        self.input_buffer.push(character);
        let consume_buffer_and_execute = |buffer: &mut String, op: &mut dyn FnMut()| {
            buffer.clear();
            op();
        };

        if app.is_input_searching() {
            self.input_state = InputState::Incomplete(self.input_buffer.clone());
        } else {
            self.input_state = InputState::Valid;
            match self.input_buffer.as_str() {
                "j" => consume_buffer_and_execute(&mut self.input_buffer, &mut || app.on_next_match()),
                "k" => {
                    consume_buffer_and_execute(&mut self.input_buffer, &mut || app.on_previous_match())
                }
                "l" => consume_buffer_and_execute(&mut self.input_buffer, &mut || app.on_next_file()),
                "h" => {
                    consume_buffer_and_execute(&mut self.input_buffer, &mut || app.on_previous_file())
                }
                "gg" => consume_buffer_and_execute(&mut self.input_buffer, &mut || app.on_top()),
                "G" => consume_buffer_and_execute(&mut self.input_buffer, &mut || app.on_bottom()),
                "dd" => consume_buffer_and_execute(&mut self.input_buffer, &mut || {
                    app.on_remove_current_entry()
                }),
                "dw" => consume_buffer_and_execute(&mut self.input_buffer, &mut || {
                    app.on_remove_current_file()
                }),
                "v" => consume_buffer_and_execute(&mut self.input_buffer, &mut || {
                    app.on_toggle_context_viewer_vertical()
                }),
                "s" => consume_buffer_and_execute(&mut self.input_buffer, &mut || {
                    app.on_toggle_context_viewer_horizontal()
                }),
                "q" => consume_buffer_and_execute(&mut self.input_buffer, &mut || app.on_exit()),
                "g" => self.input_state = InputState::Incomplete("g…".into()),
                "d" => self.input_state = InputState::Incomplete("d…".into()),
                buf => {
                    self.input_state = InputState::Invalid(buf.into());
                    self.input_buffer.clear();
                }
            }
        }

    }

    fn handle_non_char_input<A: Application>(&mut self, key_code: KeyCode, app: &mut A) {
        if app.is_input_searching() {
            match key_code {
                KeyCode::Enter => {
                    self.input_search_history.push(self.input_buffer.clone());
                    self.input_state = InputState::Valid;
                    app.on_search();
                },
                KeyCode::Down => {
                    let history = self.input_search_history.next();
                    self.input_buffer = history.to_owned();
                    self.input_state = InputState::Incomplete(self.input_buffer.clone());
                },
                KeyCode::Up => {
                    let history = self.input_search_history.pre();
                    self.input_buffer = history.to_owned();
                    self.input_state = InputState::Incomplete(self.input_buffer.clone());
                },
                KeyCode::Backspace => {
                    let _ =  self.input_buffer.pop();
                    self.input_state = InputState::Incomplete(self.input_buffer.clone());
                },
                _ => ()
            }
        } else {
            self.input_buffer.clear();
            self.input_state = InputState::Valid;
            match key_code {
                KeyCode::Down => app.on_next_match(),
                KeyCode::Up => app.on_previous_match(),
                KeyCode::Right | KeyCode::PageDown => app.on_next_file(),
                KeyCode::Left | KeyCode::PageUp => app.on_previous_file(),
                KeyCode::Home => app.on_top(),
                KeyCode::End => app.on_bottom(),
                KeyCode::Delete => app.on_remove_current_entry(),
                KeyCode::Enter => {
                        app.on_open_file();
                },
                KeyCode::F(5) => app.on_search(),
                KeyCode::F(1) => app.on_show_help(),
                KeyCode::F(2) => {
                    self.input_state = InputState::Incomplete(self.input_buffer.clone());
                    app.on_input_search();
                },
                KeyCode::Esc => {
                    if matches!(self.input_state, InputState::Valid)
                        || matches!(self.input_state, InputState::Invalid(_))
                    {
                        app.on_exit();
                    }
                }
                _ => (),
            }
        }
    }

    pub fn get_state(&self) -> &InputState {
        &self.input_state
    }
}

pub struct InputSearchHistory {
    history:Vec<String>,
    curse:usize,
}

impl Default for InputSearchHistory{
    fn default() -> Self {
        Self {
            history:vec!["没有记录了".to_owned(),],
            curse:0,
        }
    }
}

impl InputSearchHistory {

    pub fn push(&mut self,record:String) {
        if !self.history.contains(&record) {
            self.history.insert(0,record);
        } else {
            let pos = self.history.binary_search(&record).unwrap();
            let select = self.history.remove(pos);
            self.history.insert(0, select);
        }
    }

    pub fn get(&self) -> &str {
        self.history.get(self.curse).unwrap()
    }

    pub fn pre(&mut self) -> &str{
        if self.curse > 0 {
            self.curse -= 1;
        }
        self.get()
    }

    pub fn next(&mut self) -> &str{
        let len = self.history.len();
        if self.curse < len - 1 {
            self.curse += 1;
        }
        self.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::app::MockApplication;
    use crossterm::event::KeyCode::{Char, Esc};
    use test_case::test_case;

    fn handle_key<A: Application>(key_code: KeyCode, app: &mut A) {
        let mut input_handler = InputHandler::default();
        handle(&mut input_handler, key_code, app);
    }

    fn handle_key_series<A: Application>(key_codes: &[KeyCode], app: &mut A) {
        let mut input_handler = InputHandler::default();
        for key_code in key_codes {
            handle(&mut input_handler, *key_code, app);
        }
    }

    fn handle<A: Application>(input_handler: &mut InputHandler, key_code: KeyCode, app: &mut A) {
        match key_code {
            Char(character) => input_handler.handle_char_input(character, app),
            _ => input_handler.handle_non_char_input(key_code, app),
        }
    }

    #[test_case(KeyCode::Down; "down")]
    #[test_case(Char('j'); "j")]
    fn next_match(key_code: KeyCode) {
        let mut app_mock = MockApplication::default();
        app_mock.expect_on_next_match().once().return_const(());
        handle_key(key_code, &mut app_mock);
    }

    #[test_case(KeyCode::Up; "up")]
    #[test_case(Char('k'); "k")]
    fn previous_match(key_code: KeyCode) {
        let mut app_mock = MockApplication::default();
        app_mock.expect_on_previous_match().once().return_const(());
        handle_key(key_code, &mut app_mock);
    }

    #[test_case(KeyCode::Right; "right")]
    #[test_case(KeyCode::PageDown; "page down")]
    #[test_case(Char('l'); "l")]
    fn next_file(key_code: KeyCode) {
        let mut app_mock = MockApplication::default();
        app_mock.expect_on_next_file().once().return_const(());
        handle_key(key_code, &mut app_mock);
    }

    #[test_case(KeyCode::Left; "left")]
    #[test_case(KeyCode::PageUp; "page up")]
    #[test_case(Char('h'); "h")]
    fn previous_file(key_code: KeyCode) {
        let mut app_mock = MockApplication::default();
        app_mock.expect_on_previous_file().once().return_const(());
        handle_key(key_code, &mut app_mock);
    }

    #[test_case(&[KeyCode::Home]; "home")]
    #[test_case(&[Char('g'), Char('g')]; "gg")]
    fn top(key_codes: &[KeyCode]) {
        let mut app_mock = MockApplication::default();
        app_mock.expect_on_top().once().return_const(());
        handle_key_series(key_codes, &mut app_mock);
    }

    #[test_case(KeyCode::End; "end")]
    #[test_case(Char('G'); "G")]
    fn bottom(key_code: KeyCode) {
        let mut app_mock = MockApplication::default();
        app_mock.expect_on_bottom().once().return_const(());
        handle_key(key_code, &mut app_mock);
    }

    #[test_case(&[KeyCode::Delete]; "delete")]
    #[test_case(&[Char('d'), Char('d')]; "dd")]
    #[test_case(&[Char('g'), Char('d'), Char('w'), Char('d'), Char('d')]; "gdwdd")]
    fn remove_current_entry(key_codes: &[KeyCode]) {
        let mut app_mock = MockApplication::default();
        app_mock
            .expect_on_remove_current_entry()
            .once()
            .return_const(());
        handle_key_series(key_codes, &mut app_mock);
    }

    #[test_case(&[Char('d'), Char('w')]; "dw")]
    #[test_case(&[Char('w'), Char('d'), Char('w')]; "wdw")]
    fn remove_current_file(key_codes: &[KeyCode]) {
        let mut app_mock = MockApplication::default();
        app_mock
            .expect_on_remove_current_file()
            .once()
            .return_const(());
        handle_key_series(key_codes, &mut app_mock);
    }

    #[test]
    fn toggle_vertical_context_viewer() {
        let mut app_mock = MockApplication::default();
        app_mock
            .expect_on_toggle_context_viewer_vertical()
            .once()
            .return_const(());
        handle_key(KeyCode::Char('v'), &mut app_mock);
    }

    #[test]
    fn toggle_horizontal_context_viewer() {
        let mut app_mock = MockApplication::default();
        app_mock
            .expect_on_toggle_context_viewer_horizontal()
            .once()
            .return_const(());
        handle_key(KeyCode::Char('s'), &mut app_mock);
    }

    #[test]
    fn open_file() {
        let mut app_mock = MockApplication::default();
        app_mock.expect_on_open_file().once().return_const(());
        handle_key(KeyCode::Enter, &mut app_mock);
    }

    #[test]
    fn search() {
        let mut app_mock = MockApplication::default();
        app_mock.expect_on_search().once().return_const(());
        handle_key(KeyCode::F(5), &mut app_mock);
    }

    #[test_case(&[Char('q')]; "q")]
    #[test_case(&[Esc]; "empty input state")]
    #[test_case(&[Char('a'), Char('b'), Esc]; "invalid input state")]
    #[test_case(&[Char('d'), Esc, Esc]; "clear incomplete state first")]
    fn exit(key_codes: &[KeyCode]) {
        let mut app_mock = MockApplication::default();
        app_mock.expect_on_exit().once().return_const(());
        handle_key_series(key_codes, &mut app_mock);
    }
}
