use async_std::io;
use futures::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

impl From<Request> for Message {
    fn from(request: Request) -> Message {
        Message::Request(request)
    }
}

impl From<Response> for Message {
    fn from(response: Response) -> Message {
        Message::Response(response)
    }
}

impl From<Notification> for Message {
    fn from(notification: Notification) -> Message {
        Message::Notification(notification)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct RequestId(Id);

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(untagged)]
enum Id {
    U64(u64),
    String(String),
}

impl From<u64> for RequestId {
    fn from(id: u64) -> RequestId {
        RequestId(Id::U64(id))
    }
}

impl From<String> for RequestId {
    fn from(id: String) -> RequestId {
        RequestId(Id::String(id))
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            Id::U64(id) => write!(f, "{}", id),
            Id::String(id) => write!(f, "\"{}\"", id),
        }
    }
}

/// A request message to describe a request between the client and the server. Every processed
/// request must send a response back to the sender of the request.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Request {
    pub id: RequestId,
    pub method: String,
    pub params: serde_json::Value,
}

/// A Response Message sent as a result of a `Request`. If a request doesn’t provide a result value
/// the receiver of a request still needs to return a response message to conform to the JSON RPC
/// specification. The result property of the ResponseMessage should be set to null in this case to
/// signal a successful request.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Response {
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

/// An error object in case a request failed.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// An error code indicating the error type that occurred.
#[derive(Clone, Copy, Debug)]
#[allow(unused)]
pub enum ErrorCode {
    // Defined by JSON RPC
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    ServerErrorStart = -32099,
    ServerErrorEnd = -32000,
    ServerNotInitialized = -32002,
    UnknownErrorCode = -32001,

    // Defined by the protocol.
    RequestCanceled = -32800,
    ContentModified = -32801,
}

/// A notification message. A processed notification message must not send a response back. They
/// work like events.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Notification {
    pub method: String,
    pub params: serde_json::Value,
}

impl Message {
    /// Reads an RPC message from the given stream
    pub async fn read<R: AsyncBufRead + Unpin>(stream: &mut R) -> io::Result<Option<Message>> {
        let text = match read_message_string(stream).await? {
            None => return Ok(None),
            Some(text) => text,
        };
        Ok(Some(serde_json::from_str(&text)?))
    }

    /// Writes the RPC message to the given stream
    pub async fn write<R: AsyncWrite + Unpin>(self, stream: &mut R) -> io::Result<()> {
        #[derive(Serialize)]
        struct RpcMessage {
            jsonrpc: &'static str,
            #[serde(flatten)]
            msg: Message,
        }
        let text = serde_json::to_string(&RpcMessage {
            jsonrpc: "2.0",
            msg: self,
        })?;
        write_message_string(stream, &text).await
    }
}

impl Response {
    /// Constructs a `Response` object signaling the succesfull handling of a request with the
    /// specified id.
    pub fn new_ok<R: Serialize>(id: RequestId, result: R) -> Self {
        Self {
            id,
            result: Some(serde_json::to_value(result).unwrap()),
            error: None,
        }
    }

    /// Constructs a `Response` object signalling failure to handle the request with the specified
    /// id
    pub fn new_err(id: RequestId, code: i32, message: String) -> Self {
        Self {
            id,
            result: None,
            error: Some(ResponseError {
                code,
                message,
                data: None,
            }),
        }
    }
}

impl Request {
    /// Constructs a new Request object
    pub fn new<P: Serialize>(id: RequestId, method: String, params: P) -> Self {
        Self {
            id,
            method,
            params: serde_json::to_value(params).unwrap(),
        }
    }

    /// Tries to extract the specific request parameters from this request.
    pub fn try_extract<P: DeserializeOwned>(self, method: &str) -> Result<(RequestId, P), Request> {
        if self.method == method {
            let params = serde_json::from_value(self.params).unwrap_or_else(|err| {
                panic!("Invalid request\nMethod: {}\nerror: {}", method, err)
            });
            Ok((self.id, params))
        } else {
            Err(self)
        }
    }

    pub(crate) fn is_shutdown(&self) -> bool {
        self.method == "shutdown"
    }

    pub(crate) fn is_initialize(&self) -> bool {
        self.method == "initialize"
    }
}

impl Notification {
    /// Constructs a new `Notification` from the specified method name and parameters
    pub fn new<P: Serialize>(method: String, params: P) -> Self {
        Self {
            method,
            params: serde_json::to_value(params).unwrap(),
        }
    }

    /// Tries to extract the specific notification parameters from this notification.
    pub fn try_extract<P: DeserializeOwned>(self, method: &str) -> Result<P, Notification> {
        if self.method == method {
            let params = serde_json::from_value(self.params).unwrap_or_else(|err| {
                panic!("Invalid request\nMethod: {}\nerror: {}", method, err)
            });
            Ok(params)
        } else {
            Err(self)
        }
    }

    pub(crate) fn is_exit(&self) -> bool {
        self.method == "exit"
    }
    pub(crate) fn is_initialized(&self) -> bool {
        self.method == "initialized"
    }
}

/// Reads an RPC message from the specified stream.
async fn read_message_string<R: AsyncBufRead + Unpin>(
    stream: &mut R,
) -> io::Result<Option<String>> {
    /// Constructs an `InvalidData` error with a cause
    fn invalid_data(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, error)
    }

    // Loop over all headers of the incoming message.
    let mut size = None;
    let mut buf = String::new();
    loop {
        buf.clear();
        if stream.read_line(&mut buf).await? == 0 {
            return Ok(None);
        }
        if !buf.ends_with("\r\n") {
            return Err(invalid_data(format!("malformed header: {:?}", buf)));
        }

        // If there are no more headers, break to parse the rest of the message
        let buf = &buf[..buf.len() - 2];
        if buf.is_empty() {
            break;
        }

        // If this is the `Content-Length` header, parse the size of the message
        let mut parts = buf.splitn(2, ": ");
        let header_name = parts.next().unwrap();
        let header_value = parts
            .next()
            .ok_or_else(|| invalid_data(format!("malformed header: {:?}", buf)))?;
        if header_name == "Content-Length" {
            size = Some(header_value.parse::<usize>().map_err(invalid_data)?);
        }
    }

    let size: usize = size.ok_or_else(|| invalid_data("no Content-Length".to_owned()))?;
    let mut buf = buf.into_bytes();
    buf.resize(size, 0);
    stream.read_exact(&mut buf).await?;
    let buf = String::from_utf8(buf).map_err(invalid_data)?;
    log::debug!("< {}", buf);
    Ok(Some(buf))
}

/// Writes an RPC message to the specified stream.
async fn write_message_string<R: AsyncWrite + Unpin>(stream: &mut R, msg: &str) -> io::Result<()> {
    log::debug!("> {}", msg);
    let header = format!("Content-Length: {}\r\n\r\n", msg.len());
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(msg.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}
