use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() {
    let server = float_view_loopback::start("app://float-view", "smoke")
        .await
        .expect("bind loopback");
    let mut stream = tokio::net::TcpStream::connect(server.address)
        .await
        .expect("connect loopback");
    let request = format!(
        "GET /api/bootstrap HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        server.address
    );
    stream
        .write_all(request.as_bytes())
        .await
        .expect("write request");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .await
        .expect("read response");
    assert!(response.starts_with("HTTP/1.1 200"));
    server.stop().await;
    println!("float-view-loopback-smoke ok");
}
