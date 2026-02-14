use binance::websockets::*;
use std::sync::atomic::AtomicBool;

fn main() {
    let endpoints =
        // ["ETHBTC", "BNBETH"].map(|symbol| format!("{}@kline_1m", symbol.to_lowercase()));
    ["ETHBTC", "ETHUSDT", "BTCUSDT"].map(|symbol| format!("{}@bookTicker", symbol.to_lowercase()));
    // ["ETHBTC", "BNBETH"].map(|symbol| format!("{}@depth@100ms", symbol.to_lowercase()));

    let keep_running = AtomicBool::new(true);
    let mut web_socket = WebSockets::new(|event: WebsocketEvent| {
        // if let WebsocketEvent::Kline(kline_event) = event {
        //     println!(
        //         "Symbol: {}, high: {}, low: {}",
        //         kline_event.kline.symbol, kline_event.kline.low, kline_event.kline.high
        //     );
        if let WebsocketEvent::BookTicker(book_ticker) = event {
            println!("{:?}", book_ticker);
            //     // if let WebsocketEvent::DepthOrderBook(depth_order_book) = event {
            //     //     println!("{:?}", depth_order_book);
        }

        Ok(())
    });

    web_socket.connect_multiple_streams(&endpoints).unwrap(); // check error
    if let Err(e) = web_socket.event_loop(&keep_running) {
        println!("Error: {:?}", e);
    }
    web_socket.disconnect().unwrap();
}
