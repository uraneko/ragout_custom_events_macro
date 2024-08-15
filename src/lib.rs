use proc_macro::TokenStream;
use proc_macro2::TokenStream as TS2;

use quote::quote;
use quote::ToTokens;

use syn::parse::{Parse, ParseStream};
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

fn gen_lib() -> TS2 {
    quote! {
        use std::io::{StdoutLock, Write};
        use std::os::fd::AsRawFd;

        pub use ragout_assistant::init;
        use ragout_assistant::{DebugLog, Writer};
        use ragout_assistant::{History, Input};

        // TODO: get rid of crossterm dependency
        // TODO: render graphics

        // raw mode:
        // you need to create exetrns for C functions from unistd.h
        // Specifically to enable raw mode you need tcgetattr and tcsetattr functions.

        // move the logs in History::new and Input::new to this fn
        // since they can't stay there due to design limitations
        // #[cfg(any(debug_assertions, feature = "debug_logs"))]
        // pub fn log_init(i: &mut Input, h: &mut History) {
        //     i.log(&InputAction::New);
        //     h.log(&InputAction::New);
        // }

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

        enum Command {
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
            //         .values
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

        #[cfg(any(debug_assertions, feature = "debug_logs"))]
        impl DebugLog<InputAction> for Input {
            fn log(&mut self, event: &InputAction) {
                self.debug_log
                    .write_all(
                        format!(
                            "[LOG::{:?} - {:?}] {{ values[{:?}] = '{:?}' }} - {:?}\r\n",
                            event,
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

            fn dl_rfd(&self) -> i32 {
                self.debug_log.as_raw_fd()
            }
        }

        #[cfg(any(debug_assertions, feature = "debug_logs"))]
        impl DebugLog<InputAction> for History {
            fn log(&mut self, event: &InputAction) {
                self.debug_log
                    .write_all(
                        format!(
                            "[LOG::{:?} - {:?}] {{ values[{:?}] = '{:?}' }} - {:?} | temp = {:?}\r\n",
                            event,
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
                                Some(self.values[self.cursor - 1].clone())
                            },
                            self.values,
                            self.temp
                        )
                        .as_bytes(),
                    )
                    .unwrap();
            }

            fn dl_rfd(&self) -> i32 {
                self.debug_log.as_raw_fd()
            }
        }
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

    let lib = gen_lib();

    TokenStream::from(quote! {
        #lib

        #[derive(Debug)]
        #input_actions

        #kbd_events

        impl Writer<InputAction> for Input {
            #input_write
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
            // New,
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
                // InputAction::New => (),
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
                        for _ in 0..self.prompt.chars().count() + 1 {
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
