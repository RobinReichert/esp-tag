
pub struct WebServer {
}

impl WebServer {
    pub fn new() {
    }

    pub fn init() {

    }

}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}
