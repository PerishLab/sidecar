use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::time::Duration;

pub trait Duplex: Read + Write {}

impl<T: Read + Write> Duplex for T {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bridge {
    seat: String,
}

impl Bridge {
    pub fn new(project: &Path, namespace: &str, name: &str) -> Self {
        Self {
            seat: seat(project, namespace, name),
        }
    }

    pub fn seat(&self) -> &str {
        &self.seat
    }

    pub fn exchange(&self, line: &str, patience: Option<Duration>) -> Result<String, String> {
        let stream = self.dial(patience)?;
        let mut reader = BufReader::new(stream);
        reader
            .get_mut()
            .write_all(line.as_bytes())
            .map_err(|err| err.to_string())?;
        reader.get_mut().flush().map_err(|err| err.to_string())?;
        let mut response = String::new();
        reader
            .read_line(&mut response)
            .map_err(|err| err.to_string())?;
        Ok(response)
    }

    #[cfg(unix)]
    fn dial(&self, patience: Option<Duration>) -> Result<Box<dyn Duplex>, String> {
        let stream =
            std::os::unix::net::UnixStream::connect(&self.seat).map_err(|err| err.to_string())?;
        if let Some(patience) = patience {
            let _ = stream.set_read_timeout(Some(patience));
            let _ = stream.set_write_timeout(Some(patience));
        }
        Ok(Box::new(stream))
    }

    #[cfg(windows)]
    fn dial(&self, patience: Option<Duration>) -> Result<Box<dyn Duplex>, String> {
        let _ = patience;
        let stream = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.seat)
            .map_err(|err| err.to_string())?;
        Ok(Box::new(stream))
    }

    #[cfg(not(any(unix, windows)))]
    fn dial(&self, patience: Option<Duration>) -> Result<Box<dyn Duplex>, String> {
        let _ = patience;
        Err("this platform carries no inspect bridge".to_string())
    }
}

fn seat(project: &Path, namespace: &str, name: &str) -> String {
    #[cfg(unix)]
    {
        let _ = namespace;
        project
            .join("inspect")
            .join(format!("{name}.sock"))
            .display()
            .to_string()
    }

    #[cfg(windows)]
    {
        let _ = project;
        format!(r"\\.\pipe\sidecar-{namespace}-{name}")
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (project, namespace, name);
        String::new()
    }
}
