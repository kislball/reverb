use futures::sink::SinkExt;
use rvb_common::transport::{TransportError, TransportPeer};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub struct TcpPeer {
    stream: Mutex<Framed<TcpStream, LengthDelimitedCodec>>,
    shutdown: RwLock<bool>,
}

impl TcpPeer {
    pub async fn is_open(&self) -> bool {
        *self.shutdown.read().await
    }

    pub async fn must_be_open(&self) -> Result<(), TransportError> {
        if !self.is_open().await {
            return Err(TransportError::ConnectionClosed);
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl TransportPeer for TcpPeer {
    async fn bye(self) -> Result<(), TransportError> {
        self.must_be_open().await?;
        
        self.stream
            .lock()
            .await
            .get_mut()
            .shutdown()
            .await
            .map_err(TransportError::IO)
    }

    async fn send(&self, msg: Vec<u8>) -> Result<(), TransportError> {
        self.must_be_open().await?;
        
        self.stream
            .lock()
            .await
            .send(msg.into())
            .await
            .map_err(TransportError::IO)
    }

    async fn recv(&self) -> Result<Vec<u8>, TransportError> {
        self.must_be_open().await?;
        
        self.stream
            .lock()
            .await
            .next()
            .await
            .ok_or(TransportError::Runtime)?
            .map_err(TransportError::IO)
            .map(Into::into)
    }
}
