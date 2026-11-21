use std::error::Request;
use std::fmt::{Debug, Display, Formatter};

use tokio::io;

pub trait CloneError {
    type Value;
    type Error;
    fn clone_error_static(&'static self) -> Result<&'static Self::Value, Self::Error>;
    fn clone_error(&self) -> Result<&Self::Value, Self::Error>;
}

impl<T> CloneError for io::Result<T> {
    type Value = T;
    type Error = io::Error;
    fn clone_error_static(&'static self) -> Result<&'static Self::Value, Self::Error> {
        self.as_ref().map_err(|e| io::Error::new(e.kind(), e))
    }
    fn clone_error(&self) -> Result<&Self::Value, Self::Error> {
        self.as_ref()
            .map_err(|e| io::Error::new(e.kind(), e.to_string()))
    }
}

struct AnyhowRef(&'static anyhow::Error);

impl Debug for AnyhowRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.0, f)
    }
}

impl Display for AnyhowRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.0, f)
    }
}

impl std::error::Error for AnyhowRef {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
    #[expect(deprecated)]
    fn description(&self) -> &str {
        self.0.description()
    }
    #[expect(deprecated)]
    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.0.cause()
    }
    fn provide<'a>(&'a self, request: &mut Request<'a>) {
        self.0.provide(request)
    }
}

impl<T> CloneError for anyhow::Result<T> {
    type Value = T;
    type Error = anyhow::Error;
    fn clone_error_static(&'static self) -> Result<&'static Self::Value, Self::Error> {
        self.as_ref().map_err(|e| anyhow::Error::new(AnyhowRef(e)))
    }

    fn clone_error(&self) -> Result<&Self::Value, Self::Error> {
        self.as_ref()
            .map_err(|e| anyhow::Error::msg(format!("{e:?}")))
    }
}
