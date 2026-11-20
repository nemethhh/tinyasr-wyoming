use std::net::TcpListener;
use std::process::exit;
use std::time::Instant;

use tinyasr_wyoming::config::Config;
use tinyasr_wyoming::server::{load_models, serve};

fn main() {
    let cfg = match Config::from_args(std::env::args().skip(1)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("tinyasr-wyoming: {e}");
            exit(2);
        }
    };
    let t0 = Instant::now();
    let models = match load_models(&cfg.models_dir, cfg.threads) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("tinyasr-wyoming: cannot load the models: {e}");
            exit(1);
        }
    };
    let models: &'static tinyasr::Models = Box::leak(Box::new(models));
    let cfg: &'static Config = Box::leak(Box::new(cfg));
    let listener = match TcpListener::bind(&cfg.listen) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("tinyasr-wyoming: cannot listen on {}: {e}", cfg.listen);
            exit(1);
        }
    };
    eprintln!(
        "tinyasr-wyoming {} listening on {}: languages {}, models loaded in {} ms, {} threads, beam {}, end of speech {:?}",
        env!("CARGO_PKG_VERSION"),
        cfg.listen,
        models.langs().join(", "),
        t0.elapsed().as_millis(),
        cfg.threads,
        cfg.turn.beam,
        cfg.turn.end_of_speech,
    );
    serve(listener, models, cfg);
}
