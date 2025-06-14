#[cfg(feature = "crypto")]
use crate::crypto::{CryptoError, KeyPair, PublicKey};
use crate::schema::DbValue;
#[cfg(feature = "crypto_random")]
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[cfg(feature = "crypto")]
    #[error("Cryptography error {0}")]
    Crypto(CryptoError),
    #[error("Schema error {0}")]
    Schema(rmp_serde::decode::Error),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Location {
    pub namespace: String,
    pub contract_space: String,
    pub contract: Vec<u8>,
    pub key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Hello {
        public_key: Vec<u8>,
    },
    WhoAreYou {
        data: Vec<u8>,
        public_key: Vec<u8>,
    },
    ItsMe {
        signature: Vec<u8>,
        data: Vec<u8>,
    },
    Welcome {
        dht_ip: String,
        dht_port: u16,
        signature: Vec<u8>,
    },
    Insert {
        location: Location,
        incoming_data: DbValue,
        metadata: HashMap<String, DbValue>,
        state: u64,
    },
    Get {
        location: Location,
        select: Vec<Vec<String>>,
    },
    DeployContract {
        contract_payload: Vec<u8>,
        namespace: String,
        params: HashMap<String, DbValue>,
        tags: Vec<String>,
    },
    SearchTags {
        namespace: String,
        query: Vec<String>,
    },
    Gossip {
        peers: HashMap<Vec<u8>, Vec<Vec<u8>>>,
    },
}

#[cfg(feature = "crypto")]
impl Message {
    pub fn sign(
        &self,
        key: &mut KeyPair,
        publisher: String,
        #[cfg(not(feature = "crypto_random"))] id: Vec<u8>,
    ) -> TransportMessage {
        #[cfg(feature = "crypto_random")]
        return TransportMessage::sign(&vec![self.clone()], key, publisher);
        #[cfg(not(feature = "crypto_random"))]
        return TransportMessage::sign(&vec![self.clone()], key, publisher, id);
    }
}

#[cfg(not(feature = "crypto"))]
impl TryFrom<TransportMessage> for Message {
    type Error = ProtocolError;

    fn try_from(value: TransportMessage) -> Result<Self, Self::Error> {
        rmp_serde::from_slice(&value.data).map_err(ProtocolError::Schema)
    }
}

#[cfg(feature = "crypto")]
impl TransportMessage {
    pub fn sign(
        messages: &[Message],
        key: &mut KeyPair,
        publisher: String,
        #[cfg(not(feature = "crypto_random"))] id: Vec<u8>,
    ) -> TransportMessage {
        let bin = rmp_serde::to_vec(messages).unwrap();
        let signature = key.sign(&bin);

        #[cfg(feature = "crypto_random")]
        let id = {
            let mut buf = vec![0u8; 64];
            rand::thread_rng().fill_bytes(&mut buf);
            buf.to_vec()
        };

        TransportMessage {
            signature: MessageSignature {
                signed_by: key.export_public(),
                data: signature,
            },
            id,
            data: bin,
            publisher,
            received_by: Vec::new(),
        }
    }
}

#[cfg(feature = "crypto")]
impl TryFrom<TransportMessage> for Vec<Message> {
    type Error = ProtocolError;

    fn try_from(value: TransportMessage) -> Result<Self, Self::Error> {
        let public_key =
            PublicKey::import(&value.signature.signed_by).map_err(ProtocolError::Crypto)?;
        let result = public_key.verify(&value.data, &value.signature.data);

        if result {
            rmp_serde::from_slice(&value.data).map_err(ProtocolError::Schema)
        } else {
            Err(ProtocolError::Crypto(CryptoError::InvalidKey))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageSignature {
    pub data: Vec<u8>,
    pub signed_by: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransportMessage {
    #[cfg(feature = "crypto")]
    data: Vec<u8>,
    #[cfg(not(feature = "crypto"))]
    pub data: Vec<u8>,
    pub signature: MessageSignature,
    pub publisher: String,
    pub received_by: Vec<Vec<u8>>,
    pub id: Vec<u8>,
}
