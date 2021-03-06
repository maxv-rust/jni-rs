#![cfg(feature = "invocation")]

use std::{
    sync::{Arc, Barrier},
    thread::spawn,
};

use jni::{
    objects::{AutoLocal, JValue},
    sys::jint,
};

mod util;
use util::{attach_current_thread, unwrap};

#[test]
pub fn global_ref_works_in_other_threads() {
    const ITERS_PER_THREAD: usize = 10_000;

    let env = attach_current_thread();
    let mut join_handlers = Vec::new();

    let atomic_integer = {
        let local_ref = AutoLocal::new(
            &env,
            unwrap(
                &env,
                env.new_object(
                    "java/util/concurrent/atomic/AtomicInteger",
                    "(I)V",
                    &[JValue::from(0)],
                ),
            ),
        );
        unwrap(&env, env.new_global_ref(&local_ref))
    };

    let weak_atomic_integer = unwrap(&env, env.new_weak_global_ref(&atomic_integer));

    // Test with a different number of threads (from 2 to 8)
    for thread_num in 2..9 {
        let barrier = Arc::new(Barrier::new(thread_num));

        for _ in 0..thread_num {
            let barrier = barrier.clone();
            let weak_atomic_integer = weak_atomic_integer.clone();

            let jh = spawn(move || {
                let env = attach_current_thread();
                barrier.wait();
                for _ in 0..ITERS_PER_THREAD {
                    let atomic_integer =
                        unwrap(&env, env.upgrade_weak_global_ref(&weak_atomic_integer));
                    unwrap(
                        &env,
                        unwrap(
                            &env,
                            env.call_method(&atomic_integer, "incrementAndGet", "()I", &[]),
                        )
                        .i(),
                    );
                }
            });
            join_handlers.push(jh);
        }

        for jh in join_handlers.drain(..) {
            jh.join().unwrap();
        }

        let expected = (ITERS_PER_THREAD * thread_num) as jint;
        assert_eq!(
            expected,
            unwrap(
                &env,
                unwrap(
                    &env,
                    env.call_method(&atomic_integer, "getAndSet", "(I)I", &[JValue::from(0)])
                )
                .i()
            )
        );
    }
}
