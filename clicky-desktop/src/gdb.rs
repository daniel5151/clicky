use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

use crate::DynResult;

use gdbstub::{Connection, GdbStub};

/// GDB server configuration. Typically instantiated via StructOpt.
#[derive(Debug, Clone)]
pub struct GdbCfg {
    pub kind: ConnKind,
    pub on_start: bool,
    pub on_fatal_err: bool,
}

#[derive(Debug, Clone)]
pub enum ConnKind {
    Tcp(u16),
    Uds(PathBuf),
}

impl std::str::FromStr for GdbCfg {
    type Err = String;

    fn from_str(s: &str) -> Result<GdbCfg, String> {
        let mut s = s.split(',');
        let kind = s.next().unwrap().parse::<ConnKind>()?;

        let on_fatal_err = s.next() == Some("on-fatal-err");
        let on_start = if on_fatal_err {
            match s.next() {
                Some("and-on-start") => true,
                Some(o) => return Err(format!("unknown option `{}`", o)),
                None => false,
            }
        } else {
            true
        };

        Ok(GdbCfg {
            kind,
            on_start,
            on_fatal_err,
        })
    }
}

impl std::str::FromStr for ConnKind {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<ConnKind, &'static str> {
        Ok(match s.parse::<u16>() {
            Ok(port) => ConnKind::Tcp(port),
            Err(_) => ConnKind::Uds(s.into()),
        })
    }
}

fn wait_for_tcp(port: u16) -> std::io::Result<TcpStream> {
    let sockaddr = format!("127.0.0.1:{}", port);
    eprintln!("Waiting for a GDB connection on {:?}...", sockaddr);

    let sock = TcpListener::bind(sockaddr)?;
    let (stream, addr) = sock.accept()?;
    eprintln!("Debugger connected from {}", addr);

    Ok(stream)
}

#[cfg(unix)]
fn wait_for_uds(path: PathBuf) -> std::io::Result<UnixStream> {
    match std::fs::remove_file(&path) {
        Ok(_) => {}
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {}
            _ => return Err(e),
        },
    }

    eprintln!("Waiting for a GDB connection on {:?}...", path);

    let sock = UnixListener::bind(path)?;
    let (stream, addr) = sock.accept()?;
    eprintln!("Debugger connected from {:?}", addr);

    Ok(stream)
}

pub fn make_gdbstub<'a, T>(
    cfg: GdbCfg,
) -> DynResult<GdbStub<'a, T, Box<dyn Connection<Error = std::io::Error>>>>
where
    T: gdbstub::Target,
    T::Error: 'a,
{
    let connection: Box<dyn Connection<Error = std::io::Error>> = match cfg.kind {
        ConnKind::Tcp(port) => Box::new(wait_for_tcp(port)?),
        ConnKind::Uds(path) => {
            #[cfg(not(unix))]
            {
                let _ = path;
                return Err("Unix Domain Sockets can only be used on Unix".into());
            }
            #[cfg(unix)]
            {
                Box::new(wait_for_uds(path)?)
            }
        }
    };

    Ok(GdbStub::new(connection))
}
