use async_trait::async_trait;

#[derive(Debug)]
pub enum TransportError {
    Runtime,
    IO(std::io::Error),
    ConnectionClosed,
}

#[async_trait]
pub trait TransportPeer: Send + Sync {
    async fn bye(self) -> Result<(), TransportError>;
    async fn send(&self, msg: Vec<u8>) -> Result<(), TransportError>;
    async fn recv(&self) -> Result<Vec<u8>, TransportError>;
}

#[async_trait]
pub trait Server: Send + Sync {
    async fn accept(&self) -> Result<Option<Box<dyn TransportPeer>>, TransportError>;
}

#[async_trait]
pub trait Client: Send + Sync {
    async fn connect(&self, addr: &str) -> Result<Box<dyn TransportPeer>, TransportError>;
}