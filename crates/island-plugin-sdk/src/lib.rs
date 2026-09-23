//! Client SDK for out-of-process Island plugins.

use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use island_plugin_api::{
    API_VERSION, Endpoint, FramePacket, HostToPlugin, PluginToHost, SurfaceFrame, read_endpoint,
    read_message, write_frame, write_json,
};
use thiserror::Error;

#[cfg(unix)]
use std::os::unix::net::UnixStream;

type PluginReader = Box<dyn Read + Send>;
type PluginWriter = Box<dyn Write + Send>;
type PluginStream = (PluginReader, PluginWriter, Arc<dyn Fn() + Send + Sync>);

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not connected")]
    NotConnected,
}

/// Connected plugin session with background reader → channel.
pub struct PluginClient {
    writer: PluginWriter,
    events: Receiver<HostToPlugin>,
    pub instance_id: String,
    pub data_dir: PathBuf,
    pub permissions: Vec<String>,
    reader_thread: Option<thread::JoinHandle<()>>,
    shutdown: Arc<dyn Fn() + Send + Sync>,
}

impl PluginClient {
    /// Connect using `ISLAND_ENDPOINT` env, else host endpoint file.
    pub fn connect(plugin_name: impl Into<String>) -> Result<Self, SdkError> {
        let endpoint = if let Ok(addr) = std::env::var("ISLAND_ENDPOINT") {
            Endpoint { addr }
        } else {
            read_endpoint()?
        };
        Self::connect_endpoint(&endpoint, plugin_name)
    }

    pub fn connect_endpoint(
        endpoint: &Endpoint,
        plugin_name: impl Into<String>,
    ) -> Result<Self, SdkError> {
        let token = std::env::var("ISLAND_PLUGIN_TOKEN")
            .map_err(|_| SdkError::Protocol("missing host-issued ISLAND_PLUGIN_TOKEN".into()))?;
        Self::connect_authenticated(endpoint, plugin_name, &token)
    }

    pub fn connect_authenticated(
        endpoint: &Endpoint,
        plugin_name: impl Into<String>,
        token: &str,
    ) -> Result<Self, SdkError> {
        let name = plugin_name.into();
        let (reader, writer, shutdown) = open_stream(&endpoint.addr)?;
        let mut reader = BufReader::new(reader);
        let mut writer = writer;

        // Expect Hello
        let hello = recv_one(&mut reader)?;
        let instance_id = match hello {
            HostToPlugin::Hello {
                api_version,
                instance_id,
                ..
            } => {
                if api_version != API_VERSION {
                    return Err(SdkError::Protocol(format!(
                        "api_version mismatch host={api_version} sdk={API_VERSION}"
                    )));
                }
                instance_id
            }
            other => {
                return Err(SdkError::Protocol(format!("expected Hello, got {other:?}")));
            }
        };

        let id_name = std::env::var("ISLAND_PLUGIN_ID").unwrap_or(name);
        write_json(
            &mut writer,
            &PluginToHost::HelloOk {
                api_version: API_VERSION,
                name: id_name,
                token: token.into(),
            },
        )?;

        let (data_dir, permissions) = match recv_one(&mut reader)? {
            HostToPlugin::Configure {
                data_dir,
                permissions,
            } => (PathBuf::from(data_dir), permissions),
            other => {
                return Err(SdkError::Protocol(format!(
                    "expected Configure, got {other:?}"
                )));
            }
        };

        // Background reader → channel
        let (tx_ev, rx_ev) = mpsc::sync_channel::<HostToPlugin>(64);
        let reader_shutdown = shutdown.clone();
        let reader_thread = thread::Builder::new()
            .name("island-plugin-rx".into())
            .spawn(move || {
                let mut reader = reader;
                while let Ok(msg) = recv_one(&mut reader) {
                    if matches!(msg, HostToPlugin::Shutdown) {
                        let _ = tx_ev.try_send(msg);
                        break;
                    }
                    if tx_ev.try_send(msg).is_err() {
                        break;
                    }
                }
                reader_shutdown();
            })
            .map_err(|e| SdkError::Io(std::io::Error::other(e)))?;

        Ok(Self {
            writer,
            events: rx_ev,
            instance_id,
            data_dir,
            permissions,
            reader_thread: Some(reader_thread),
            shutdown,
        })
    }

    pub fn send(&mut self, msg: &PluginToHost) -> Result<(), SdkError> {
        write_json(&mut self.writer, msg)?;
        Ok(())
    }

    pub fn present(
        &mut self,
        priority: u8,
        ttl_ms: Option<u64>,
        sticky: bool,
        reason: Option<String>,
    ) -> Result<(), SdkError> {
        self.send(&PluginToHost::Present {
            priority,
            mode: island_plugin_api::PresentationMode::Expanded,
            ttl_ms,
            sticky,
            reason,
        })
    }

    pub fn present_with_mode(
        &mut self,
        priority: u8,
        ttl_ms: Option<u64>,
        sticky: bool,
        reason: Option<String>,
        mode: island_plugin_api::PresentationMode,
    ) -> Result<(), SdkError> {
        self.send(&PluginToHost::Present {
            priority,
            ttl_ms,
            sticky,
            reason,
            mode,
        })
    }

    pub fn dismiss(&mut self) -> Result<(), SdkError> {
        self.send(&PluginToHost::Dismiss)
    }

    pub fn preferred_size(&mut self, w: u32, h: u32) -> Result<(), SdkError> {
        self.send(&PluginToHost::PreferredSize { w, h })
    }

    pub fn set_collapsed_meta(
        &mut self,
        leading: Option<String>,
        trailing: Option<String>,
        dot: Option<String>,
    ) -> Result<(), SdkError> {
        self.send(&PluginToHost::SetCollapsedMeta {
            leading,
            trailing,
            dot,
            icon: None,
        })
    }

    pub fn submit_frame(&mut self, frame: SurfaceFrame) -> Result<(), SdkError> {
        write_frame(&mut self.writer, &frame)?;
        Ok(())
    }

    pub fn log(
        &mut self,
        level: island_plugin_api::LogLevel,
        msg: impl Into<String>,
    ) -> Result<(), SdkError> {
        self.send(&PluginToHost::Log {
            level,
            msg: msg.into(),
        })
    }

    /// Non-blocking poll of host messages (Input, SurfaceResize, Shutdown, …).
    pub fn try_recv(&self) -> Option<HostToPlugin> {
        match self.events.try_recv() {
            Ok(m) => Some(m),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(HostToPlugin::Shutdown),
        }
    }

    /// Drain all pending host events.
    pub fn drain(&self) -> Vec<HostToPlugin> {
        let mut out = Vec::new();
        while let Some(m) = self.try_recv() {
            let stop = matches!(&m, HostToPlugin::Shutdown);
            out.push(m);
            if stop {
                break;
            }
        }
        out
    }
}

impl Drop for PluginClient {
    fn drop(&mut self) {
        (self.shutdown)();
        if let Some(reader) = self.reader_thread.take() {
            let _ = reader.join();
        }
    }
}

fn recv_one(reader: &mut BufReader<PluginReader>) -> Result<HostToPlugin, SdkError> {
    match read_message(reader)? {
        FramePacket::Json(bytes) => Ok(serde_json::from_slice(&bytes)?),
        FramePacket::Frame(_) => Err(SdkError::Protocol(
            "unexpected binary frame from host".into(),
        )),
    }
}

fn open_stream(addr: &str) -> Result<PluginStream, SdkError> {
    if let Some(_path) = addr.strip_prefix("unix:") {
        #[cfg(unix)]
        {
            let stream = UnixStream::connect(_path)?;
            // The host may be silent indefinitely after initial surface
            // configuration. This dedicated reader thread must block rather
            // than interpret healthy inactivity as a plugin shutdown.
            stream.set_read_timeout(None)?;
            stream.set_write_timeout(Some(Duration::from_secs(30)))?;
            let reader = stream.try_clone()?;
            let closer = stream.try_clone()?;
            return Ok((
                Box::new(reader),
                Box::new(stream),
                Arc::new(move || {
                    let _ = closer.shutdown(std::net::Shutdown::Both);
                }),
            ));
        }
        #[cfg(not(unix))]
        {
            return Err(SdkError::Protocol("unix sockets unsupported".into()));
        }
    }
    if let Some(hostport) = addr.strip_prefix("tcp:") {
        let stream = TcpStream::connect(hostport)?;
        stream.set_nodelay(true)?;
        // Match Unix behavior: host silence is normal, not a disconnect.
        stream.set_read_timeout(None)?;
        stream.set_write_timeout(Some(Duration::from_secs(30)))?;
        let reader = stream.try_clone()?;
        let closer = stream.try_clone()?;
        return Ok((
            Box::new(reader),
            Box::new(stream),
            Arc::new(move || {
                let _ = closer.shutdown(std::net::Shutdown::Both);
            }),
        ));
    }
    Err(SdkError::Protocol(format!("bad endpoint addr: {addr}")))
}
