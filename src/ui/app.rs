use super::{
    cmd_parse::SearchCmd,
    context_viewer::ContextViewerState,
    editor::Editor,
    input_handler::{InputHandler, InputState},
    result_list::ResultList,
    scroll_offset_list::{List, ListItem, ListState, ScrollOffset},
    soft_warp::{SoftWrapper, SplitPosType},
    theme::Theme,
};

use crate::{
    file_entry::EntryType,
    ig::{Ig, SearchConfig},
};
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::path::PathBuf;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame, Terminal,
};

#[derive(Default, PartialEq, Eq)]
enum BottomBarState {
    Input,
    #[default]
    Normal,
}

pub struct App {
    ig: Ig,
    result_list: ResultList,
    result_list_state: ListState,
    context_viewer_state: ContextViewerState,
    bottom_bar_state: BottomBarState,
    theme: Box<dyn Theme>,
    show_help: bool,
}

impl App {
    pub fn new(config: SearchConfig, editor: Editor, theme: Box<dyn Theme>) -> Self {
        Self {
            ig: Ig::new(config, editor),
            result_list: ResultList::default(),
            result_list_state: ListState::default(),
            bottom_bar_state: BottomBarState::default(),
            context_viewer_state: ContextViewerState::default(),
            theme,
            show_help: false,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let mut input_handler = InputHandler::default();
        self.ig.search(&mut self.result_list);

        loop {
            let backend = CrosstermBackend::new(std::io::stdout());
            let mut terminal = Terminal::new(backend)?;
            terminal.hide_cursor()?;

            enable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                // NOTE: This is necessary due to upstream `crossterm` requiring that we "enable"
                // mouse handling first, which saves some state that necessary for _disabling_
                // mouse events.
                EnableMouseCapture,
                EnterAlternateScreen,
                DisableMouseCapture
            )?;

            while self.ig.is_searching() || self.ig.is_idle() {
                terminal.draw(|f| Self::draw(f, self, &input_handler))?;

                if let Some(entry) = self.ig.handle_searcher_event() {
                    self.result_list.add_entry(entry);
                }
                input_handler.handle_input(self)?;

                if let Some((file_name, _)) = self.result_list.get_selected_entry() {
                    if let Some(context_viewer) = self.context_viewer_state.viewer() {
                        context_viewer.highlight_file_if_needed(
                            &PathBuf::from(file_name),
                            self.theme.as_ref(),
                        );
                    }
                }
            }

            self.ig
                .open_file_if_requested(self.result_list.get_selected_entry());

            if self.ig.exit_requested() {
                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                disable_raw_mode()?;
                break;
            }
        }

        Ok(())
    }

    fn draw(
        frame: &mut Frame<CrosstermBackend<std::io::Stdout>>,
        app: &mut App,
        input_handler: &InputHandler,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
            .split(frame.size());

        let (view_area, bottom_bar_area) = (chunks[0], chunks[1]);
        let (list_area, cv_area) = match &app.context_viewer_state {
            ContextViewerState::None => (view_area, None),
            ContextViewerState::Vertical(_) => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(view_area);

                let (left, right) = (chunks[0], chunks[1]);
                (left, Some(right))
            }
            ContextViewerState::Horizontal(_) => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                    .split(view_area);

                let (top, bottom) = (chunks[0], chunks[1]);
                (top, Some(bottom))
            }
        };

        Self::draw_list(frame, list_area, app);
        if let Some(cv_area) = cv_area {
            Self::draw_context_viewer(frame, cv_area, app);
        }
        Self::draw_bottom_bar(frame, bottom_bar_area, app, input_handler);

        if app.show_help {
            let help_area = Self::centered_rect(50, 70, view_area);
            draw_help(frame, help_area);
        }
    }

    fn draw_list(frame: &mut Frame<CrosstermBackend<std::io::Stdout>>, area: Rect, app: &mut App) {

        let mut skip = app.result_list_state.get_skip();
        let entries = app.result_list.entries();
        let end = entries.len().min(skip+200);
        if skip > end {
            skip = end
        }
        let files_list: Vec<ListItem> = entries[skip..end]
            .iter()
            // .iter()
            .map(|e| 
                match e {
                    EntryType::Header(h) => {
                        let h = h.trim_start_matches("./");
                        ListItem::new(Span::styled(h, app.theme.file_path_color()))
                    }
                    EntryType::Match(n, t, offsets) => {
                        let line_number =
                            Span::styled(format!(" {n}: "), app.theme.line_number_color());

                        let mut line: Vec<Spans> = Vec::new();

                        let max_width = area.width as usize;
                        let mut current_position = 0;
                        let soft_wrapper = SoftWrapper::new(max_width, offsets, t);

                        let mut match_flag = false;
                        let mut spans = vec![line_number];

                        for split_pos in soft_wrapper.positions {
                            let sty = if match_flag {
                                app.theme.match_color()
                            } else {
                                app.theme.list_font_color()
                            };
                            match split_pos {
                                SplitPosType::Crlf(x) => {
                                    let newline_span =
                                        Span::styled(&t[current_position..x], sty);
                                    spans.push(newline_span);
                                    line.push(Spans::from(spans.clone()));
                                    spans.clear();
                                    current_position = x;
                                }
                                SplitPosType::MatchStart(x) => {
                                    let before_match =
                                        Span::styled(&t[current_position..x], sty);
                                    spans.push(before_match);
                                    current_position = x;
                                    match_flag = true;
                                }
                                SplitPosType::MatchEnd(x) => {
                                    let actual_match_line =
                                        Span::styled(&t[current_position..x], sty);
                                    spans.push(actual_match_line);
                                    current_position = x;
                                    match_flag = false;
                                }
                            }
                        }
                        ListItem::new(line)
                }
            }).collect();

        let list_widget = List::new(files_list)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(app.theme.background_color())
            .highlight_style(Style::default().bg(app.theme.highlight_color()))
            .scroll_offset(ScrollOffset::default().top(1).bottom(0));

        app.result_list_state
            .select(app.result_list.get_state().selected());
        frame.render_stateful_widget(list_widget, area, &mut app.result_list_state);
    }

    fn draw_context_viewer(
        frame: &mut Frame<CrosstermBackend<std::io::Stdout>>,
        area: Rect,
        app: &mut App,
    ) {
        let block_widget = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        if let Some((_, line_number)) = app.result_list.get_selected_entry() {
            let height = area.height as u64;
            let first_line_index = line_number.saturating_sub(height / 2);

            let paragraph_widget =
                Paragraph::new(app.context_viewer_state.viewer().unwrap().get_styled_spans(
                    first_line_index as usize,
                    height as usize,
                    area.width as usize,
                    line_number as usize,
                    app.theme.as_ref(),
                ))
                .block(block_widget);

            frame.render_widget(paragraph_widget, area);
        } else {
            frame.render_widget(block_widget, area);
        }
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_y) / 2),
                    Constraint::Min(3),
                    Constraint::Percentage((100 - percent_y) / 2),
                ]
                .as_ref(),
            )
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_x) / 2),
                    Constraint::Percentage(percent_x),
                    Constraint::Percentage((100 - percent_x) / 2),
                ]
                .as_ref(),
            )
            .split(popup_layout[1])[1]
    }

    fn draw_bottom_bar(
        frame: &mut Frame<CrosstermBackend<std::io::Stdout>>,
        area: Rect,
        app: &mut App,
        input_handler: &InputHandler,
    ) {
        match app.bottom_bar_state {
            BottomBarState::Input => draw_bottom_bar_input(app, input_handler, area, frame),
            BottomBarState::Normal => draw_bottom_bar_normal(app, input_handler, area, frame),
        }
    }
}

fn draw_bottom_bar_input(
    app: &mut App,
    input_handler: &InputHandler,
    area: Rect,
    frame: &mut Frame<CrosstermBackend<std::io::Stdout>>,
) {
    let app_status_text = "输入";
    let app_status_style = app.theme.searching_state_style();
    let app_status = Span::styled(app_status_text, app_status_style);

    let (current_input_content, current_input_color) = match input_handler.get_state() {
        InputState::Valid => (String::default(), app.theme.bottom_bar_font_color()),
        InputState::Incomplete(input) => (input.to_owned(), app.theme.bottom_bar_font_color()),
        InputState::Invalid(input) => (input.to_owned(), app.theme.invalid_input_color()),
    };
    let current_input = Span::styled(
        current_input_content,
        Style::default()
            .bg(app.theme.bottom_bar_color())
            .fg(current_input_color),
    );

    let hsplit = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(12), Constraint::Min(1)].as_ref())
        .split(area);
    frame.render_widget(
        Paragraph::new(app_status)
            .style(Style::default().bg(app_status_style.bg.expect("背景色没有设置")))
            .alignment(Alignment::Center),
        hsplit[0],
    );
    frame.render_widget(
        Paragraph::new(current_input)
            .style(app.theme.bottom_bar_style())
            .alignment(Alignment::Left),
        hsplit[1],
    );
}

fn draw_bottom_bar_normal(
    app: &mut App,
    input_handler: &InputHandler,
    area: Rect,
    frame: &mut Frame<CrosstermBackend<std::io::Stdout>>,
) {
    let current_match_index = app.result_list.get_current_match_index();
    let (app_status_text, app_status_style) = if app.ig.is_searching() {
        ("搜索中", app.theme.searching_state_style())
    } else {
        ("完成", app.theme.finished_state_style())
    };
    let app_status = Span::styled(app_status_text, app_status_style);
    let search_result = Span::raw(if app.ig.is_searching() {
        "".into()
    } else {
        let total_no_of_matches = app.result_list.get_total_number_of_matches();
        if total_no_of_matches == 0 {
            "没有找到匹配项".into()
        } else {
            let no_of_files = app.result_list.get_total_number_of_file_entries();

            let matches_str = "匹配";
            let files_str = "文件";

            let filtered_count = app.result_list.get_filtered_matches_count();
            let filtered_str = if filtered_count != 0 {
                format!(" (过滤掉{filtered_count} 个)")
            } else {
                String::default()
            };

            format!(
                " {total_no_of_matches}个{matches_str},在{no_of_files}个{files_str}中{filtered_str}."
            )
        }
    });
    let (current_input_content, current_input_color) = match input_handler.get_state() {
        InputState::Valid => (String::default(), app.theme.bottom_bar_font_color()),
        InputState::Incomplete(input) => (input.to_owned(), app.theme.bottom_bar_font_color()),
        InputState::Invalid(input) => (input.to_owned(), app.theme.invalid_input_color()),
    };
    let current_input = Span::styled(
        current_input_content,
        Style::default()
            .bg(app.theme.bottom_bar_color())
            .fg(current_input_color),
    );
    let current_no_of_matches = app.result_list.get_current_number_of_matches();
    let selected_info_text = {
        let width = current_no_of_matches.to_string().len();
        format!(" | {current_match_index: >width$}/{current_no_of_matches} ")
    };
    let selected_info_length = selected_info_text.len();
    let selected_info = Span::styled(selected_info_text, app.theme.bottom_bar_style());
    let hsplit = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Length(10),
                Constraint::Min(1),
                Constraint::Length(5),
                Constraint::Length(selected_info_length as u16),
            ]
            .as_ref(),
        )
        .split(area);
    frame.render_widget(
        Paragraph::new(app_status)
            .style(Style::default().bg(app_status_style.bg.expect("背景色没有设置")))
            .alignment(Alignment::Center),
        hsplit[0],
    );
    frame.render_widget(
        Paragraph::new(search_result)
            .style(app.theme.bottom_bar_style())
            .alignment(Alignment::Left),
        hsplit[1],
    );
    frame.render_widget(
        Paragraph::new(current_input)
            .style(app.theme.bottom_bar_style())
            .alignment(Alignment::Right),
        hsplit[2],
    );
    frame.render_widget(
        Paragraph::new(selected_info)
            .style(app.theme.bottom_bar_style())
            .alignment(Alignment::Right),
        hsplit[3],
    );
}

fn draw_help(frame: &mut Frame<CrosstermBackend<std::io::Stdout>>, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("帮助");
    let block_l = Block::default()
        .borders(Borders::RIGHT)
        .border_type(BorderType::Rounded)
        .title("按键");
    let block_r = Block::default()
        .border_type(BorderType::Rounded)
        .title("说明");
    frame.render_widget(Clear, area);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let vsplit = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)].as_ref())
        .split(inner_area);

    let style1 = Style::default()
        .add_modifier(Modifier::ITALIC)
        .fg(Color::Green);
    let style2 = Style::default()
        .add_modifier(Modifier::BOLD)
        .fg(Color::Yellow);

    let l_help = vec![
        help_item("h\\j\\k\\l", style1),
        help_item("gg\\Shift+g", style1),
        help_item("<num>g", style1),
        help_item("+/-<num>g", style1),
        help_item("dd\\df", style1),
        help_item("F1\\z", style1),
        help_item("F5", style1),
        help_item("F2\\:", style1),
        help_item("Enter", style1),
        help_item(" |", style1),
        help_item(" |", style1),
        help_item("v/s", style1),
    ];

    let r_help = vec![
        help_item("上\\下\\上个文件\\下个文件导航", style2),
        help_item("跳到开头\\跳到末尾", style2),
        help_item("跳到指定<num>行数", style2),
        help_item("跳到指定相对增加/减少<num>行数", style2),
        help_item("删除当前行\\删除当前文件(只删除显示)", style2),
        help_item("打开帮助", style2),
        help_item("刷新", style2),
        help_item("输入搜索条件", style2),
        help_item("浏览模式下:打开当前行的文件", style2),
        help_item(
            "    默认用vim,--editor或者环境变量`$IGREP_EDITOR`或`$EDITOR`配置",
            style2,
        ),
        help_item("输入模式下:搜索输入的条件", style2),
        help_item("打开垂直\\水平预览窗口", style2),
    ];

    let helpk1 = Paragraph::new(Text::from(l_help))
        .alignment(Alignment::Left)
        .block(block_l);
    let helpv1 = Paragraph::new(Text::from(r_help))
        .alignment(Alignment::Left)
        .block(block_r);

    frame.render_widget(helpk1, vsplit[0]);
    frame.render_widget(helpv1, vsplit[1]);
}

fn help_item<'a>(action: &'a str, sty: Style) -> Spans<'a> {
    let ac_span = Span::styled(action, sty);
    Spans::from(vec![ac_span])
}

impl Application for App {
    fn is_searching(&self) -> bool {
        self.ig.is_searching()
    }

    fn on_next_match(&mut self) {
        self.result_list.next_match();
    }

    fn on_previous_match(&mut self) {
        self.result_list.previous_match();
    }

    fn on_next_file(&mut self) {
        self.result_list.next_file();
    }

    fn on_previous_file(&mut self) {
        self.result_list.previous_file();
    }

    fn on_top(&mut self) {
        self.result_list.top();
    }

    fn on_bottom(&mut self) {
        self.result_list.bottom();
        self.result_list_state.set_offset(0);           
    }

    fn on_remove_current_entry(&mut self) {
        self.result_list.remove_current_entry();
    }

    fn on_remove_current_file(&mut self) {
        self.result_list.remove_current_file();
    }

    fn on_toggle_context_viewer_vertical(&mut self) {
        self.context_viewer_state.toggle_vertical();
    }

    fn on_toggle_context_viewer_horizontal(&mut self) {
        self.context_viewer_state.toggle_horizontal();
    }

    fn on_open_file(&mut self) {
        self.ig.open_file();
    }

    fn on_search(&mut self) {
        self.bottom_bar_state = BottomBarState::Normal;
        self.result_list = ResultList::default();
        self.result_list_state = ListState::default();
        self.ig.search(&mut self.result_list);
    }

    fn on_exit(&mut self) {
        match self.bottom_bar_state {
            BottomBarState::Normal => self.ig.exit(),
            _ => self.bottom_bar_state = BottomBarState::Normal,
        }
    }

    fn on_show_help(&mut self) {
        self.show_help = !self.show_help;
    }

    fn on_input_search(&mut self) {
        self.bottom_bar_state = BottomBarState::Input;
    }

    fn is_input_searching(&self) -> bool {
        self.bottom_bar_state == BottomBarState::Input
    }

    fn is_normal(&self) -> bool {
        self.bottom_bar_state == BottomBarState::Normal
    }

    fn on_to_normal(&mut self) {
        self.bottom_bar_state = BottomBarState::Normal;
    }

    fn update_cmd(&mut self, cmd: SearchCmd) {
        self.ig.update_cmd(cmd);
    }

    fn jump_to(&mut self, line: usize) {
        self.result_list.jump_to(line);
    }

    fn jump_to_relative(&mut self, delta: i32) {
        self.result_list.jump_to_relative(delta);
    }
}

#[cfg_attr(test, mockall::automock)]
pub trait Application {
    fn update_cmd(&mut self, cmd: SearchCmd);
    fn is_searching(&self) -> bool;
    fn is_input_searching(&self) -> bool;
    fn is_normal(&self) -> bool;
    fn on_next_match(&mut self);
    fn on_previous_match(&mut self);
    fn on_next_file(&mut self);
    fn on_previous_file(&mut self);
    fn on_top(&mut self);
    fn on_bottom(&mut self);
    fn on_remove_current_entry(&mut self);
    fn on_remove_current_file(&mut self);
    fn on_toggle_context_viewer_vertical(&mut self);
    fn on_toggle_context_viewer_horizontal(&mut self);
    fn on_open_file(&mut self);
    fn on_search(&mut self);
    fn on_exit(&mut self);
    fn on_show_help(&mut self);
    fn on_input_search(&mut self);
    fn on_to_normal(&mut self);
    fn jump_to(&mut self, line: usize);
    fn jump_to_relative(&mut self, delta: i32);
}
