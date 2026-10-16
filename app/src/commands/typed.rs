//! Commands arrive as JSON. `typed` turns a handler written over a plain Rust
//! argument struct and result into one the command registry accepts, so every
//! command declares its arguments as a type instead of digging through JSON.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use svelte_gpui::CommandError;

pub fn typed<Arguments, Answer, Handler>(handler: Handler) -> impl Fn(Value) -> Result<Value, CommandError>
where
    Arguments: DeserializeOwned,
    Answer: Serialize,
    Handler: Fn(Arguments) -> Result<Answer, CommandError>,
{
    move |input| {
        let arguments = read_arguments(input)?;
        let answer = handler(arguments)?;
        serde_json::to_value(answer).map_err(|error| CommandError::new(error.to_string()))
    }
}

/// A command called with no arguments sends `null`, which reads as `()` or as a
/// struct whose fields all have defaults.
fn read_arguments<Arguments: DeserializeOwned>(input: Value) -> Result<Arguments, CommandError> {
    let input = match input {
        Value::Null => Value::Object(Default::default()),
        other => other,
    };
    serde_json::from_value(input).map_err(|error| CommandError::with_code("BadArguments", error.to_string()))
}

/// Turns any displayable failure into the message the UI shows.
pub fn failure(error: impl std::fmt::Display) -> CommandError {
    CommandError::new(error.to_string())
}

/// Arguments for a command that takes none.
#[derive(serde::Deserialize, Default)]
pub struct NoArguments {}
