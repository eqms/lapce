#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use lapce_app::app;

pub fn main() {
    // RT-01: ambient runtime for the process lifetime;
    // guard wraps launch() (D-01/D-02)
    let _rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("lapce-app-worker")
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("failed to build tokio runtime: {e:?}");
            eprintln!("lapce: failed to build tokio runtime: {e}");
            std::process::exit(1);
        }
    };
    let _guard = _rt.enter();
    app::launch();
}
