use std::io::{Read, Write};

use serde::Serialize;

use crate::messages::SurfaceFrame;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Json = 1,
    Frame = 2,
}

#[derive(Debug)]
pub enum FramePacket {
    Json(Vec<u8>),
    Frame(SurfaceFrame),
}

pub fn write_json<W: Write + ?Sized, T: Serialize>(w: &mut W, value: &T) -> std::io::Result<()> {
    let body = serde_json::to_vec(value).map_err(std::io::Error::other)?;
    if body.len() > 64 * 1024 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "JSON control message exceeds 64 KiB",
        ));
    }
    write_raw(w, MessageKind::Json, &body)
}

pub fn write_frame<W: Write + ?Sized>(w: &mut W, frame: &SurfaceFrame) -> std::io::Result<()> {
    frame
        .validate()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut body = Vec::with_capacity(16 + frame.pixels.len());
    body.extend_from_slice(&frame.width.to_le_bytes());
    body.extend_from_slice(&frame.height.to_le_bytes());
    body.extend_from_slice(&frame.frame_id.to_le_bytes());
    body.extend_from_slice(&frame.pixels);
    write_raw(w, MessageKind::Frame, &body)
}

fn write_raw<W: Write + ?Sized>(w: &mut W, kind: MessageKind, body: &[u8]) -> std::io::Result<()> {
    let len = (1 + body.len()) as u32;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(&[kind as u8])?;
    w.write_all(body)?;
    w.flush()
}

pub fn read_message<R: Read>(r: &mut R) -> std::io::Result<FramePacket> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > crate::MAX_FRAME_BYTES + 64 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid message length",
        ));
    }
    let mut kind_buf = [0u8; 1];
    r.read_exact(&mut kind_buf)?;
    let kind = kind_buf[0];
    if kind == MessageKind::Json as u8 && len > 64 * 1024 + 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "JSON control message exceeds 64 KiB",
        ));
    }
    let mut buf = vec![0u8; len - 1];
    r.read_exact(&mut buf)?;
    let payload = &buf[..];
    match kind {
        x if x == MessageKind::Json as u8 => Ok(FramePacket::Json(payload.to_vec())),
        x if x == MessageKind::Frame as u8 => {
            if payload.len() < 16 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "frame header too short",
                ));
            }
            let width = u32::from_le_bytes(payload[0..4].try_into().unwrap());
            let height = u32::from_le_bytes(payload[4..8].try_into().unwrap());
            let frame_id = u64::from_le_bytes(payload[8..16].try_into().unwrap());
            let pixels = payload[16..].to_vec();
            let frame = SurfaceFrame {
                width,
                height,
                frame_id,
                pixels,
            };
            frame
                .validate()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(FramePacket::Frame(frame))
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unknown message kind",
        )),
    }
}
