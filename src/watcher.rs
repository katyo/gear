use futures::Stream;
use gear::{
    system::{Path, PathBuf},
    Error, Map, Result, Time,
};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher as FsWatcher};
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

#[derive(Default)]
struct State {
    done: bool,
    error: Option<Error>,
    paths: Map<PathBuf, Time>,
    waker: Option<Waker>,
}

pub struct Watcher(RecommendedWatcher);

#[derive(Default, Clone)]
pub struct Events(Arc<Mutex<State>>);

impl Events {
    fn handle(&self, result: notify::Result<Event>) {
        let mut state = self.0.lock().unwrap();
        match result {
            Ok(event) => {
                let time = Time::now();
                for path in event.paths {
                    state
                        .paths
                        .entry(path.into())
                        .and_modify(|value| {
                            *value = time;
                        })
                        .or_insert(time);
                }
            }
            Err(error) => {
                state.error = Some(error.into());
                state.done = true;
            }
        }
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }
}

impl Stream for Events {
    type Item = Result<Vec<(PathBuf, Time)>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut state = self.0.lock().unwrap();

        state.waker = Some(cx.waker().clone());

        if !state.paths.is_empty() {
            Poll::Ready(Some(Ok(state.paths.drain(..).collect())))
        } else if let Some(error) = state.error.take() {
            Poll::Ready(Some(Err(error)))
        } else if state.done {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

impl Watcher {
    pub fn new() -> Result<(Self, Events)> {
        let events = Events::default();

        let watcher = RecommendedWatcher::new_immediate({
            let handler = events.clone();
            move |result| handler.handle(result)
        })?;

        Ok((Self(watcher), events))
    }

    pub fn watch(&mut self, path: impl AsRef<Path>, recursive: bool) -> Result<()> {
        self.0.watch(
            path.as_ref(),
            if recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            },
        )?;
        Ok(())
    }

    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.0.unwatch(path.as_ref())?;
        Ok(())
    }
}
