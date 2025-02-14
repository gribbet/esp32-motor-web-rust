use embassy_executor::Spawner;
use embassy_net::Stack;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::Duration;
use esp_alloc as _;
use esp_backtrace as _;
use esp_println::println;
use picoserve::extract;
use picoserve::extract::State;
use picoserve::response::{File, Json};
use picoserve::routing::{get, get_service, PathRouter};
use picoserve::Config;
use picoserve::{make_static, AppRouter, AppWithStateBuilder, Router, Timeouts};

use crate::motor::Motor;

const WEB_TASK_POOL_SIZE: usize = 8;

#[derive(Clone, Copy)]
struct SharedMotor(&'static Mutex<CriticalSectionRawMutex, Motor<'static>>);

struct App {}

impl AppWithStateBuilder for App {
    type State = SharedMotor;
    type PathRouter = impl PathRouter<Self::State>;

    fn build_app(self) -> Router<Self::PathRouter, Self::State> {
        Router::new()
            .route("/", get_service(File::html(include_str!("index.html"))))
            .route(
                "/speed",
                get(async |State(SharedMotor(led)): State<SharedMotor>| {
                    Json(led.lock().await.get_speed())
                })
                .post(
                    async |State(SharedMotor(led)): State<SharedMotor>,
                           extract::Json(value): extract::Json<f32>| {
                        println!("Set {}", value);
                        led.lock().await.set_speed(value).unwrap();
                        Json(value)
                    },
                ),
            )
    }
}

pub async fn start_server(spawner: Spawner, stack: Stack<'static>, motor: Motor<'static>) {
    let config = make_static!(
        Config::<Duration>,
        Config::new(Timeouts {
            start_read_request: Some(Duration::from_secs(5)),
            read_request: Some(Duration::from_secs(1)),
            write: Some(Duration::from_secs(1)),
        })
        .keep_connection_alive()
    );

    let motor =
        SharedMotor(make_static!(Mutex<CriticalSectionRawMutex, Motor<'_>>, Mutex::new(motor)));
    let app = make_static!(AppRouter<App>, App {}.build_app());

    for id in 0..WEB_TASK_POOL_SIZE {
        spawner.must_spawn(web_task(id, stack, app, config, motor));
    }
}

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
async fn web_task(
    id: usize,
    stack: Stack<'static>,
    app: &'static AppRouter<App>,
    config: &'static Config<Duration>,
    motor: SharedMotor,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    picoserve::listen_and_serve_with_state(
        id,
        app,
        config,
        stack,
        port,
        &mut tcp_rx_buffer,
        &mut tcp_tx_buffer,
        &mut http_buffer,
        &motor,
    )
    .await
}
