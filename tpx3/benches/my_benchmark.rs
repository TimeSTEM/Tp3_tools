use criterion::{black_box, criterion_group, criterion_main, Criterion};
use timepix3::spimlib;
    
//Electron Packets (0 and >500 ms)
//[2, 0, 109, 131, 230, 16, 101, 178]
//[197, 4, 199, 0, 51, 167, 17, 180]
//Tdc Packets (0 and >500 ms)
//[64, 188, 207, 130, 5, 128, 200, 111]
//[96, 70, 153, 115, 31, 32, 120, 111]



fn criterion_benchmark(c: &mut Criterion) {
    let electron_chunk = [84, 80, 88, 51, 0, 0, 0, 0, 2, 0, 109, 131, 230, 16, 101, 178, 197, 4, 199, 0, 51, 167, 17, 180];
    //let electron_chunk = [2, 0, 109, 131, 230, 16, 101, 178];
    let tdc_chunk = [84, 80, 88, 51, 0, 0, 0, 0, 64, 188, 207, 130, 5, 128, 200, 111, 96, 70, 153, 115, 31, 32, 120, 111];
    //let tdc_chunk = [64, 188, 207, 130, 5, 128, 200, 111];
    
    c.bench_function("spim_electron", |b| b.iter(|| spimlib::debug_build_spim_data(black_box(&electron_chunk))));
    c.bench_function("spim_tdc", |b| b.iter(|| spimlib::debug_build_spim_data(black_box(&tdc_chunk))));
    c.bench_function("spim_multithread", |b| b.iter(|| spimlib::debug_multithread(black_box(electron_chunk))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
