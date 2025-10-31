use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor::{self, ThreadPool},
    try_join, FutureExt,
};
use rumpsteak_aura::{
    channel::{Bidirectional, Nil},
    session, try_session, End, Message, Receive, Role, Roles, Send,
};
use std::{error::Error, future::Future, marker, result};

type Result<T> = result::Result<T, Box<dyn Error + marker::Send + Sync>>;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
struct Roles(S, K, T);

#[derive(Role)]
#[message(Label)]
struct S(#[route(K)] Channel, #[route(T)] Nil);

#[derive(Role)]
#[message(Label)]
struct K(#[route(S)] Channel, #[route(T)] Channel);

#[derive(Role)]
#[message(Label)]
struct T(#[route(S)] Nil, #[route(K)] Channel);

#[derive(Message)]
enum Label {
    Ready(Ready),
    Copy(Copy),
}

struct Ready;
struct Copy(i32);

#[session]
type Source = Receive<K, Ready, Send<K, Copy, Receive<K, Ready, Send<K, Copy, End>>>>;

#[session]
#[rustfmt::skip]
type Kernel = Send<S, Ready, Receive<S, Copy, Receive<T, Ready, Send<T, Copy, Send<S, Ready, Receive<S, Copy, Receive<T, Ready, Send<T, Copy, End>>>>>>>>;

#[session]
type Sink = Send<K, Ready, Receive<K, Copy, Send<K, Ready, Receive<K, Copy, End>>>>;

async fn source(role: &mut S, input: (i32, i32)) -> Result<()> {
    try_session(role, |s: Source<'_, _>| async {
        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(input.0)).await?;

        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(input.1)).await?;

        Ok(((), s))
    })
    .await
}

async fn kernel(role: &mut K) -> Result<()> {
    try_session(role, |s: Kernel<'_, _>| async {
        let s = s.send(Ready).await?;
        let (Copy(x), s) = s.receive().await?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(x)).await?;

        let s = s.send(Ready).await?;
        let (Copy(y), s) = s.receive().await?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(y)).await?;

        Ok(((), s))
    })
    .await
}

async fn sink(role: &mut T) -> Result<(i32, i32)> {
    try_session(role, |s: Sink<'_, _>| async {
        let s = s.send(Ready).await?;
        let (Copy(x), s) = s.receive().await?;

        let s = s.send(Ready).await?;
        let (Copy(y), s) = s.receive().await?;

        Ok(((x, y), s))
    })
    .await
}

async fn spawn<F: Future + marker::Send + 'static>(pool: &ThreadPool, future: F) -> F::Output
where
    F::Output: marker::Send,
{
    let (future, handle) = future.remote_handle();
    pool.spawn_ok(future);
    handle.await
}

fn main() {
    let Roles(mut s, mut k, mut t) = Roles::default();
    let pool = ThreadPool::new().unwrap();

    let input = (1, 2);
    println!("input = {:?}", input);

    let (_, _, output) = executor::block_on(async {
        try_join!(
            spawn(&pool, async move { source(&mut s, input).await }),
            spawn(&pool, async move { kernel(&mut k).await }),
            spawn(&pool, async move { sink(&mut t).await }),
        )
        .unwrap()
    });

    println!("output = {:?}", output);
    assert_eq!(input, output);
}
