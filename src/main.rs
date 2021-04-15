use anyhow::{anyhow, Error, Result};
use polygon::ws::Quote;
use serde::Serialize;
use std::borrow::Cow;
use stream_processor::{StreamProcessor, StreamRunner};

mod settings;
use settings::Settings;

struct MicroPricer;

#[derive(Debug, Serialize)]
struct MicroPrice {
    ticker: String,
    timestamp: u64,
    midprice: f64,
    microprice: f64,
}

fn calculate_microprice(ask_price: f64, ask_size: u32, bid_price: f64, bid_size: u32) -> f64 {
    let total_size = bid_size as f64 + ask_size as f64;
    ask_price * bid_size as f64 / total_size + bid_price * ask_size as f64 / total_size
}

#[cfg(test)]
#[test]
fn test_microprice() {
    assert_eq!(calculate_microprice(101.0, 100, 98.0, 50), 99.0)
}

#[async_trait::async_trait]
impl StreamProcessor for MicroPricer {
    type Input = Quote;
    type Output = MicroPrice;
    type Error = Error;

    async fn handle_message(&self, input: Self::Input) -> Result<Option<Vec<Self::Output>>> {
        let bid_quote = input
            .bid_quote
            .ok_or_else(|| anyhow!("Missing bid quote"))?;
        let ask_quote = input
            .ask_quote
            .ok_or_else(|| anyhow!("Missing ask quote"))?;
        let mp = MicroPrice {
            ticker: input.symbol,
            timestamp: input.timestamp,
            midprice: (bid_quote.price + ask_quote.price) / 2.0,
            microprice: calculate_microprice(
                ask_quote.price,
                ask_quote.size,
                bid_quote.price,
                bid_quote.size,
            ),
        };
        Ok(Some(vec![mp]))
    }

    fn assign_key(&self, output: &Self::Output) -> Cow<str> {
        output.ticker.clone().into()
    }

    fn assign_topic(&self, _output: &Self::Output) -> Cow<str> {
        "microprice".into()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();
    let settings = Settings::new()?;
    let processor = MicroPricer;
    let runner = StreamRunner::new(processor, settings.kafka);
    runner.run().await.map_err(From::from)
}
