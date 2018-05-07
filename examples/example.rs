extern crate strong_scope_guard;

use strong_scope_guard::{scope, ScopeGuard};

struct Dma<S> {
    state: S,
}

struct Off {}

struct Running<'guard, 'data: 'guard> {
    data: &'data mut [u8],
    guard: &'guard mut ScopeGuard<'data, fn()>,
}

impl Dma<Off> {
    pub fn start<'guard, 'data>(
        self,
        data: &'data mut [u8],
        guard: &'guard mut ScopeGuard<'data, fn()>,
    ) -> Dma<Running<'guard, 'data>> {
        // start DMA
        guard.assign(Some(|| println!("stop DMA")));
        Dma {
            state: Running { data, guard },
        }
    }
}

impl<'guard, 'data> Dma<Running<'guard, 'data>> {
    pub fn stop(self) -> (Dma<Off>, &'data mut [u8]) {
        // stop DMA
        // Clear guard.
        self.state.guard.assign(None);
        (Dma { state: Off {} }, self.state.data)
    }
}

fn usage1() {
    let dma = Dma { state: Off {} };
    let mut data = [1u8, 2, 3];
    let dma = scope(|&mut (ref mut guard, ())| {
        let dma = dma.start(&mut data, guard);
        let (dma, data) = dma.stop();
        println!("{}", data[0]);
    });
}

fn usage2() {
    let dma = Dma { state: Off {} };
    let mut data = [1u8, 2, 3];
    scope(|&mut (ref mut guard, ())| {
        dma.start(&mut data, guard);
        // must be stopped here
    });
}

// fn usage3() {
//     let dma = Dma { state: Off {} };
//     let data = [1u8, 2, 3];
//     let dma = scope(|&mut (ref mut guard, ())| {
//         let dma = dma.start(&data, guard);
//         dma
//     });
// }

fn main() {
    usage1();
    usage2();
}
