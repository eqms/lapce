use lapce_proxy::mainloop;

fn main() {
    // RT-01: ambient runtime for the process lifetime;
    // guard wraps mainloop() (D-01/D-02)
    let _rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("lapce-proxy-worker")
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("failed to build tokio runtime: {e:?}");
            eprintln!("lapce-proxy: failed to build tokio runtime: {e}");
            std::process::exit(1);
        }
    };
    let _guard = _rt.enter();
    mainloop();
}
