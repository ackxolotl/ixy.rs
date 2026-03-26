use std::collections::VecDeque;
use std::env;
use std::process;
use std::time::Instant;

use ixy::memory::Packet;
use ixy::*;
use simple_logger::SimpleLogger;

const BATCH_SIZE: usize = 32;

pub fn main() {
    SimpleLogger::new().init().unwrap();

    let mut args = env::args();
    args.next();

    let pci_addr = match args.next() {
        Some(arg) => arg,
        None => {
            eprintln!("Usage: cargo run --example echo <pci bus id>");
            process::exit(1);
        }
    };

    let mut dev = ixy_init(&pci_addr, 1, 1, 0).unwrap();

    let mut dev_stats = Default::default();
    let mut dev_stats_old = Default::default();

    dev.reset_stats();

    dev.read_stats(&mut dev_stats);
    dev.read_stats(&mut dev_stats_old);

    let mut buffer: VecDeque<Packet> = VecDeque::with_capacity(BATCH_SIZE);
    let mut time = Instant::now();
    let mut counter = 0;

    loop {
        // receive packets and send them back out the same NIC
        let num_rx = dev.rx_batch(0, &mut buffer, BATCH_SIZE);

        if num_rx > 0 {
            // swap src and dst MAC addresses
            for p in buffer.iter_mut() {
                // dst MAC (bytes 0..6) <-> src MAC (bytes 6..12)
                for i in 0..6 {
                    p.swap(i, i + 6);
                }
            }

            dev.tx_batch(0, &mut buffer);

            // drop packets that haven't been sent out
            buffer.drain(..);
        }

        // don't poll the time unnecessarily
        if counter & 0xfff == 0 {
            let elapsed = time.elapsed();
            let nanos = elapsed.as_secs() * 1_000_000_000 + u64::from(elapsed.subsec_nanos());
            // every second
            if nanos > 1_000_000_000 {
                dev.read_stats(&mut dev_stats);
                dev_stats.print_stats_diff(&*dev, &dev_stats_old, nanos);
                dev_stats_old = dev_stats;

                time = Instant::now();
            }
        }

        counter += 1;
    }
}