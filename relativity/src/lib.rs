pub use instant::Instant;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        pub type Timeout = wasm::WasmTimer;
    } else {
        pub type Timeout = default::Timer;
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::time::Duration;

    use gloo_timers::future::TimeoutFuture;

    pub struct WasmTimer(TimeoutFuture);

    impl WasmTimer {
        pub fn new(timeout: Duration) -> WasmTimer {
            WasmTimer(TimeoutFuture::new(timeout.as_millis() as u32))
        }
    }

    impl Future for WasmTimer {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            Pin::new(&mut self.0).poll(cx)
        }
    }

    // XXX: clicky requires Send futures. this is an "okay" hack since wasm is
    // single-threaded (for now), but should really be fixed...
    unsafe impl Send for WasmTimer {}
}

#[cfg(not(target_arch = "wasm32"))]
mod default {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::time::Duration;

    use async_timer::timer::Platform;

    pub struct Timer(Platform);

    impl Timer {
        pub fn new(timeout: Duration) -> Timer {
            Timer(Platform::new(timeout))
        }
    }

    impl Future for Timer {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            Pin::new(&mut self.0).poll(cx)
        }
    }
}
