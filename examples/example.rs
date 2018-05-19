#[macro_use(scope)]
extern crate strong_scope_guard;

use strong_scope_guard::GuardHandle;

struct Dma<S> {
    state: S,
}

struct Off {}

struct Running<'guard, 'data: 'guard> {
    data: &'data mut [u8],
    guard: GuardHandle<'guard, 'data, fn()>,
}

impl Dma<Off> {
    pub fn start<'guard, 'data>(
        self,
        data: &'data mut [u8],
        mut guard: GuardHandle<'guard, 'data, fn()>,
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
        let Running { data, mut guard } = self.state;
        // stop DMA
        // Clear guard.
        guard.assign(None);
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
