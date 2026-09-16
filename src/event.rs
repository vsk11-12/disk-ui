use anyhow::Result;
use crossterm::event::{self, Event as CEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

pub enum Event {
    Key(KeyEvent),
    Tick,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new(tick_ms: u64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx2 = tx.clone();

        // Spawn blocking reader for crossterm events
        tokio::task::spawn_blocking(move || loop {
            if event::poll(Duration::from_millis(tick_ms)).unwrap_or(false) {
                if let Ok(CEvent::Key(k)) = event::read() {
                    if tx.send(Event::Key(k)).is_err() { break }
                }
            } else {
                if tx.send(Event::Tick).is_err() { break }
            }
        });

        // Also drive ticks if no keyboard activity
        let _ = tx2; // keep alive
        Self { rx }
    }

    pub async fn next(&mut self) -> Result<Event> {
        self.rx.recv().await.ok_or_else(|| anyhow::anyhow!("Event channel closed"))
    }
}
