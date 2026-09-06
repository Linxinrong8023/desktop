use serde::Serialize;
use serde_json::Value;
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Frame tag identifying a JSON-RPC payload inside Ora's plugin transport envelope.
pub const JSON_RPC_FRAME_TYPE: u8 = 0x01;
/// Largest complete plugin frame, including its one-byte type tag.
pub const MAX_FRAME_LENGTH: usize = 16 * 1024 * 1024;

/// Reads and decodes one complete JSON-RPC value, returning `None` only at a clean EOF.
pub async fn read_message<R>(reader: &mut R) -> io::Result<Option<Value>>
where
    R: AsyncRead + Unpin,
{
    let Some(payload) = read_frame(reader).await? else {
        return Ok(None);
    };
    serde_json::from_slice(&payload)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// Encodes and writes one JSON-RPC value as a complete plugin transport frame.
pub async fn write_message<W, Message>(writer: &mut W, message: &Message) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
    Message: Serialize + ?Sized,
{
    let payload = serde_json::to_vec(message)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write_frame(writer, &payload).await
}

/// Reads one length-delimited plugin protocol frame, returning `None` at a clean EOF.
async fn read_frame<R>(reader: &mut R) -> io::Result<Option<Vec<u8>>>
where
    R: AsyncRead + Unpin,
{
    let mut length_bytes = [0_u8; 4];
    match reader.read_u8().await {
        Ok(first_byte) => length_bytes[0] = first_byte,
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    reader.read_exact(&mut length_bytes[1..]).await?;

    let length = u32::from_be_bytes(length_bytes) as usize;
    if !(1..=MAX_FRAME_LENGTH).contains(&length) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("plugin frame length {length} is outside the supported range"),
        ));
    }

    let mut frame = vec![0_u8; length];
    reader.read_exact(&mut frame).await?;
    if frame[0] != JSON_RPC_FRAME_TYPE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported plugin frame type {}", frame[0]),
        ));
    }
    Ok(Some(frame.split_off(1)))
}

/// Writes one JSON payload using Ora's binary plugin frame envelope.
async fn write_frame<W>(writer: &mut W, payload: &[u8]) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let length = payload
        .len()
        .checked_add(1)
        .filter(|length| *length <= MAX_FRAME_LENGTH)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "plugin frame is too large"))?;
    let length = u32::try_from(length)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "plugin frame is too large"))?;

    writer.write_all(&length.to_be_bytes()).await?;
    writer.write_u8(JSON_RPC_FRAME_TYPE).await?;
    writer.write_all(payload).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use super::{JSON_RPC_FRAME_TYPE, MAX_FRAME_LENGTH, read_message, write_message};
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tokio::io::{AsyncWriteExt, duplex};

    /// Verifies a JSON value survives a fragmented asynchronous frame round trip.
    #[tokio::test]
    async fn round_trips_json_rpc_message() {
        let (mut writer, mut reader) = duplex(64);
        let expected = json!({ "jsonrpc": "2.0", "id": 1, "result": "ok" });
        let message = expected.clone();
        let write_task = tokio::spawn(async move { write_message(&mut writer, &message).await });

        assert_eq!(read_message(&mut reader).await.unwrap(), Some(expected));
        write_task.await.unwrap().unwrap();
    }

    /// Rejects unknown frame types instead of guessing how their payload should be handled.
    #[tokio::test]
    async fn rejects_unknown_frame_type() {
        let (mut writer, mut reader) = duplex(16);
        writer.write_all(&[0, 0, 0, 2, 0xff, b'{']).await.unwrap();

        assert_eq!(
            read_message(&mut reader).await.unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    }

    /// Rejects lengths above the protocol ceiling before allocating their declared buffer.
    #[tokio::test]
    async fn rejects_oversized_frame() {
        let (mut writer, mut reader) = duplex(8);
        let length = u32::try_from(MAX_FRAME_LENGTH + 1).unwrap();
        writer.write_all(&length.to_be_bytes()).await.unwrap();
        writer.write_u8(JSON_RPC_FRAME_TYPE).await.unwrap();

        assert_eq!(
            read_message(&mut reader).await.unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    }

    /// Treats a truncated header as corruption while preserving clean EOF as normal closure.
    #[tokio::test]
    async fn distinguishes_partial_header_from_clean_eof() {
        let (mut writer, mut reader) = duplex(8);
        writer.write_all(&[0, 0]).await.unwrap();
        writer.shutdown().await.unwrap();

        assert_eq!(
            read_message(&mut reader).await.unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }
}
