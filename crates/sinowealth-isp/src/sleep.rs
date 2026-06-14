//! Cross-target asynchronous delay.
//!
//! The ISP protocol needs to wait for the device to settle after erase and
//! reboot. This module provides a single `sleep` future that works on every
//! target the crate supports without pulling in a full async runtime:
//!
//! - native: a background thread sleeps and then wakes the task,
//! - wasm: a browser `setTimeout` resolved through `wasm-bindgen-futures`.

use core::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(duration: Duration) {
    native::Sleep::new(duration).await
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll, Waker};
    use core::time::Duration;
    use std::sync::{Arc, Mutex};

    struct Shared {
        done: bool,
        waker: Option<Waker>,
    }

    pub struct Sleep {
        duration: Duration,
        started: bool,
        shared: Arc<Mutex<Shared>>,
    }

    impl Sleep {
        pub fn new(duration: Duration) -> Self {
            Self {
                duration,
                started: false,
                shared: Arc::new(Mutex::new(Shared {
                    done: false,
                    waker: None,
                })),
            }
        }
    }

    impl Future for Sleep {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            let this = self.as_mut().get_mut();

            if !this.started {
                this.started = true;
                let shared = this.shared.clone();
                let duration = this.duration;
                std::thread::spawn(move || {
                    std::thread::sleep(duration);
                    let mut guard = shared.lock().unwrap();
                    guard.done = true;
                    if let Some(waker) = guard.waker.take() {
                        waker.wake();
                    }
                });
            }

            let mut guard = this.shared.lock().unwrap();
            if guard.done {
                Poll::Ready(())
            } else {
                guard.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep(duration: Duration) {
    let millis = duration.as_millis() as i32;
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let window = web_sys::window().expect("no global window available");
        window
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis)
            .expect("failed to schedule setTimeout");
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}
