extern crate strong_scope_guard;

use strong_scope_guard::{scope, LocalScopeGuard, ScopeGuard, StaticScopeGuard};

struct Dma<S> {
    state: S,
}

struct Off {}

struct Running<'data, G>
where
    G: ScopeGuard<'data, Handler = Option<fn()>>,
{
    data: &'data mut [u8],
    guard: G,
}

impl Dma<Off> {
    pub fn start<'data, G>(self, data: &'data mut [u8], mut guard: G) -> Dma<Running<'data, G>>
    where
        G: ScopeGuard<'data, Handler = Option<fn()>>,
    {
        // start DMA
        *guard.handler_mut() = Some(|| println!("stop DMA"));
        Dma {
            state: Running { data, guard },
        }
    }
}

impl<'data, G> Dma<Running<'data, G>>
where
    G: ScopeGuard<'data, Handler = Option<fn()>>,
{
    pub fn stop(self) -> (Dma<Off>, &'data mut [u8], G) {
        let Running { data, mut guard } = self.state;
        // stop DMA
        // Clear guard.
        *guard.handler_mut() = Some(|| println!("stop DMA"));
        (Dma { state: Off {} }, data, guard)
    }
}

fn usage1() {
    let dma = Dma { state: Off {} };
    let mut data = [1u8, 2, 3];
    let dma = scope(|(g1, g2): (LocalScopeGuard<Option<fn()>>, LocalScopeGuard<Option<fn()>>)| {
        let dma = dma.start(&mut data, g1);
        let (dma, data, g1) = dma.stop();
        println!("{}", data[0]);
        // let g2 = StaticScopeGuard::new(); // We shouldn't be able to return this as if it was the original g2.
        ((g1, g2), dma)
    });

    static mut DATA: [u8; 3] = [1, 2, 3];
    let dma = dma.start(unsafe { &mut DATA }, StaticScopeGuard::new());
    let dma = dma.stop();
}

// fn usage1() {
//     let dma = Dma { state: Off {} };
//     let mut data = [1u8, 2, 3];
//     let dma = scope!(|guard| {
//         let dma = dma.start(&mut data, guard);
//         let (dma, data) = dma.stop();
//         println!("{}", data[0]);
//         dma
//     });
// }

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
