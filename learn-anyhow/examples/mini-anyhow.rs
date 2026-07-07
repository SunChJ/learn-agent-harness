use std::error::Error as StdError;
use std::fmt;

type Result<T> = std::result::Result<T, MiniError>;

#[derive(Debug)]
struct MiniError {
    inner: Box<dyn StdError + Send + Sync + 'static>,
}

impl MiniError {
    fn new<E>(err: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        MiniError {
            inner: Box::new(err),
        }
    }
}

impl fmt::Display for MiniError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<E> From<E> for MiniError
where
    E: StdError + Send + Sync + 'static,
{
    fn from(err: E) -> Self {
        MiniError::new(err)
    }
}

fn read_config() -> Result<String> {
    let path = format!("{}/config.toml", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(path)?;
    Ok(text)
}

fn main() -> Result<()> {
    let config = read_config()?;
    println!("{config}");
    Ok(())
}
