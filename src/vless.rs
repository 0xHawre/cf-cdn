use bytes::{Buf, BytesMut};
use uuid::Uuid;
use crate::error::{CdnError, Result};

/// VLESS command types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    TCP = 1,
    UDP = 2,
}

impl TryFrom<u8> for Command {
    type Error = CdnError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Command::TCP),
            2 => Ok(Command::UDP),
            _ => Err(CdnError::VlessProtocol(format!("Unknown command: {}", value))),
        }
    }
}

/// Address type for VLESS protocol
#[derive(Debug, Clone)]
pub enum AddressType {
    IPv4,
    IPv6,
    DomainName,
}

impl TryFrom<u8> for AddressType {
    type Error = CdnError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(AddressType::IPv4),
            2 => Ok(AddressType::DomainName),
            3 => Ok(AddressType::IPv6),
            _ => Err(CdnError::VlessProtocol(format!("Unknown address type: {}", value))),
        }
    }
}

/// VLESS request header
#[derive(Debug, Clone)]
pub struct VlessRequest {
    pub uuid: Uuid,
    pub command: Command,
    pub address: String,
    pub port: u16,
}

impl VlessRequest {
    /// Parse VLESS request from bytes
    /// Format: [1 byte version] [16 bytes UUID] [1 byte command] [address] [2 bytes port]
    pub fn parse(mut buf: BytesMut) -> Result<(Self, BytesMut)> {
        if buf.len() < 24 {
            return Err(CdnError::VlessProtocol("Incomplete header".to_string()));
        }

        // Version (skip for now)
        let _version = buf.get_u8();

        // UUID (16 bytes)
        let uuid_bytes = buf.copy_to_bytes(16);
        let uuid = Uuid::from_bytes(uuid_bytes.as_ref().try_into().unwrap());

        // Command
        let cmd_byte = buf.get_u8();
        let command = Command::try_from(cmd_byte)?;

        // Address type + address
        let addr_type_byte = buf.get_u8();
        let addr_type = AddressType::try_from(addr_type_byte)?;

        let address = match addr_type {
            AddressType::IPv4 => {
                if buf.len() < 4 {
                    return Err(CdnError::VlessProtocol("Incomplete IPv4 address".to_string()));
                }
                let ip_bytes = buf.copy_to_bytes(4);
                format!(
                    "{}.{}.{}.{}",
                    ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]
                )
            }
            AddressType::IPv6 => {
                if buf.len() < 16 {
                    return Err(CdnError::VlessProtocol("Incomplete IPv6 address".to_string()));
                }
                let ip_bytes = buf.copy_to_bytes(16);
                // Simplified IPv6 formatting
                format!("{:02x?}", ip_bytes)
            }
            AddressType::DomainName => {
                if buf.is_empty() {
                    return Err(CdnError::VlessProtocol("Incomplete domain name".to_string()));
                }
                let len = buf.get_u8() as usize;
                if buf.len() < len {
                    return Err(CdnError::VlessProtocol("Incomplete domain name data".to_string()));
                }
                let domain_bytes = buf.copy_to_bytes(len);
                String::from_utf8(domain_bytes.to_vec())
                    .map_err(|e| CdnError::VlessProtocol(format!("Invalid domain: {}", e)))?
            }
        };

        // Port (2 bytes, big-endian)
        if buf.len() < 2 {
            return Err(CdnError::VlessProtocol("Incomplete port".to_string()));
        }
        let port = buf.get_u16();

        Ok((
            VlessRequest {
                uuid,
                command,
                address,
                port,
            },
            buf,
        ))
    }
}

/// Validate VLESS UUID against configured UUID
pub fn validate_uuid(request_uuid: &Uuid, configured_uuid: &str) -> Result<()> {
    let configured = Uuid::parse_str(configured_uuid)
        .map_err(|e| CdnError::VlessProtocol(format!("Invalid configured UUID: {}", e)))?;
    
    if *request_uuid != configured {
        return Err(CdnError::VlessProtocol("UUID mismatch".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parsing() {
        assert_eq!(Command::try_from(1).unwrap(), Command::TCP);
        assert_eq!(Command::try_from(2).unwrap(), Command::UDP);
    }

    #[test]
    fn test_address_type_parsing() {
        assert_eq!(AddressType::try_from(1).unwrap(), AddressType::IPv4);
        assert_eq!(AddressType::try_from(2).unwrap(), AddressType::DomainName);
        assert_eq!(AddressType::try_from(3).unwrap(), AddressType::IPv6);
    }
}
