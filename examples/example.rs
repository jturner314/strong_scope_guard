#[macro_use(scope)]
extern crate strong_scope_guard;

use strong_scope_guard::ScopeGuard;

struct Dma<S> {
    state: S,
}

struct Off {}

struct Running<'body, 'data: 'body> {
    data: &'data mut [u8],
    guard: ScopeGuard<'body, 'data, Option<fn()>>,
}

impl Dma<Off> {
    pub fn start<'body, 'data>(
        self,
        data: &'data mut [u8],
        mut guard: ScopeGuard<'body, 'data, Option<fn()>>,
    ) -> Dma<Running<'body, 'data>> {
        // start DMA
        guard.set_handler(Some(|| println!("stop DMA")));
        Dma {
            state: Running { data, guard },
        }
    }
}

impl<'body, 'data> Dma<Running<'body, 'data>> {
    pub fn stop(self) -> (Dma<Off>, &'data mut [u8]) {
        let Running { data, mut guard } = self.state;
        // stop DMA
        // Clear guard.
        guard.set_handler(Some(|| println!("stop DMA")));
        (Dma { state: Off {} }, data)
    }
}

fn usage1() {
    let dma = Dma { state: Off {} };
    let mut data = [1u8, 2, 3];
    let dma = scope!(|guard| {
        let dma = dma.start(&mut data, guard);
        let (dma, data) = dma.stop();
        println!("{}", data[0]);
        dma
    });
}

// fn usage2() {
//     let dma = Dma { state: Off {} };
//     let mut data = [1u8, 2, 3];
//     scope(|&mut (ref mut guard, ())| {
//         dma.start(&mut data, guard);
//         // must be stopped here
//     });
// }

// fn usage3() {
//     let dma = Dma { state: Off {} };
//     let data = [1u8, 2, 3];
//     let dma = scope(|&mut (ref mut guard, ())| {
//         let dma = dma.start(&data, guard);
//         dma
//     });
// }

// fn usage4() {
//     scope(|&mut (ref mut guard, ()): &mut (ScopeGuard<_>, ())| {
//         let x = [1, 2, 3];
//         guard.assign(Some(move || {
//             println!("{:?}", x);
//         }));
//     });
// }

// fn usage5() {
//     scope(|&mut (ref mut guard, ()): &mut (ScopeGuard<_>, ())| {
//         let view = ScopeGuardView { guard };
//         let x = [1, 2, 3];
//         view.guard.assign(Some(move || {
//             println!("{:?}", x);
//         }));
//     });
// }

fn main() {
    usage1();
}
