use proc_macro::TokenStream;
use proc_macro2::TokenStream as TS2;

use quote::quote;
use quote::ToTokens;

use syn::parse::{Parse, ParseStream};
use syn::parse_str;
use syn::punctuated::Punctuated;
use syn::token::Semi;
use syn::{parse_macro_input, Token};
use syn::{Expr, ExprClosure, Ident, Lit, Result, Variant};

struct CustomEvent {
    key_code: Expr,
    modifiers: Lit,
    input_action: Variant,
    func: ExprClosure,
}

impl Parse for CustomEvent {
    fn parse(input: ParseStream) -> Result<Self> {
        let key_code = Expr::parse(input)?;
        eprintln!("stuck on line: {}", line!());
        println!("{}", key_code.to_token_stream().to_string());

        _ = <Token![,]>::parse(input)?;
        eprintln!("stuck on line: {}", line!());
        let modifiers = Lit::parse(input)?;
        eprintln!("stuck on line: {}", line!());

        _ = <Token![,]>::parse(input)?;
        eprintln!("stuck on line: {}", line!());
        let input_action = Variant::parse(input)?;
        eprintln!("stuck on line: {}", line!());

        _ = <Token![,]>::parse(input)?;
        eprintln!("passed line: {}", line!());
        let func = ExprClosure::parse(input)?;
        eprintln!("stuck on line: {}", line!());

        Ok(CustomEvent {
            key_code,
            modifiers,
            input_action,
            func,
        })
    }
}

struct MacroInput {
    punc: Punctuated<CustomEvent, Semi>,
}

impl MacroInput {
    fn unpack(self) -> Punctuated<CustomEvent, Semi> {
        self.punc
    }
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(MacroInput {
            punc: Punctuated::parse_terminated_with(input, CustomEvent::parse).unwrap(),
        })
    }
}

#[proc_macro]
pub fn ragout_custom_events(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as MacroInput);
    let ast = ast.unpack();

    let (input_actions, kbd_events, write_arms) = punctuated_converge(ast);

    let input_actions = input_actions_enum(input_actions);
    let kbd_events = kbd_events_fn(kbd_events);
    let input_write = input_write(write_arms);

    TokenStream::from(quote! {
        use std::io::{StdoutLock, Write};

        pub fn init(
            prompt: &str,
            alt_screen: bool,
        ) -> (std::io::StdoutLock<'static>, Input, History, String) {
            _ = enable_raw_mode();

            let mut sol = std::io::stdout().lock();

            if alt_screen {
                _ = sol.write(b"\x1b[?1049h");
                _ = sol.write(b"\x1b[1;1f");
            }

            let i = Input::new(prompt, alt_screen);
            i.write_prompt(&mut sol);

            (sol, i, History::new(), String::new())
        }

        pub fn run(
            input: &mut Input,
            history: &mut History,
            stdout: &mut std::io::StdoutLock<'static>,
            user_input: &mut String,
        ) -> String {
            let cmd = keyboard();
            cmd.execute(input, history, stdout, user_input);

            user_input.drain(..).collect::<String>()
        }

        #[derive(Debug)]
        #input_actions

        pub enum Command {
            InputAction(InputAction),
            // Script,
            Exit(i32),
            None,
        }

        impl Command {
            fn execute(&self, i: &mut Input, h: &mut History, sol: &mut StdoutLock<'_>, ui: &mut String) {
                match self {
                    Command::InputAction(ia) => i.write(h, ia, sol, ui),
                    Command::Exit(code) => {
                        if i.alt_screen {
                            _ = sol.write(b"\x1b[?1049l");
                        }
                        Command::exit(*code)
                    }
                    // Command::Script => Command::script(&h, &ui),
                    Command::None => (),
                }
            }

            fn exit(code: i32) {
                std::process::exit(code);
            }

            // fn script(h: &History, name: &str) {
            //     let script = h
            //         .log
            //         .iter()
            //         .map(|vec| vec.iter().collect::<String>())
            //         .filter(|l| &l[..7] != "script ")
            //         .fold(String::new(), |acc, x| acc + &x + "\r\n");
            //
            //     std::fs::write(
            //         "resources/app/scripts/".to_string() + name + ".txt",
            //         script.into_bytes(),
            //     )
            //     .unwrap()
            // }
        }

        use crossterm::event::{read as kbd_read, Event, KeyCode, KeyEvent, KeyModifiers};
        use crossterm::terminal::enable_raw_mode;

        #kbd_events

        fn keyboard() -> Command {
            match kbd_read() {
                Ok(Event::Key(key_event)) => kbd_event(key_event),
                Err(e) => {
                    eprintln!("read error\n{:?}", e);
                    Command::None
                }
                _ => Command::None,
            }
        }

        #[derive(Debug)]
        pub struct Input {
            pub values: Vec<char>,
            pub cursor: usize,
            #[cfg(debug_assertions)]
            pub debug_log: std::fs::File,
            pub prompt: String,
            pub alt_screen: bool,
        }

        impl Input {
            pub fn new(prompt: &str, alt_screen: bool) -> Self {
                let mut i = Self {
                    #[cfg(debug_assertions)]
                    debug_log: std::fs::File::create("resources/logs/terminal/input").unwrap_or_else(
                        |_| {
                            std::fs::create_dir_all("resources/logs/terminal").unwrap();
                            std::fs::File::create("resources/logs/terminal/input").unwrap()
                        },
                    ),
                    values: Vec::new(),
                    cursor: 0,
                    prompt: prompt.to_owned(),
                    alt_screen,
                };
                #[cfg(debug_assertions)]
                i.log(&InputAction::New);

                i
            }

            pub fn put_char(&mut self, c: char) {
                match self.values.is_empty() {
                    true => {
                        self.values.push(c);
                        self.cursor += 1;
                    }
                    false => match self.cursor == self.values.len() {
                        true => {
                            self.values.push(c);
                            self.cursor += 1;
                        }

                        false => {
                            self.values.insert(self.cursor, c);
                            self.cursor += 1;
                        }
                    },
                }
            }

            // PRIORITY HIGH:
            // TODO: add prompt (wip)
            // TODO: add documentation for the whole crate (branch docs)
            //

            // TODO: shift cr registers input and sends it to command; aka multi line input
            // WARN: do NOT touch this Input implementation
            // the fns other than write are not to be touched

            pub fn cr_lf(&mut self, h: &mut History, user_input: &mut String) {
                h.push(self.values.to_vec());
                *user_input = self.values.drain(..).collect::<String>();
                self.cursor = 0;
            }

            pub fn backspace(&mut self) {
                if self.values.is_empty() || self.cursor == 0 {
                    return;
                }
                if self.cursor > 0 {
                    self.values.remove(self.cursor - 1);
                    self.cursor -= 1;
                }
            }

            pub fn to_the_right(&mut self) -> bool {
                if self.values.is_empty() || self.cursor == self.values.len() {
                    return false;
                }
                self.cursor += 1;

                true
            }

            pub fn to_the_left(&mut self) -> bool {
                if self.values.is_empty() || self.cursor == 0 {
                    return false;
                }
                self.cursor -= 1;

                true
            }

            pub fn to_end(&mut self) -> usize {
                let diff = self.values.len() - self.cursor;
                if diff > 0 {
                    self.cursor = self.values.len();
                }

                diff
            }

            pub fn to_home(&mut self) -> bool {
                if self.cursor == 0 {
                    return false;
                }
                self.cursor = 0;

                true
            }

            pub fn clear_line(&mut self) {
                self.cursor = 0;
                self.values.clear();
            }

            pub fn clear_right(&mut self) {
                for _ in self.cursor..self.values.len() {
                    self.values.pop();
                }
            }

            pub fn clear_left(&mut self) {
                for _ in 0..self.cursor {
                    self.values.remove(0);
                }
                self.cursor = 0;
            }

            const STOPPERS: [char; 11] = ['/', ' ', '-', '_', ',', '"', '\'', ';', ':', '.', ','];

            pub fn to_right_jump(&mut self) {
                if self.cursor == self.values.len() {
                    return;
                }

                match self.values[if self.cursor + 1 < self.values.len() {
                    self.cursor + 1
                } else {
                    self.cursor
                }] == ' '
                {
                    true => {
                        while self.cursor + 1 < self.values.len() && self.values[self.cursor + 1] == ' ' {
                            self.cursor += 1;
                        }
                    }
                    false => {
                        while self.cursor + 1 < self.values.len()
                            && !Self::STOPPERS.contains(&self.values[self.cursor + 1])
                        {
                            self.cursor += 1;
                        }
                        self.cursor += 1;
                    }
                }
            }

            pub fn to_left_jump(&mut self) {
                if self.cursor == 0 {
                    return;
                }

                match self.values[self.cursor - 1] == ' ' {
                    true => {
                        while self.cursor > 0 && self.values[self.cursor - 1] == ' ' {
                            self.cursor -= 1;
                        }
                    }
                    false => {
                        while self.cursor > 1 && !Self::STOPPERS.contains(&self.values[self.cursor - 1]) {
                            self.cursor -= 1;
                        }
                        self.cursor -= 1;
                    }
                }
            }

            #[cfg(debug_assertions)]
            pub fn log(&mut self, method: &InputAction) {
                self.debug_log
                    .write_all(
                        format!(
                            "[LOG::{:?} - {:?}] {{ values[{:?}] = '{:?}' }} - {:?}\r\n",
                            method,
                            std::process::Command::new("date")
                                .arg("+\"%H:%M:%S:%N\"")
                                .output()
                                .expect("couldnt get time from linux command 'date'")
                                .stdout
                                .into_iter()
                                .map(|u| u as char)
                                .collect::<String>()
                                .replacen("\"", "", 2)
                                .trim_end_matches("\n"),
                            if self.cursor == 0 {
                                None
                            } else {
                                Some(self.cursor - 1)
                            },
                            if self.values.is_empty() || self.cursor == 0 {
                                None
                            } else {
                                Some(self.values[self.cursor - 1])
                            },
                            self.values,
                        )
                        .as_bytes(),
                    )
                    .unwrap();
            }
        }

        impl Input {
            #input_write
        }

        // NOTE: the cursor in both input and history does not point to the item it's on,
        // but is alawys pointing at the item to the left
        // basically cursor = 0 points at nothing and cursor = 4 points at eg. input[3]
        // this logic is implemented in the first impl of Input

        #[derive(Debug)]
        pub struct History {
            #[cfg(debug_assertions)]
            pub debug_log: std::fs::File,
            pub log: Vec<Vec<char>>,
            pub cursor: usize,
            pub temp: Option<Vec<char>>,
        }

        impl History {
            pub fn new() -> Self {
                let mut h = Self {
                    #[cfg(debug_assertions)]
                    debug_log: std::fs::File::create("resources/logs/terminal/history").unwrap_or_else(
                        |_| {
                            std::fs::create_dir_all("resources/logs/terminal").unwrap();
                            std::fs::File::create("resources/logs/terminal/history").unwrap()
                        },
                    ),
                    log: Vec::new(),
                    cursor: 0,
                    temp: None,
                };
                #[cfg(debug_assertions)]
                h.log(&InputAction::New);

                h
            }

            pub fn prev(&mut self, value: &mut Vec<char>) -> bool {
                if self.cursor == 0 {
                    return false;
                }

                if self.temp.is_none() || self.cursor == self.log.len() {
                    self.temp = Some(value.clone()); // temporarily keep input val
                }

                *value = self.log[self.cursor - 1].clone();
                self.cursor -= 1;

                true
            }

            pub fn next(&mut self, value: &mut Vec<char>) -> bool {
                if self.cursor == self.log.len() {
                    return false;
                }

                if self.cursor + 1 == self.log.len() {
                    *value = self.temp.as_ref().unwrap().clone();
                } else {
                    *value = self.log[self.cursor + 1].clone();
                }
                self.cursor += 1;

                true
            }

            pub fn push(&mut self, value: Vec<char>) {
                if value.iter().filter(|c| **c != ' ').count() > 0 && !self.log.contains(&value) {
                    self.log.push(value);
                }
                self.temp = None;
                self.cursor = self.log.len();
            }

            #[cfg(debug_assertions)]
            pub fn log(&mut self, method: &InputAction) {
                self.debug_log
                    .write_all(
                        format!(
                            "[LOG::{:?} - {:?}] {{ values[{:?}] = '{:?}' }} - {:?} | temp = {:?}\r\n",
                            method,
                            std::process::Command::new("date")
                                .arg("+\"%H:%M:%S:%N\"")
                                .output()
                                .expect("couldnt get time from linux command 'date'")
                                .stdout
                                .into_iter()
                                .map(|u| u as char)
                                .collect::<String>()
                                .replacen("\"", "", 2)
                                .trim_end_matches("\n"),
                            if self.cursor == 0 {
                                None
                            } else {
                                Some(self.cursor - 1)
                            },
                            if self.log.is_empty() || self.cursor == 0 {
                                None
                            } else {
                                Some(self.log[self.cursor - 1].clone())
                            },
                            self.log,
                            self.temp
                        )
                        .as_bytes(),
                    )
                    .unwrap();
            }
        }

        impl Input {
            fn overwrite_prompt(&mut self, new_prompt: &str) {
                self.prompt.clear();
                self.prompt.push_str(new_prompt);
            }

            fn write_prompt(&self, sol: &mut StdoutLock) {
                _ = sol.write(b"\x1b[2K");
                _ = sol.write(&[13]);
                _ = sol.write(
                    &self
                        .prompt
                        .chars()
                        .into_iter()
                        .map(|c| c as u8)
                        .collect::<Vec<u8>>(),
                );
                _ = sol.write(&self.values.iter().map(|c| *c as u8).collect::<Vec<u8>>());
                _ = sol.flush();

            }

            fn sync_cursor(&self, sol: &mut StdoutLock) {
                _ = sol.write(&[13]);
                for _idx in 0..self.prompt.len() + self.cursor {
                    _ = sol.write(b"\x1b[C");
                }
            }

            // fn toggle_alt_screen(&mut self, sol: &mut StdoutLock) {
            //     match self.alt_screen {
            //         true => {
            //             _ = sol.write(b"\x1b[?1049l");
            //         }
            //         false => {
            //             _ = sol.write(b"\x1b[?1049h");
            //         }
            //     }
            //
            //     self.alt_screen = !self.alt_screen;
            // }
        }
    })
}

// goes through the entire punctuated ast and generates the relevant code
fn punctuated_converge(input: Punctuated<CustomEvent, Semi>) -> (Vec<TS2>, Vec<TS2>, Vec<TS2>) {
    let mut input_actions = Vec::new();
    let mut kbd_events = Vec::new();
    let mut write_arms = Vec::new();
    input.into_iter().for_each(|ce| {
        let ia = &ce.input_action;
        input_actions.push(quote! { #ia });
        kbd_events.push(kbd_events_arm(
            &ce.key_code,
            &ce.modifiers,
            &ce.input_action,
        ));

        write_arms.push(input_write_arm(
            &ce.func,
            &ce.input_action,
            ce.key_code
                .to_token_stream()
                .to_string()
                .contains(['(', ')', '{', '}']),
        ))
    });
    (input_actions, kbd_events, write_arms)
}

// generates InputAction enum
fn input_actions_enum(ia: Vec<TS2>) -> TS2 {
    let ia = ia.iter();
    quote! {
        enum InputAction {
            PutChar(char),
            BackSpace,
            CRLF,
            MoveRight,
            MoveLeft,
            New,
            MoveEnd,
            MoveRightJump,
            MoveLeftJump,
            ClearLine,
            ClearRight,
            ClearLeft,
            // RmRightWord,
            // RmLeftWord,
            MoveHome,
            HistoryPrev,
            HistoryNext,
            #(#ia,)*
        }
    }
}

// generates kbd_events fn arm
fn kbd_events_arm(key_code: &Expr, modifiers: &Lit, ia: &Variant) -> TS2 {
    let event_val = {
        if let Expr::Call(call) = key_code {
            let args = &call.args;
            Some(quote! { (#args) })
        } else {
            None
        }
    };

    let ia_ident = &ia.ident;

    quote! {
        #key_code if key_event.modifiers == KeyModifiers::from_bits(#modifiers).unwrap() => {
            Command::InputAction(InputAction::#ia_ident #event_val)
        }
    }
}

// generates final kbd_events match fn
fn kbd_events_fn(kbd_events: Vec<TS2>) -> TS2 {
    quote! {
        fn kbd_event(key_event: KeyEvent) -> Command {
            match key_event.code {
                // default hard coded events
                KeyCode::Enter if key_event.modifiers == KeyModifiers::from_bits(0x0).unwrap() => {
                    Command::InputAction(InputAction::CRLF)
                }

                KeyCode::Backspace if key_event.modifiers == KeyModifiers::from_bits(0x0).unwrap() => {
                    Command::InputAction(InputAction::BackSpace)
                }

                KeyCode::Backspace if key_event.modifiers == KeyModifiers::from_bits(0x4).unwrap() => {
                    Command::InputAction(InputAction::ClearLine)
                }

                KeyCode::Up if key_event.modifiers == KeyModifiers::from_bits(0x0).unwrap() => {
                    Command::InputAction(InputAction::HistoryPrev)
                }

                KeyCode::Down if key_event.modifiers == KeyModifiers::from_bits(0x0).unwrap() => {
                    Command::InputAction(InputAction::HistoryNext)
                }

                KeyCode::Right if key_event.modifiers == KeyModifiers::from_bits(0x0).unwrap() => {
                    Command::InputAction(InputAction::MoveRight)
                }

                KeyCode::Left if key_event.modifiers == KeyModifiers::from_bits(0x0).unwrap() => {
                    Command::InputAction(InputAction::MoveLeft)
                }

                KeyCode::End if key_event.modifiers == KeyModifiers::from_bits(0x0).unwrap() => {
                    Command::InputAction(InputAction::MoveEnd)
                }

                KeyCode::Home if key_event.modifiers == KeyModifiers::from_bits(0x0).unwrap() => {
                    Command::InputAction(InputAction::MoveHome)
                }

                KeyCode::Right if key_event.modifiers == KeyModifiers::from_bits(0x4).unwrap() => {
                    Command::InputAction(InputAction::MoveRightJump)
                }
                KeyCode::Left if key_event.modifiers == KeyModifiers::from_bits(0x4).unwrap() => {
                    Command::InputAction(InputAction::MoveLeftJump)
                }
                KeyCode::Right if key_event.modifiers == KeyModifiers::from_bits(0x6).unwrap() => {
                    Command::InputAction(InputAction::ClearRight)
                }

                KeyCode::Left if key_event.modifiers == KeyModifiers::from_bits(0x6).unwrap() => {
                    Command::InputAction(InputAction::ClearLeft)
                }

                KeyCode::Char(c) => match c {
                    'c' if key_event.modifiers == KeyModifiers::from_bits(0x2).unwrap() => Command::Exit(0),
                    c if key_event.modifiers == KeyModifiers::from_bits(0x0).unwrap() => {
                        Command::InputAction(InputAction::PutChar(c))
                    }
                    _ => Command::None,
                },

                // custom events
                #(#kbd_events)*

                _ => Command::None,
            }
        }
    }
}

// generates one arm in the Input.write() fn
fn input_write_arm(func: &ExprClosure, ia: &Variant, with_data: bool) -> TS2 {
    let ia_ident = &ia.ident;
    let val = if with_data {
        Some(quote! { (val )})
    } else {
        None
    };
    quote! {
        InputAction::#ia_ident #val => {
            (#func)()
        }
    }
}

// generates final Input.write() fn
fn input_write(arms: Vec<TS2>) -> TS2 {
    quote! {
        fn write(
            &mut self,
            h: &mut History,
            ia: &InputAction,
            sol: &mut StdoutLock<'_>,
            ui: &mut String,
        ) {
            match ia {
                // default hard coded arms
                InputAction::New => (),
                InputAction::MoveRight => {
                    if self.to_the_right() {
                        _ = sol.write(b"\x1b[C");
                    }
                }

                InputAction::MoveLeft => {
                    if self.to_the_left() {
                        _ = sol.write(b"\x1b[D");
                    }
                }

                InputAction::BackSpace => {
                    self.backspace();
                    self.write_prompt(sol);
                    self.sync_cursor(sol);
                }

                InputAction::ClearLine => {
                    self.clear_line();
                    self.write_prompt(sol);
                }

                InputAction::ClearRight => {
                    self.clear_right();
                    _ = sol.write(b"\x1b[0K");
                    self.sync_cursor(sol);
                }

                InputAction::ClearLeft => {
                    self.clear_left();
                    self.write_prompt(sol);
                    self.sync_cursor(sol);
                }
                    InputAction::CRLF => {
                    self.cr_lf(h, ui);
                    #[cfg(debug_assertions)]
                    h.log(&ia);
                    _ = sol.write(&[13, 10]);
                    self.write_prompt(sol);

                    // TODO: tokens probably should be peekable in general
                    // HACK: this is a wasteful hack
                    // should be prompted in a popup buffer for the name
                }

                InputAction::PutChar(c) => {
                    self.put_char(*c);
                    // _ = sol.write(b"\x1b[31;1;4m");
                    self.write_prompt(sol);
                    self.sync_cursor(sol);
                }

                InputAction::MoveEnd => match self.to_end() {
                    0 => (),
                    val => {
                        for _ in 0..val {
                            _ = sol.write(b"\x1b[C");
                        }
                    }
                },

                InputAction::MoveHome => {
                    if self.to_home() {
                        _ = sol.write(&[13]);
                        for _ in 0..self.prompt.len() {
                            _ = sol.write(b"\x1b[C");
                        }
                        // OR
                        // self.write_prompt(sol);
                    }
                }

                InputAction::MoveRightJump => {
                    self.to_right_jump();
                    _ = sol.write(&[13]);
                    self.sync_cursor(sol);
                }
                    InputAction::MoveLeftJump => {
                    self.to_left_jump();
                    _ = sol.write(&[13]);
                    self.sync_cursor(sol);
                }

                InputAction::HistoryPrev => {
                    if h.prev(&mut self.values) {
                        self.write_prompt(sol);
                        self.cursor = self.values.len();
                    }
                    #[cfg(debug_assertions)]
                    h.log(&ia);
                }

                InputAction::HistoryNext => {
                    if h.next(&mut self.values) {
                        self.write_prompt(sol);
                        self.cursor = self.values.len();
                    }
                    #[cfg(debug_assertions)]
                    h.log(&ia);
                }

                #(#arms,)*
            }
            sol.flush();
            #[cfg(debug_assertions)]
            self.log(&ia);
        }
    }
}
