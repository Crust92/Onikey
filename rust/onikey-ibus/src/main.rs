//! onikey-engine-rs — engine IBus của Onikey viết bằng Rust.
//!
//! Chạy SONG SONG với engine Go: khác tên thành phần, khác tên engine, nên cài
//! cả hai rồi đổi qua lại bằng cách chọn nguồn nhập. Bản Rust hỏng thì chỉ cần
//! chọn lại "Onikey" là gõ tiếp được — đó là điều kiện để dám thay dần.

mod bus;
mod engine;
mod ibus_text;

use std::sync::atomic::{AtomicU64, Ordering};

use zbus::connection::Builder;
use zbus::interface;
use zvariant::OwnedObjectPath;

const COMPONENT_NAME: &str = "org.freedesktop.IBus.OnikeyRust";
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";

struct Factory {
    conn: zbus::Connection,
    counter: AtomicU64,
}

#[interface(name = "org.freedesktop.IBus.Factory")]
impl Factory {
    async fn create_engine(&self, engine_name: String) -> zbus::fdo::Result<OwnedObjectPath> {
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        let path = format!("/org/freedesktop/IBus/Engine/onikeyrust/{n}");
        let obj = OwnedObjectPath::try_from(path.clone())
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        eprintln!("tạo engine: {engine_name} -> {path}");
        let e = engine::OnikeyEngine::new("Telex", engine::default_flags());
        self.conn
            .object_server()
            .at(&obj, e)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        Ok(obj)
    }

    async fn destroy(&self) {}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let embedded = std::env::args().any(|a| a == "--ibus");
    let address = bus::address()?;
    eprintln!("nối tới ibus: {address}");

    let conn = Builder::address(address.as_str())?
        .name(COMPONENT_NAME)?
        .build()
        .await?;

    let factory = Factory {
        conn: conn.clone(),
        counter: AtomicU64::new(0),
    };
    conn.object_server().at(FACTORY_PATH, factory).await?;

    eprintln!(
        "onikey-engine-rs sẵn sàng (chế độ {}).",
        if embedded { "ibus" } else { "độc lập" }
    );
    tokio::signal::ctrl_c().await?;
    Ok(())
}
