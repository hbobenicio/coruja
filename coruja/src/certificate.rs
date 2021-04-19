use std::net::TcpStream;

use anyhow::{anyhow, Context, Result};

use openssl::nid::Nid;
use openssl::ssl::{SslConnector, SslConnectorBuilder, SslMethod, SslStream, SslVerifyMode};
use openssl::stack::StackRef;
use openssl::x509::{X509Ref, X509};

// TODO make this async!
// tokio-openssl and openssl-async crates may help
pub fn get_server_cert_chain(host: &str, port: &str, insecure: bool) -> Result<Vec<String>> {
    let connector: SslConnector = new_ssl_connector(insecure)?;

    let url = format!("{}:{}", host, port);
    let stream: TcpStream = TcpStream::connect(&url).context("io")?;

    let stream: SslStream<TcpStream> = connector
        .connect(&url, stream)
        .map_err(|openssl_err| anyhow!("openssl: handshake: {}", openssl_err))?;

    let cert_stack: &StackRef<X509> = stream.ssl().peer_cert_chain().ok_or(anyhow!(
        "it was not possible to get certificate chain from server"
    ))?;

    let mut pem_list: Vec<String> = Vec::with_capacity(cert_stack.len());
    for cert in cert_stack.iter().flat_map(X509Ref::to_pem) {
        pem_list.push(String::from_utf8(cert)?);
    }
    Ok(pem_list)
}

/// Get a vec of certificates from a https server for a given url.
// TODO Make this function async!
pub fn get_certs(url: &str, insecure: bool) -> Result<Vec<X509>> {
    let connector: SslConnector = new_ssl_connector(insecure)?;

    let stream: TcpStream = TcpStream::connect(&url).context("io")?;

    let stream: SslStream<TcpStream> = connector
        .connect(&url, stream)
        .map_err(|openssl_err| anyhow!("openssl: handshake: {}", openssl_err))?;

    let cert_stack: &StackRef<X509> = stream.ssl().peer_cert_chain().ok_or(anyhow!(
        "it was not possible to get certificate chain from server"
    ))?;

    let certs: Vec<X509> = cert_stack.iter().map(X509Ref::to_owned).collect();

    Ok(certs)
}

/// Creates a new SSL connector
fn new_ssl_connector(insecure: bool) -> Result<SslConnector> {
    let mut connector_builder: SslConnectorBuilder =
        SslConnector::builder(SslMethod::tls()).context("openssl")?;

    if insecure {
        connector_builder.set_verify(SslVerifyMode::NONE);
    } else {
        connector_builder
            .set_default_verify_paths()
            .map_err(|openssl_error_stack| {
                anyhow!(
                    "openssl: connector builder: {:?}",
                    openssl_error_stack.errors()
                )
            })?;
    }

    Ok(connector_builder.build())
}

/// Returns the common name of the certificate
fn _cert_common_name(cert: &X509) -> Result<String> {
    for name_entry in cert.subject_name().entries() {
        let asn1_object = name_entry.object();
        if asn1_object.nid() == Nid::COMMONNAME {
            return name_entry
                .data()
                .as_utf8()
                .map(|openssl_str| openssl_str.to_string())
                .map_err(|openssl_error_stack| {
                    anyhow!("openssl: utf-8 parsing: {:?}", openssl_error_stack.errors())
                });
        }
    }

    Err(anyhow!("common name not found"))
}
