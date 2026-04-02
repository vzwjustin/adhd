use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

/// Application events — either terminal input or internal ticks.
#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
    Autosave,
}

/// Spawns an event loop that reads terminal events and sends them
/// to the app via a channel. Also produces periodic ticks for animations
/// and autosave events.
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
    _tx: mpsc::UnboundedSender<AppEvent>,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64, autosave_interval_secs: u64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let event_tx = tx.clone();

        // Terminal event reader
        tokio::spawn(async move {
            let tick_duration = Duration::from_millis(tick_rate_ms);
            let mut autosave_counter: u64 = 0;
            let autosave_ticks = (autosave_interval_secs * 1000) / tick_rate_ms;

            loop {
                if event::poll(tick_duration).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            if event_tx.send(AppEvent::Key(key)).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Resize(w, h)) => {
                            if event_tx.send(AppEvent::Resize(w, h)).is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                } else {
                    // Tick
                    if event_tx.send(AppEvent::Tick).is_err() {
                        break;
                    }
                    autosave_counter += 1;
                    if autosave_counter >= autosave_ticks {
                        autosave_counter = 0;
                        if event_tx.send(AppEvent::Autosave).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self { rx, _tx: tx }
    }

    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }
}
