use crate::error::ApiTesterError;
use crate::http::Response;

pub enum AppEvent {
    Key(crossterm::event::KeyEvent),
    Tick,
    RequestStarted,
    RequestDone(Result<Response, ApiTesterError>),
    Quit,
}
