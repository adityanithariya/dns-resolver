use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls_pemfile::{certs, private_key};

use std::{
    fs::File,
    io::{self, BufReader},
};

pub fn get_tls_config() -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let cert_file = File::open("cert.pem")?;
    let mut cert_reader = BufReader::new(cert_file);

    let cert_chain: Vec<CertificateDer<'static>> =
        certs(&mut cert_reader).collect::<Result<_, _>>()?;

    let key_file = File::open("key.pem")?;
    let mut key_reader = BufReader::new(key_file);

    let private_key: PrivateKeyDer<'static> = private_key(&mut key_reader)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no private key found"))?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)?;

    Ok(config)
}
