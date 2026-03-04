use crossterm::event::KeyEvent;

use crate::model::ComposeProject;

/// Events flowing through the application's main channel.
#[derive(Debug)]
pub enum AppEvent {
    /// A keyboard event from the terminal.
    Key(KeyEvent),
    /// Terminal resize.
    Resize,
    /// Render tick (~30fps).
    Tick,
    /// Fresh Docker data arrived.
    DockerUpdate(Vec<ComposeProject>),
    /// Docker connection error.
    DockerError(String),
}
