use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::process::Command;
use std::time::Duration;

#[test]
fn test_serve_absolute_path_traversal() {
    let bin = env!("CARGO_BIN_EXE_cargo-slide");

    // Start server in background
    let mut child = Command::new(bin)
        .arg("serve")
        .arg(".") // serve current dir
        .arg("--port")
        .arg("9999")
        .spawn()
        .expect("Failed to start cargo-slide serve");

    // Wait for server to start
    std::thread::sleep(Duration::from_millis(1500));

    let req = "GET /C:/Windows/win.ini HTTP/1.1\r\nHost: localhost\r\n\r\n";
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:9999") {
        stream.write_all(req.as_bytes()).unwrap();

        let mut buf = [0; 512];
        let bytes_read = stream.read(&mut buf).unwrap_or(0);
        let response = String::from_utf8_lossy(&buf[..bytes_read]);

        // Ensure 403 Forbidden is returned
        assert!(response.contains("403 Forbidden") || response.contains("404 Not Found"));
    }

    let req2 = "GET /\\etc\\passwd HTTP/1.1\r\nHost: localhost\r\n\r\n";
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:9999") {
        stream.write_all(req2.as_bytes()).unwrap();

        let mut buf = [0; 512];
        let bytes_read = stream.read(&mut buf).unwrap_or(0);
        let response = String::from_utf8_lossy(&buf[..bytes_read]);

        // Ensure 403 Forbidden is returned
        assert!(response.contains("403 Forbidden") || response.contains("404 Not Found"));
    }

    // Cleanup
    let _ = child.kill();
    let _ = child.wait();
}
