use serde_json::Value;
use std::io::{Error, ErrorKind};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process::Command;
use thiserror::Error;

pub fn get_sway_socketpath() -> Result<String, Error> {
    let output = Command::new("sway").arg("--get-socketpath").output()?;

    if !output.status.success() {
        return Err(Error::new(
            ErrorKind::Other,
            format!(
                "sway --get-socketpath failed with exit code: {}",
                output.status
            ),
        ));
    }

    let socketpath = String::from_utf8(output.stdout)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?
        .trim()
        .to_string();

    if socketpath.is_empty() {
        return Err(Error::new(
            ErrorKind::NotFound,
            "Empty socketpath returned from sway",
        ));
    }

    Ok(socketpath)
}

#[derive(Error, Debug)]
pub enum SwayError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Sway IPC error: {0}")]
    IpcError(String),
}

pub(crate) struct Sway {
    stream: UnixStream,
}

impl Sway {
    pub fn new() -> Result<Sway, Error> {
        let socket_path = get_sway_socketpath()?;
        let stream = UnixStream::connect(&socket_path)?;

        Ok(Sway { stream })
    }

    pub fn get_active_workspace(&self) -> Result<usize, anyhow::Error> {
        let workspaces: Value = self.send_command(1, "get_workspaces")?;

        for workspace in workspaces.as_array().unwrap() {
            if workspace["focused"].as_bool().unwrap_or(false) {
                return workspace["num"]
                    .as_u64()
                    .map(|n| n as usize)
                    .ok_or_else(|| {
                        SwayError::IpcError("Invalid workspace number".to_string()).into()
                    });
            }
        }

        Err(SwayError::IpcError("No active workspace found".to_string()).into())
    }

    pub fn set_active_workspace(&mut self, workspace: usize) -> Result<(), anyhow::Error> {
        self.send_command(0, &format!("workspace number {}", workspace))?;
        Ok(())
    }

    fn send_command(&self, command_type: u32, command: &str) -> Result<Value, anyhow::Error> {
        // Create the IPC message
        let payload = command.as_bytes();

        let header = [
            // Magic string
            b"i3-ipc",
            // Message length
            &(payload.len() as u32).to_ne_bytes()[0..4],
            // Message type
            &command_type.to_ne_bytes(),
        ];

        // Write the message
        let mut stream = self.stream.try_clone()?;
        stream.write_all(&header.concat())?;
        stream.write_all(payload)?;
        stream.flush()?;

        // Read the response header
        let mut header = [0u8; 14];
        stream.read_exact(&mut header)?;

        // Verify magic string
        if &header[0..6] != b"i3-ipc" {
            return Err(SwayError::IpcError("Invalid magic string in response".to_string()).into());
        }

        // Get payload length
        let length = u32::from_ne_bytes(header[6..10].try_into().unwrap()) as usize;

        // Read payload
        let mut payload = vec![0u8; length];
        stream.read_exact(&mut payload)?;

        // Parse JSON response
        let response: Value = serde_json::from_slice(&payload)?;

        if let Some(success) = response["success"].as_bool() {
            if !success {
                return Err(SwayError::IpcError(
                    response["error"]
                        .as_str()
                        .unwrap_or("Unknown error")
                        .to_string(),
                )
                .into());
            }
        }

        Ok(response)
    }
}
