//! PAR-14 : le parser seul doit tenir >= 5000 mains/s sur un coeur du Celeron.
//! `just bench` (ou `cargo bench -p gr-parser-winamax`) mesure ce debit sur
//! le plus gros fixture reel du corpus (OBELISK, 328 mains, 3-max/re-entry).

use std::path::Path;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use gr_parser_winamax::{split_hand_blocks, WinamaxParser};

fn obelisk_hands() -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../fixtures/winamax/mtt/space-ko-3max-itm-reentry/20260923_OBELISK - TRIDENT SPACE KO(1173012730)_real_holdem_no-limit.txt",
    );
    let text = std::fs::read_to_string(path).expect("fixture OBELISK introuvable");
    let (blocks, _offset) = split_hand_blocks(&text);
    blocks.into_iter().map(str::to_string).collect()
}

fn bench_parse_hand(c: &mut Criterion) {
    let hands = obelisk_hands();
    let mut group = c.benchmark_group("parse_hand");
    group.throughput(Throughput::Elements(hands.len() as u64));
    group.bench_function("obelisk_328_hands", |b| {
        b.iter_batched(
            || hands.clone(),
            |hands| {
                for hand in &hands {
                    WinamaxParser::parse_hand(hand)
                        .expect("le fixture OBELISK doit toujours se parser");
                }
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(benches, bench_parse_hand);
criterion_main!(benches);
