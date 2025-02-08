use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_time::Duration;
use esp_alloc as _;
use esp_backtrace as _;
use picoserve::routing::{get, PathRouter};
use picoserve::{make_static, AppRouter, Router, Timeouts};
use picoserve::{AppBuilder, Config};

const WEB_TASK_POOL_SIZE: usize = 8;

struct AppProps {
    message: &'static str,
}

impl AppBuilder for AppProps {
    type PathRouter = impl PathRouter;

    fn build_app(self) -> Router<Self::PathRouter> {
        let Self { message } = self;

        Router::new().route("/", get(move || async move { message }))
    }
}

pub async fn start_server(spawner: Spawner, stack: Stack<'static>) {
    let config = make_static!(
        Config::<Duration>,
        Config::new(Timeouts {
            start_read_request: Some(Duration::from_secs(5)),
            read_request: Some(Duration::from_secs(1)),
            write: Some(Duration::from_secs(1)),
        })
        .keep_connection_alive()
    );

    let app = make_static!(
        AppRouter<AppProps>,
        AppProps {
            message: "Hello World"
        }
        .build_app()
    );

    for id in 0..WEB_TASK_POOL_SIZE {
        spawner.must_spawn(web_task(id, stack, app, config));
    }
}

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
async fn web_task(
    id: usize,
    stack: Stack<'static>,
    app: &'static AppRouter<AppProps>,
    config: &'static Config<Duration>,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    picoserve::listen_and_serve(
        id,
        app,
        config,
        stack,
        port,
        &mut tcp_rx_buffer,
        &mut tcp_tx_buffer,
        &mut http_buffer,
    )
    .await
}
