// run with shell command:
// $ cargo expand --example default
use ragout_custom_events_macro::ragout_custom_events;

ragout_custom_events! {
    KeyCode::F(5), 0x0, TestF(u8),
    || {
        let date = std::process::Command::new("date")
            .output()
            .unwrap()
            .stdout.into_iter()
            .map(|u| u as char)
            .collect::<String>()
            .replacen("\"", "", 2);


        self.overwrite_prompt(date
            .trim_end_matches('\n'));
        self.write_prompt(sol);
        // TODO: sol.write input, should be called from inside input.write_prompt() right before
        // sol.flush() at the end
    };
    KeyCode::Esc, 0x0, TestPrintScreen,
    || {
        // requires that the grim cli tool (or something similar, replace as needed) is installed
        _ = std::process::Command::new("grim").arg("target/screenshot.png").output().unwrap();


        let inst = std::time::Instant::now();

        let temp = self.prompt.drain(..).collect::<String>();
        self.overwrite_prompt("saved screenshot to target/screenshot.png> ");
        self.write_prompt(sol);

        // TODO: need async for non blocking
        let notify =  std::thread::spawn(move || loop {
                if inst.elapsed() > std::time::Duration::from_secs(3) {
                    break true;
                }
        });

        let notify = notify.join().unwrap();
        if notify {
            self.overwrite_prompt(&temp);
            self.write_prompt(sol);
        }

    };
}

fn main() {}
