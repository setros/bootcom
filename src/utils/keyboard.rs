use std::io::stdout;
use std::{process, time::Duration};

use crossterm::{
    cursor::{Hide, MoveToColumn, Show},
    event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
    Result,
};

pub(crate) fn poll_escape() -> Result<bool> {
    enable_raw_mode()?;

    let mut esc_pressed = false;

    enable_raw_mode()?;
    execute!(stdout(), Hide)?;
    let result = poll(Duration::from_millis(500))?;
    execute!(stdout(), MoveToColumn(0), Show)?;
    disable_raw_mode()?;

    if result {
        // It's guaranteed that read() wont block if `poll` returns `Ok(true)`
        let event = read()?;

        if event == Event::Key(KeyCode::Esc.into()) {
            esc_pressed = true;
        } else if event
            == Event::Key(KeyEvent {
                modifiers: KeyModifiers::CONTROL,
                code: KeyCode::Char('c'),
            })
        {
            // As we are in raw mode, Ctrl+C will be captured here as a key
            // event. Catch it and exit the process if that happens
            process::exit(0);
        }
    } else {
        // Timeout expired with no event
    }

    Ok(esc_pressed)
}
