//! Input handling — bridges crossterm events to application events.

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::time::Duration;

/// Application-level events.
#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
    Quit,
    Error,
}

/// Handles input event polling.
pub struct InputHandler;

impl InputHandler {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Wait for the next event or tick.
    pub async fn next_event(&self, timeout: Duration) -> anyhow::Result<Event> {
        let event: Event = tokio::task::spawn_blocking(move || -> anyhow::Result<Event> {
            if event::poll(timeout)? {
                match event::read()? {
                    CrosstermEvent::Key(key) => Ok(Event::Key(key)),
                    CrosstermEvent::Mouse(mouse) => Ok(Event::Mouse(mouse)),
                    CrosstermEvent::Resize(w, h) => Ok(Event::Resize(w, h)),
                    CrosstermEvent::FocusGained | CrosstermEvent::FocusLost => {
                        Ok(Event::Tick)
                    }
                    _ => Ok(Event::Tick),
                }
            } else {
                Ok(Event::Tick)
            }
        })
        .await??;

        Ok(event)
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}
