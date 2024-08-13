use proc_macro::TokenStream;
use proc_macro2::TokenStream as TS2;

use quote::quote;

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Semi;
use syn::{parse_macro_input, Token};
use syn::{ExprClosure, Ident, Lit, Result, Variant};

struct CustomEvent {
    key_code: Ident,
    modifiers: Lit,
    input_action: Variant,
    func: ExprClosure,
}

impl Parse for CustomEvent {
    fn parse(input: ParseStream) -> Result<Self> {
        let key_code = Ident::parse(input)?;
        _ = <Token![<]>::parse(input)?;
        let modifiers = Lit::parse(input)?;
        _ = <Token![>]>::parse(input)?;
        _ = <Token![,]>::parse(input)?;
        let input_action = Variant::parse(input)?;
        _ = <Token![,]>::parse(input)?;
        let func = ExprClosure::parse(input)?;

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
pub fn ragout_input(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as MacroInput);
    let ast = ast.unpack();

    let (input_actions, kbd_events, write_arms) = punctuated_converge(ast);

    let input_actions = input_actions_enum(input_actions);
    let kbd_events = kbd_events_fn(kbd_events);
    let input_write = input_write(write_arms);

    TokenStream::from(quote! {
        #input_actions

        #kbd_events

        impl Input {
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
        input_actions.push(quote! { ce.input_action });
        kbd_events.push(kbd_events_arm(
            &ce.key_code,
            &ce.modifiers,
            &ce.input_action,
        ));
        write_arms.push(input_write_arm(&ce.func, &ce.input_action))
    });
    (input_actions, kbd_events, write_arms)
}

// generates InputAction enum
fn input_actions_enum(ia: Vec<TS2>) -> TS2 {
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
fn kbd_events_arm(key_code: &Ident, modifiers: &Lit, ia: &Variant) -> TS2 {
    quote! {
        #key_code if key_event.modifiers == KeyModifiers::from_bits(#modifiers).unwrap() {
            Command::InputAction(InputAction::#ia)
        }
    }
}

// generates final kbd_events match fn
fn kbd_events_fn(kbd_events: Vec<TS2>) -> TS2 {
    quote! {
        fn kbd_event(key_event: KeyEvent) -> Command {
            match key_event.code {
                // my  default hard coded events
                   24         KeyCode::Enter if key_event.modifiers == KeyModifiers::from_bits(0x0).unwrap() => {
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

                _ => Command::None,

                // custom events
                #(#kbd_events,)*
                _ => Command::None,
            }
        }
    }
}

// generates one arm in the Input.write() fn
fn input_write_arm(func: &ExprClosure, ia: &Variant) -> TS2 {
    quote! {
        InputAction::#ia => {
            #func
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
                    _ = sol.write(&self.values.iter().map(|c| *c as u8).collect::<Vec<u8>>());
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
                    _ = sol.write(&self.values.iter().map(|c| *c as u8).collect::<Vec<u8>>());
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
                    _ = sol.write(&self.values.iter().map(|c| *c as u8).collect::<Vec<u8>>());
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
                        _ = sol.write(&self.values.iter().map(|c| *c as u8).collect::<Vec<u8>>());
                    }
                    #[cfg(debug_assertions)]
                    h.log(&ia);
                }

                InputAction::HistoryNext => {
                    if h.next(&mut self.values) {
                        self.write_prompt(sol);
                        self.cursor = self.values.len();
                        _ = sol.write(&self.values.iter().map(|c| *c as u8).collect::<Vec<u8>>());
                    }
                    #[cfg(debug_assertions)]
                    h.log(&ia);
                }

                #(#arms,)*
            }
        }
    }
}
